use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(target_os = "windows")]
use windows::{
    Win32::UI::WindowsAndMessaging::*,
    Win32::UI::Accessibility::*,
    Win32::Foundation::*,
};

use tauri::Emitter;

/// Shared state between the capture thread and the main app
pub struct DragCaptureState {
    pub active: AtomicBool,
    /// Source HWND being dragged
    pub dragged_hwnd: Mutex<Option<isize>>,
    /// Known container HWNDs to check against
    pub containers: Mutex<Vec<isize>>,
    /// Current hovered container (for highlighting)
    pub hovered_container: Mutex<Option<isize>>,
    /// Last dragged window info (for auto-embed on drop)
    pub last_drag_source: Mutex<Option<(isize, String)>>,
}

impl DragCaptureState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            active: AtomicBool::new(false),
            dragged_hwnd: Mutex::new(None),
            containers: Mutex::new(Vec::new()),
            hovered_container: Mutex::new(None),
            last_drag_source: Mutex::new(None),
        })
    }

    pub fn register_container(&self, hwnd: isize) {
        if let Ok(mut c) = self.containers.lock() {
            if !c.contains(&hwnd) { c.push(hwnd); }
        }
    }

    pub fn unregister_container(&self, hwnd: isize) {
        if let Ok(mut c) = self.containers.lock() {
            c.retain(|&h| h != hwnd);
        }
    }

    /// Check for auto-embed: if a drag just ended over a container, return (source_hwnd, container_hwnd)
    pub fn take_auto_embed(&self) -> Option<(isize, isize)> {
        let hover = self.hovered_container.lock().ok()?;
        let hover_hwnd = (*hover)?;
        let source = self.last_drag_source.lock().ok()?;
        source.as_ref()?;
        let src_hwnd = source.as_ref().unwrap().0;
        Some((src_hwnd, hover_hwnd))
    }
}

/// Start drag capture: installs WinEvent hooks and runs message loop
#[cfg(target_os = "windows")]
pub fn start_drag_capture(
    state: Arc<DragCaptureState>,
    app_handle: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    if state.active.load(Ordering::SeqCst) {
        return Err("Drag capture already active".into());
    }
    state.active.store(true, Ordering::SeqCst);

    let active_ref = Arc::clone(&state);
    let app = app_handle.clone();

    thread::spawn(move || unsafe {
        // Install hooks for window move/size start and end
        let hook_start = SetWinEventHook(
            EVENT_SYSTEM_MOVESIZESTART,
            EVENT_SYSTEM_MOVESIZESTART,
            None,
            Some(win_event_callback),
            0, 0,
            WINEVENT_OUTOFCONTEXT,
        );
        let hook_end = SetWinEventHook(
            EVENT_SYSTEM_MOVESIZEEND,
            EVENT_SYSTEM_MOVESIZEEND,
            None,
            Some(win_event_callback),
            0, 0,
            WINEVENT_OUTOFCONTEXT,
        );

        let mut msg = MSG::default();
        // Polling for cursor position during drag
        let mut last_hover: Option<isize> = None;

        while active_ref.active.load(Ordering::SeqCst) {
            let has_msg = PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE);

            if has_msg.as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // If a window is being dragged, poll cursor to check hover
            if let Ok(dragged) = active_ref.dragged_hwnd.lock() {
                if dragged.is_some() {
                    let mut cursor = POINT::default();
                    if GetCursorPos(&mut cursor).is_ok() {
                        let hwnd_under = WindowFromPoint(cursor);
                        let root = GetAncestor(hwnd_under, GA_ROOT);
                        let check_hwnd = if root.0.is_null() { hwnd_under.0 } else { root.0 };

                        let containers = active_ref.containers.lock().unwrap();
                        let new_hover = containers.iter().find(|&&h| h == check_hwnd as isize).copied();

                        if new_hover != last_hover {
                            // Emit events for highlight
                            if let Some(prev) = last_hover {
                                let _ = app.emit("panel:drag-leave", prev);
                            }
                            if let Some(curr) = new_hover {
                                let _ = app.emit("panel:drag-enter", curr);
                            }
                            last_hover = new_hover;
                            if let Ok(mut h) = active_ref.hovered_container.lock() {
                                *h = new_hover;
                            }
                        }
                    }
                }
            }

            if !has_msg.as_bool() {
                thread::sleep(std::time::Duration::from_millis(30));
            }
        }

        // Cleanup hooks
        if !hook_start.0.is_null() { let _ = UnhookWinEvent(hook_start); }
        if !hook_end.0.is_null() { let _ = UnhookWinEvent(hook_end); }
    });

    log::info!("Drag capture started");
    Ok(())
}

/// WinEvent callback — runs in the hook thread
#[cfg(target_os = "windows")]
unsafe extern "system" fn win_event_callback(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if id_object != OBJID_WINDOW.0 { return; }

    let root = GetAncestor(hwnd, GA_ROOT);
    let target = if root.0 != std::ptr::null_mut() { root } else { hwnd };
    let target_hwnd = target.0 as isize;

    // Get window title
    let mut title_buf = [0u16; 256];
    let title_len = GetWindowTextW(target, &mut title_buf);
    let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

    DRAG_STATE.with(|s| {
        let state = match s.borrow().as_ref() { Some(x) => x.clone(), None => return };

        let title_clone = title.clone();
        if event == EVENT_SYSTEM_MOVESIZESTART {
            if let Ok(mut d) = state.dragged_hwnd.lock() {
                *d = Some(target_hwnd);
            }
            if let Ok(mut src) = state.last_drag_source.lock() {
                *src = Some((target_hwnd, title));
            }
            // Emit drag-start to frontend
            APP_HANDLE.with(|a| {
                if let Some(app) = a.borrow().as_ref() {
                    let _ = app.emit("drag:started", serde_json::json!({
                        "hwnd": target_hwnd, "title": title_clone
                    }));
                }
            });
        } else if event == EVENT_SYSTEM_MOVESIZEEND {
            if let Ok(mut d) = state.dragged_hwnd.lock() {
                *d = None;
            }
            let hover = state.hovered_container.lock().ok().and_then(|h| *h);
            let title2 = title_clone.clone();
            APP_HANDLE.with(|a| {
                if let Some(app) = a.borrow().as_ref() {
                    let _ = app.emit("drag:ended", serde_json::json!({
                        "hwnd": target_hwnd,
                        "hoveredContainer": hover,
                        "title": title2
                    }));
                }
            });
            // Reset hover after drag ends
            if let Ok(mut h) = state.hovered_container.lock() { *h = None; }
        }
    });
}

// Thread-local storage for the callback
std::thread_local! {
    static DRAG_STATE: std::cell::RefCell<Option<Arc<DragCaptureState>>> = std::cell::RefCell::new(None);
    static APP_HANDLE: std::cell::RefCell<Option<tauri::AppHandle>> = std::cell::RefCell::new(None);
}

/// Set the global drag state for the current thread (called after spawning the hook thread)
pub fn set_thread_drag_state(state: Option<Arc<DragCaptureState>>) {
    DRAG_STATE.with(|s| {
        *s.borrow_mut() = state;
    });
}

/// Set the app handle for use in the hook callback
pub fn set_thread_app_handle(app_handle: Option<tauri::AppHandle>) {
    APP_HANDLE.with(|a| {
        *a.borrow_mut() = app_handle;
    });
}

/// Stop drag capture
pub fn stop_drag_capture(state: &DragCaptureState) {
    state.active.store(false, Ordering::SeqCst);
    log::info!("Drag capture stopped");
}

// Non-Windows stubs
#[cfg(not(target_os = "windows"))]
pub fn start_drag_capture(
    _state: Arc<DragCaptureState>,
    _app_handle: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("Drag capture only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn stop_drag_capture(_state: &DragCaptureState) {}

#[cfg(not(target_os = "windows"))]
pub fn set_thread_drag_state(_state: Option<Arc<DragCaptureState>>) {}

#[cfg(not(target_os = "windows"))]
pub fn set_thread_app_handle(_app_handle: Option<tauri::AppHandle>) {}

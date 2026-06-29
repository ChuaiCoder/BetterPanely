use serde::Serialize;
use std::{
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::ClientToScreen,
    UI::{
        Accessibility::{SetWinEventHook, HWINEVENTHOOK},
        WindowsAndMessaging::{GetClientRect, GetCursorPos, IsWindow},
    },
};

const EVENT_SYSTEM_MOVESIZESTART_ID: u32 = 0x000A;
const EVENT_SYSTEM_MOVESIZEEND_ID: u32 = 0x000B;
const OBJID_WINDOW_ID: i32 = 0;
const CHILDID_SELF_ID: i32 = 0;
const WINEVENT_OUTOFCONTEXT_FLAG: u32 = 0x0000;
const WINEVENT_SKIPOWNPROCESS_FLAG: u32 = 0x0002;
const DRAG_POLL_INTERVAL_MS: u64 = 50;

static DRAG_MONITOR: OnceLock<DragCaptureMonitor> = OnceLock::new();

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DragEnteredWorkbenchPayload {
    pub source_hwnd: isize,
    pub title: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DragPositionPayload {
    pub source_hwnd: isize,
    pub x: i32,
    pub y: i32,
}

struct DragCaptureMonitor {
    app: AppHandle,
    state: Arc<Mutex<DragCaptureState>>,
    _hook: isize,
}

#[derive(Default)]
struct DragCaptureState {
    active_source_hwnd: Option<isize>,
    emitted_for_active_source: bool,
}

struct WorkbenchClientRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

pub fn install_drag_capture_monitor(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    if DRAG_MONITOR.get().is_some() {
        return Ok(());
    }

    let hook = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_MOVESIZESTART_ID,
            EVENT_SYSTEM_MOVESIZEEND_ID,
            None,
            Some(move_size_callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT_FLAG | WINEVENT_SKIPOWNPROCESS_FLAG,
        )
    };

    if hook.0.is_null() {
        return Err("Failed to install drag capture monitor".into());
    }

    let state = Arc::new(Mutex::new(DragCaptureState::default()));
    let monitor = DragCaptureMonitor {
        app: app.clone(),
        state: state.clone(),
        _hook: hook.0 as isize,
    };
    let _ = DRAG_MONITOR.set(monitor);

    thread::spawn(move || poll_dragged_window_entry(app, state));

    Ok(())
}

fn poll_dragged_window_entry(app: AppHandle, state: Arc<Mutex<DragCaptureState>>) {
    loop {
        thread::sleep(Duration::from_millis(DRAG_POLL_INTERVAL_MS));

        let Some((source_hwnd, already_emitted)) = active_source(&state) else {
            continue;
        };

        if !is_valid_window(source_hwnd) {
            clear_active_source(&state, source_hwnd);
            continue;
        }

        let Some((client_x, client_y)) = cursor_position_in_workbench(&app) else {
            continue;
        };

        if already_emitted {
            emit_drag_position(
                &app,
                "drag:moved-workbench",
                source_hwnd,
                client_x,
                client_y,
            );
            continue;
        }

        let Some(payload) = payload_for_source(source_hwnd, client_x, client_y) else {
            mark_active_source_emitted(&state, source_hwnd);
            continue;
        };

        if emit_drag_entered(&app, payload) {
            mark_active_source_emitted(&state, source_hwnd);
        }
    }
}

fn active_source(state: &Arc<Mutex<DragCaptureState>>) -> Option<(isize, bool)> {
    state.lock().ok().and_then(|state| {
        state
            .active_source_hwnd
            .map(|hwnd| (hwnd, state.emitted_for_active_source))
    })
}

fn mark_active_source_emitted(state: &Arc<Mutex<DragCaptureState>>, source_hwnd: isize) {
    if let Ok(mut state) = state.lock() {
        if state.active_source_hwnd == Some(source_hwnd) {
            state.emitted_for_active_source = true;
        }
    }
}

fn clear_active_source(state: &Arc<Mutex<DragCaptureState>>, source_hwnd: isize) {
    if let Ok(mut state) = state.lock() {
        if state.active_source_hwnd == Some(source_hwnd) {
            state.active_source_hwnd = None;
            state.emitted_for_active_source = false;
        }
    }
}

fn is_valid_window(hwnd: isize) -> bool {
    unsafe { IsWindow(HWND(hwnd as *mut _)).as_bool() }
}

fn cursor_position_in_workbench(app: &AppHandle) -> Option<(i32, i32)> {
    let rect = workbench_client_rect(app)?;
    let mut cursor = POINT::default();
    unsafe {
        GetCursorPos(&mut cursor).ok()?;
    }

    if cursor.x < rect.left
        || cursor.x > rect.right
        || cursor.y < rect.top
        || cursor.y > rect.bottom
    {
        return None;
    }

    Some((cursor.x - rect.left, cursor.y - rect.top))
}

fn workbench_client_rect(app: &AppHandle) -> Option<WorkbenchClientRect> {
    let window = app.get_webview_window(crate::WORKBENCH_WINDOW_LABEL)?;
    let hwnd = window.hwnd().ok()?;
    let hwnd = HWND(hwnd.0);
    let mut client = RECT::default();
    unsafe {
        GetClientRect(hwnd, &mut client).ok()?;

        let mut top_left = POINT {
            x: client.left,
            y: client.top,
        };
        let mut bottom_right = POINT {
            x: client.right,
            y: client.bottom,
        };
        if !ClientToScreen(hwnd, &mut top_left).as_bool()
            || !ClientToScreen(hwnd, &mut bottom_right).as_bool()
        {
            return None;
        }

        Some(WorkbenchClientRect {
            left: top_left.x,
            top: top_left.y,
            right: bottom_right.x,
            bottom: bottom_right.y,
        })
    }
}

fn payload_for_source(source_hwnd: isize, x: i32, y: i32) -> Option<DragEnteredWorkbenchPayload> {
    let window = crate::window_embedder::enumerator::enumerate_windows()
        .ok()?
        .into_iter()
        .find(|window| window.hwnd == source_hwnd)?;

    if !window.is_compatible {
        return None;
    }

    Some(DragEnteredWorkbenchPayload {
        source_hwnd,
        title: window.title,
        x,
        y,
    })
}

fn emit_drag_entered(app: &AppHandle, payload: DragEnteredWorkbenchPayload) -> bool {
    if let Some(window) = app.get_webview_window(crate::WORKBENCH_WINDOW_LABEL) {
        let _ = window.emit("drag:entered-workbench", payload);
        true
    } else {
        false
    }
}

fn emit_drag_position(
    app: &AppHandle,
    event_name: &str,
    source_hwnd: isize,
    x: i32,
    y: i32,
) -> bool {
    if let Some(window) = app.get_webview_window(crate::WORKBENCH_WINDOW_LABEL) {
        let payload = DragPositionPayload { source_hwnd, x, y };
        let _ = window.emit(event_name, payload);
        true
    } else {
        false
    }
}

unsafe extern "system" fn move_size_callback(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    object_id: i32,
    child_id: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if hwnd.0.is_null() || object_id != OBJID_WINDOW_ID || child_id != CHILDID_SELF_ID {
        return;
    }

    let Some(monitor) = DRAG_MONITOR.get() else {
        return;
    };

    let source_hwnd = hwnd.0 as isize;
    if event == EVENT_SYSTEM_MOVESIZESTART_ID {
        if let Ok(mut state) = monitor.state.lock() {
            state.active_source_hwnd = Some(source_hwnd);
            state.emitted_for_active_source = false;
        }
    } else if event == EVENT_SYSTEM_MOVESIZEEND_ID {
        if let Some((x, y)) = cursor_position_in_workbench(&monitor.app) {
            let should_emit = monitor
                .state
                .lock()
                .map(|state| {
                    state.active_source_hwnd == Some(source_hwnd) && state.emitted_for_active_source
                })
                .unwrap_or(false);
            if should_emit {
                emit_drag_position(&monitor.app, "drag:ended-workbench", source_hwnd, x, y);
            }
        }
        clear_active_source(&monitor.state, source_hwnd);
    }
}

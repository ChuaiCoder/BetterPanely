use crate::panel_manager::panel::{Panel, PanelType};
use crate::AppState;
use tauri::{command, AppHandle, Emitter, State};

/// Create a new panel (async — WebView creation is slow; container uses main-thread dispatch)
#[command]
pub async fn create_panel(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    title: String,
    panel_type: PanelType,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<Panel, String> {
    log::info!("[create_panel] title={}, type={:?}", title, panel_type);
    let mut manager = state.panel_manager.lock().map_err(|e| e.to_string())?;
    let panel = manager.create(title.clone(), panel_type.clone(), width, height).clone();
    log::info!("[create_panel] panel created in manager: id={}", panel.id);

    // Get current language
    let state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
    let lang = state_mgr.get_language();
    drop(state_mgr);

    // Create the actual window based on panel type
    let panel_mut = manager.get_mut(&panel.id).ok_or("Panel not found after creation")?;

    match &panel_mut.panel_type.clone() {
        PanelType::Tool { tool_id } => {
            let url = match tool_id.as_str() {
                "calculator" => "src/tools/calculator/index.html",
                "notes" => "src/tools/notes/index.html",
                "timer" => "src/tools/timer/index.html",
                "weather" => "src/tools/weather/index.html",
                _ => return Err(format!("Unknown tool: {}", tool_id)),
            };
            panel_mut.create_webview(&app_handle, url, &lang).map_err(|e| e.to_string())?;
        }
        PanelType::Embedded { .. } => {
            // Container window must be created on main thread (Win32 rule).
            // Dispatch via run_on_main_thread + channel to get the HWND back.
            #[cfg(target_os = "windows")]
            {
                let px = panel.x; let py = panel.y;
                let pw = panel.width; let ph = panel.height;
                let ptitle = panel.title.clone();
                let drag_state = state.drag_capture.clone();
                let pid = panel.id.clone();

                let (tx, rx) = std::sync::mpsc::channel::<Result<isize, String>>();
                app_handle.run_on_main_thread(move || {
                    let result = crate::panel_manager::container::create_container(
                        px, py, pw, ph, &ptitle,
                    ).map_err(|e| e.to_string());
                    let _ = tx.send(result);
                }).map_err(|e| e.to_string())?;

                let hwnd = rx.recv().map_err(|e| e.to_string())??;
                panel_mut.container_hwnd = Some(hwnd);
                if let Some(ref drag) = drag_state {
                    drag.register_container(hwnd, &pid);
                }
            }
        }
    }

    let updated = panel_mut.clone();
    drop(manager);

    // Emit event
    let _ = app_handle.emit("panel:created", &updated);

    Ok(updated)
}

/// Destroy a panel (sync — runs on main thread for Win32 safety)
#[command]
pub fn destroy_panel(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    panel_id: String,
) -> Result<(), String> {
    let mut manager = state.panel_manager.lock().map_err(|e| e.to_string())?;

    // Unregister container from drag capture before removing
    if let Some(panel) = manager.get(&panel_id) {
        if let Some(hwnd) = panel.container_hwnd {
            if let Some(ref drag) = state.drag_capture {
                drag.unregister_container(hwnd);
            }
        }
    }

    if let Some(mut panel) = manager.remove(&panel_id) {
        panel.cleanup(&app_handle).map_err(|e| e.to_string())?;
    }

    drop(manager);
    let _ = app_handle.emit("panel:destroyed", &panel_id);
    Ok(())
}

/// List all panels
#[command]
pub async fn list_panels(
    state: State<'_, AppState>,
) -> Result<Vec<Panel>, String> {
    let manager = state.panel_manager.lock().map_err(|e| e.to_string())?;
    Ok(manager.list().into_iter().cloned().collect())
}

/// Get a single panel by ID
#[command]
pub async fn get_panel(
    state: State<'_, AppState>,
    panel_id: String,
) -> Result<Panel, String> {
    let manager = state.panel_manager.lock().map_err(|e| e.to_string())?;
    manager.get(&panel_id).cloned().ok_or("Panel not found".into())
}

/// Move a panel (sync — Win32 SetWindowPos must run on main thread)
#[command]
pub fn move_panel(
    state: State<'_, AppState>,
    panel_id: String,
    x: f64,
    y: f64,
) -> Result<(), String> {
    let mut manager = state.panel_manager.lock().map_err(|e| e.to_string())?;
    let panel = manager.get_mut(&panel_id).ok_or("Panel not found")?;
    panel.x = x;
    panel.y = y;

    #[cfg(target_os = "windows")]
    if let Some(hwnd) = panel.container_hwnd {
        crate::panel_manager::container::update_container(hwnd, x, y, panel.width, panel.height)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Resize a panel
#[command]
pub fn resize_panel(
    state: State<'_, AppState>,
    panel_id: String,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let mut manager = state.panel_manager.lock().map_err(|e| e.to_string())?;
    let panel = manager.get_mut(&panel_id).ok_or("Panel not found")?;
    panel.width = width;
    panel.height = height;

    #[cfg(target_os = "windows")]
    if let Some(hwnd) = panel.container_hwnd {
        crate::panel_manager::container::update_container(hwnd, panel.x, panel.y, width, height)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Toggle always-on-top for a panel
#[command]
pub fn set_panel_always_on_top(
    state: State<'_, AppState>,
    panel_id: String,
    always_on_top: bool,
) -> Result<(), String> {
    let mut manager = state.panel_manager.lock().map_err(|e| e.to_string())?;
    let panel = manager.get_mut(&panel_id).ok_or("Panel not found")?;
    panel.always_on_top = always_on_top;

    #[cfg(target_os = "windows")]
    if let Some(hwnd) = panel.container_hwnd {
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::Win32::Foundation::HWND;
        unsafe {
            let insert_after = if always_on_top { HWND_TOPMOST } else { HWND_NOTOPMOST };
            let _ = SetWindowPos(
                HWND(hwnd as *mut std::ffi::c_void),
                insert_after,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    Ok(())
}

/// Set panel opacity
#[command]
pub fn set_panel_opacity(
    state: State<'_, AppState>,
    panel_id: String,
    opacity: f64,
) -> Result<(), String> {
    let mut manager = state.panel_manager.lock().map_err(|e| e.to_string())?;
    let panel = manager.get_mut(&panel_id).ok_or("Panel not found")?;
    let clamped = opacity.clamp(0.1, 1.0);
    panel.opacity = clamped;

    #[cfg(target_os = "windows")]
    if let Some(hwnd) = panel.container_hwnd {
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::Win32::Foundation::HWND;
        unsafe {
            let h = HWND(hwnd as *mut std::ffi::c_void);
            let mut ex_style = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
            if clamped >= 0.999 {
                ex_style &= !WS_EX_LAYERED.0;
            } else {
                ex_style |= WS_EX_LAYERED.0;
            }
            let _ = SetWindowLongPtrW(h, GWL_EXSTYLE, ex_style as isize);
            let alpha = (clamped * 255.0) as u8;
            let _ = SetLayeredWindowAttributes(h, windows::Win32::Foundation::COLORREF(0), alpha, LWA_ALPHA);
            let _ = SetWindowPos(h, HWND(std::ptr::null_mut()), 0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED);
        }
    }

    Ok(())
}

/// Toggle click-through for a panel
#[command]
pub fn set_panel_click_through(
    state: State<'_, AppState>,
    panel_id: String,
    click_through: bool,
) -> Result<(), String> {
    let mut manager = state.panel_manager.lock().map_err(|e| e.to_string())?;
    let panel = manager.get_mut(&panel_id).ok_or("Panel not found")?;
    panel.click_through = click_through;

    #[cfg(target_os = "windows")]
    if let Some(hwnd) = panel.container_hwnd {
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::Win32::Foundation::HWND;
        unsafe {
            let h = HWND(hwnd as *mut std::ffi::c_void);
            let mut ex_style = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
            if click_through {
                ex_style |= WS_EX_TRANSPARENT.0 | WS_EX_LAYERED.0;
            } else {
                ex_style &= !WS_EX_TRANSPARENT.0;
            }
            let _ = SetWindowLongPtrW(h, GWL_EXSTYLE, ex_style as isize);
            let _ = SetWindowPos(h, HWND(std::ptr::null_mut()), 0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED);
        }
    }

    Ok(())
}

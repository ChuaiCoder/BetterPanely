use crate::panel_manager::panel::PanelType;
use crate::window_embedder::enumerator::WindowInfo;
use crate::AppState;
use tauri::{command, AppHandle, Emitter, State};

/// Enumerate all visible windows
#[command]
pub async fn enumerate_windows() -> Result<Vec<WindowInfo>, String> {
    crate::window_embedder::enumerator::enumerate_windows().map_err(|e| e.to_string())
}

/// Refresh window list (alias for enumerate)
#[command]
pub async fn refresh_window_list() -> Result<Vec<WindowInfo>, String> {
    crate::window_embedder::enumerator::enumerate_windows().map_err(|e| e.to_string())
}

/// Embed a window into a panel
#[command]
pub async fn embed_window(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    panel_id: String,
    source_hwnd: isize,
) -> Result<(), String> {
    let mut manager = state.panel_manager.lock().map_err(|e| e.to_string())?;
    let panel = manager.get_mut(&panel_id).ok_or("Panel not found")?;

    // Get the container HWND
    let container_hwnd = panel.container_hwnd.ok_or("Panel has no container window")?;

    // Perform the embedding
    let embed_info = crate::window_embedder::embed_window(source_hwnd, container_hwnd)
        .map_err(|e| {
            let _ = app_handle.emit("panel:embed-error", e.to_string());
            e.to_string()
        })?;

    // Update panel type
    panel.panel_type = PanelType::Embedded {
        embed_info: Some(embed_info),
    };

    let panel_id_clone = panel_id.clone();
    drop(manager);

    let _ = app_handle.emit(
        "panel:embedded",
        serde_json::json!({ "panelId": panel_id_clone, "hwnd": source_hwnd }),
    );

    Ok(())
}

/// Release an embedded window from its panel
#[command]
pub async fn release_window(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    panel_id: String,
) -> Result<(), String> {
    let mut manager = state.panel_manager.lock().map_err(|e| e.to_string())?;
    let panel = manager.get_mut(&panel_id).ok_or("Panel not found")?;

    if let PanelType::Embedded {
        embed_info: Some(ref info),
    } = panel.panel_type
    {
        crate::window_embedder::release_window(info.source_hwnd, info).map_err(|e| {
            let _ = app_handle.emit("panel:embed-error", e.to_string());
            e.to_string()
        })?;

        let hwnd = info.source_hwnd;
        panel.panel_type = PanelType::Embedded { embed_info: None };

        let panel_id_clone = panel_id.clone();
        drop(manager);

        let _ = app_handle.emit(
            "panel:released",
            serde_json::json!({ "panelId": panel_id_clone, "hwnd": hwnd }),
        );

        Ok(())
    } else {
        Err("Panel has no embedded window".into())
    }
}

/// Start drag-to-panel capture mode
#[command]
pub async fn start_drag_capture(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let drag = state.drag_capture.as_ref().ok_or("Drag capture not initialized")?;
    if drag.active.load(std::sync::atomic::Ordering::SeqCst) {
        return Err("Drag capture is already active".into());
    }

    #[cfg(target_os = "windows")]
    {
        // Register all current container HWNDs
        let panel_mgr = state.panel_manager.lock().map_err(|e| e.to_string())?;
        for panel in panel_mgr.list() {
            if let Some(hwnd) = panel.container_hwnd {
                drag.register_container(hwnd);
            }
        }
        drop(panel_mgr);

        crate::drag_capture::hook::start_drag_capture(
            drag.clone(),
            app_handle.clone(),
        ).map_err(|e| e.to_string())?;

        // Set thread-local state for the hook callback
        crate::drag_capture::hook::set_thread_drag_state(Some(drag.clone()));
        crate::drag_capture::hook::set_thread_app_handle(Some(app_handle.clone()));
    }

    log::info!("Drag capture mode started");
    Ok(())
}

/// Stop drag capture mode
#[command]
pub async fn stop_drag_capture(
    state: State<'_, AppState>,
) -> Result<(), String> {
    if let Some(drag) = &state.drag_capture {
        crate::drag_capture::hook::stop_drag_capture(drag);
        crate::drag_capture::hook::set_thread_drag_state(None);
    }
    log::info!("Drag capture mode stopped");
    Ok(())
}

/// Hotkey-based window capture (Ctrl+Shift+W)
#[command]
pub async fn capture_window_via_hotkey(
    _app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    capture_window_via_hotkey_internal(&state).map_err(|e| e.to_string())
}

/// Internal hotkey capture implementation
pub fn capture_window_via_hotkey_internal(
    state: &AppState,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if let Some((hwnd, title)) = crate::drag_capture::hotkey::get_window_under_cursor() {
            log::info!("Hotkey capture: window '{}' (hwnd={})", title, hwnd);

            // Check compatibility
            let windows = crate::window_embedder::enumerator::enumerate_windows()
                .map_err(|e| e.to_string())?;
            let win_info = windows.iter().find(|w| w.hwnd == hwnd);

            if let Some(info) = win_info {
                if !info.is_compatible {
                    return Err(format!(
                        "Window is not compatible: {}",
                        info.incompatibility_reason.as_deref().unwrap_or("Unknown reason")
                    ));
                }
            }

            // Create a new embedded panel and embed the window
            let mut manager = state.panel_manager.lock().map_err(|e| e.to_string())?;

            let panel = manager.create(
                title,
                PanelType::Embedded { embed_info: None },
                None,
                None,
            ).clone();

            let panel_id = panel.id.clone();

            // Create native container
            #[cfg(target_os = "windows")]
            {
                let container_hwnd = crate::panel_manager::container::create_container(
                    panel.x, panel.y, panel.width, panel.height, &panel.title,
                )
                .map_err(|e| e.to_string())?;

                if let Some(p) = manager.get_mut(&panel_id) {
                    p.container_hwnd = Some(container_hwnd);
                }

                // Embed the window
                let embed_info = crate::window_embedder::embed_window(hwnd, container_hwnd)
                    .map_err(|e| e.to_string())?;

                if let Some(p) = manager.get_mut(&panel_id) {
                    p.panel_type = PanelType::Embedded {
                        embed_info: Some(embed_info),
                    };
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = state;
        return Err("Window capture is only supported on Windows".into());
    }

    Ok(())
}

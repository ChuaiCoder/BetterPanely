mod panel_manager;
mod window_embedder;
mod drag_capture;
mod commands;
mod builtin_tools;
mod locales;
mod state;
mod tray;

use panel_manager::PanelManager;
use state::AppStateManager;
use drag_capture::hook::DragCaptureState;
use std::sync::{Arc, Mutex};
use tauri::Manager;

/// Application state shared across all handlers
pub struct AppState {
    pub panel_manager: Mutex<PanelManager>,
    pub state_manager: Mutex<AppStateManager>,
    pub drag_capture: Option<Arc<DragCaptureState>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let drag_state = DragCaptureState::new();
    let app_state = AppState {
        panel_manager: Mutex::new(PanelManager::new()),
        state_manager: Mutex::new(AppStateManager::new()),
        drag_capture: Some(drag_state.clone()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(app_state)
        .setup(|app| {
            log::info!("BetterPanely starting up...");

            // Load saved state (including language setting)
            let state = app.state::<AppState>();
            let mut state_mgr = state.state_manager.lock().unwrap();
            if let Err(e) = state_mgr.load(app.handle()) {
                log::warn!("Failed to load saved state: {}", e);
            }
            let current_lang = state_mgr.get_language();
            let saved_panels = state_mgr.take_loaded_panels();
            drop(state_mgr);

            // Restore tool panels from saved state
            {
                let mut panel_mgr = state.panel_manager.lock().unwrap();
                for saved in &saved_panels {
                    if let Some(ref tool_id) = saved.tool_id {
                        let panel_type = panel_manager::panel::PanelType::Tool {
                            tool_id: tool_id.clone(),
                        };
                        let panel = panel_mgr.create(
                            saved.title.clone(),
                            panel_type.clone(),
                            Some(saved.width),
                            Some(saved.height),
                        ).clone();
                        if let Some(p) = panel_mgr.get_mut(&panel.id) {
                            p.x = saved.x;
                            p.y = saved.y;
                            p.always_on_top = saved.always_on_top;
                            p.opacity = saved.opacity;
                        }

                        // Create the WebView immediately
                        let url = match tool_id.as_str() {
                            "calculator" => "src/tools/calculator/index.html",
                            "notes" => "src/tools/notes/index.html",
                            "timer" => "src/tools/timer/index.html",
                            "weather" => "src/tools/weather/index.html",
                            _ => continue,
                        };
                        if let Some(p) = panel_mgr.get_mut(&panel.id) {
                            if let Err(e) = p.create_webview(app.handle(), url, &current_lang) {
                                log::warn!("Failed to restore panel {}: {}", panel.id, e);
                            }
                        }
                    }
                }
                log::info!("Restored {} panels from saved state", saved_panels.len());
            }

            // Initialize system tray with loaded language
            tray::create_tray(app.handle(), &current_lang)?;

            // Register global hotkey for window capture (Ctrl+Shift+W)
            #[cfg(target_os = "windows")]
            {
                use tauri_plugin_global_shortcut::GlobalShortcutExt;
                let app_handle = app.handle().clone();
                let _ = app.global_shortcut().on_shortcut("Ctrl+Shift+W", move |_app, _shortcut, _event| {
                    let state = app_handle.state::<AppState>();
                    let _ = commands::embed_cmds::capture_window_via_hotkey_internal(&state);
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::panel_cmds::create_panel,
            commands::panel_cmds::destroy_panel,
            commands::panel_cmds::list_panels,
            commands::panel_cmds::get_panel,
            commands::panel_cmds::move_panel,
            commands::panel_cmds::resize_panel,
            commands::panel_cmds::set_panel_always_on_top,
            commands::panel_cmds::set_panel_opacity,
            commands::panel_cmds::set_panel_click_through,
            commands::embed_cmds::enumerate_windows,
            commands::embed_cmds::refresh_window_list,
            commands::embed_cmds::embed_window,
            commands::embed_cmds::release_window,
            commands::embed_cmds::start_drag_capture,
            commands::embed_cmds::stop_drag_capture,
            commands::embed_cmds::capture_window_via_hotkey,
            commands::tool_cmds::launch_tool,
            commands::tool_cmds::list_tools,
            commands::tool_cmds::open_settings,
            commands::tool_cmds::save_state,
            commands::tool_cmds::load_state,
            commands::settings_cmds::get_settings,
            commands::settings_cmds::get_language,
            commands::settings_cmds::set_language,
        ])
        .run(tauri::generate_context!())
        .expect("error while running BetterPanely");
}

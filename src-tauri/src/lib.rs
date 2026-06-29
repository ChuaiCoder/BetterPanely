mod window_embedder;
mod drag_capture;
mod commands;
mod locales;
mod state;
mod tray;
mod thumbnail;

use state::AppStateManager;
use thumbnail::SharedThumbnailManager;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

/// Application state shared across all handlers
pub struct AppState {
    pub state_manager: Mutex<AppStateManager>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let thumbnail_manager = SharedThumbnailManager::new();
    let app_state = AppState {
        state_manager: Mutex::new(AppStateManager::new()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(app_state)
        .manage(thumbnail_manager)
        .setup(|app| {
            log::info!("BetterPanely starting up...");

            // Load saved state (including language setting)
            let state = app.state::<AppState>();
            let mut state_mgr = state.state_manager.lock().unwrap();
            if let Err(e) = state_mgr.load(app.handle()) {
                log::warn!("Failed to load saved state: {}", e);
            }
            let current_lang = state_mgr.get_language();
            drop(state_mgr);

            // Initialize system tray with loaded language
            tray::create_tray(app.handle(), &current_lang)?;

            // Register global hotkey for window capture (Ctrl+Shift+W)
            // Emits event to main window; frontend calls the sync capture command on main thread
            #[cfg(target_os = "windows")]
            {
                use tauri_plugin_global_shortcut::GlobalShortcutExt;
                let app_handle = app.handle().clone();
                match app.global_shortcut().on_shortcut("Ctrl+Shift+W", move |_app, _shortcut, _event| {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.emit("tray:capture-hotkey", ());
                    }
                }) {
                    Ok(_) => log::info!("Global shortcut Ctrl+Shift+W registered"),
                    Err(e) => log::error!("Failed to register Ctrl+Shift+W: {}", e),
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings_cmds::get_settings,
            commands::settings_cmds::get_language,
            commands::settings_cmds::set_language,
            commands::settings_cmds::open_settings,
            commands::workbench_cmds::wb_enumerate_windows,
            commands::workbench_cmds::wb_capture_window_under_cursor,
            commands::workbench_cmds::wb_add_thumbnail,
            commands::workbench_cmds::wb_update_thumbnail_rect,
            commands::workbench_cmds::wb_remove_panel,
            commands::workbench_cmds::wb_focus_source,
            commands::workbench_cmds::wb_get_workbench_hwnd,
            commands::workbench_cmds::wb_save_layout,
            commands::workbench_cmds::wb_load_layout,
        ])
        .run(tauri::generate_context!())
        .expect("error while running BetterPanely");
}

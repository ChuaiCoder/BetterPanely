use crate::state::AppSettings;
use crate::AppState;
use tauri::{command, AppHandle, Emitter, Manager, State};

/// Get the full application settings
#[command]
pub async fn get_settings(
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    let state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
    Ok(state_mgr.get_settings().clone())
}

/// Get only the language setting
#[command]
pub async fn get_language(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
    Ok(state_mgr.get_language())
}

/// Set the language and persist
#[command]
pub async fn set_language(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    lang: String,
) -> Result<String, String> {
    // Step 1: Update language in state_manager
    let new_lang = {
        let mut state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
        let lang = state_mgr.set_language(&lang).map_err(|e| e.to_string())?;
        drop(state_mgr);
        lang
    };

    let state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
    state_mgr.save_settings().map_err(|e| e.to_string())?;

    let _ = app_handle.emit("language-changed", &new_lang);
    log::info!("Language changed to: {}", new_lang);
    Ok(new_lang)
}

/// Open the settings page as a standalone utility window.
#[command]
pub async fn open_settings(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
    let lang = state_mgr.get_language();
    drop(state_mgr);

    let url = format!("src/tools/settings/index.html#lang={}", lang);
    let label = "settings_window";

    if let Some(window) = app_handle.get_webview_window(label) {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    tauri::WebviewWindowBuilder::new(
        &app_handle,
        label,
        tauri::WebviewUrl::App(url.into()),
    )
    .title("Settings")
    .inner_size(420.0, 520.0)
    .center()
    .decorations(true)
    .resizable(false)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

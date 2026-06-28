use crate::state::AppSettings;
use crate::AppState;
use tauri::{command, AppHandle, Emitter, State};

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
    let mut state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
    let new_lang = state_mgr.set_language(&lang).map_err(|e| e.to_string())?;

    // Save state with new settings
    let panel_mgr = state.panel_manager.lock().map_err(|e| e.to_string())?;
    state_mgr.save(&app_handle, &panel_mgr).map_err(|e| e.to_string())?;
    drop(panel_mgr);
    drop(state_mgr);

    // Emit event so all windows react
    let _ = app_handle.emit("language-changed", &new_lang);

    log::info!("Language changed to: {}", new_lang);
    Ok(new_lang)
}

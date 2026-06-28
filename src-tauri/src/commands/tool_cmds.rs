use crate::builtin_tools::ToolDefinition;
use crate::panel_manager::panel::{Panel, PanelType};
use crate::AppState;
use tauri::{command, AppHandle, Emitter, Manager, State};

/// Launch a built-in tool as a new tool panel
#[command]
pub async fn launch_tool(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    tool_id: String,
    x: Option<f64>,
    y: Option<f64>,
) -> Result<Panel, String> {
    // Get current language
    let state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
    let lang = state_mgr.get_language();
    drop(state_mgr);

    // Verify the tool exists with localized names
    let tools = crate::builtin_tools::get_builtin_tools(&lang);
    let tool = tools
        .iter()
        .find(|t| t.id == tool_id)
        .ok_or_else(|| format!("Unknown tool: {}", tool_id))?;

    let mut manager = state.panel_manager.lock().map_err(|e| e.to_string())?;

    let panel_type = PanelType::Tool {
        tool_id: tool_id.clone(),
    };

    let mut panel = manager
        .create(tool.name.clone(), panel_type.clone(), None, None)
        .clone();

    // Override position if provided
    if let Some(px) = x {
        panel.x = px;
    }
    if let Some(py) = y {
        panel.y = py;
    }

    // Create the WebView with language
    let panel_mut = manager.get_mut(&panel.id).ok_or("Panel not found after creation")?;
    panel_mut.create_webview(&app_handle, &tool.url, &lang).map_err(|e| e.to_string())?;
    panel_mut.x = panel.x;
    panel_mut.y = panel.y;

    let updated = panel_mut.clone();
    drop(manager);

    let _ = app_handle.emit("panel:created", &updated);

    Ok(updated)
}

/// List all available built-in tools
#[command]
pub async fn list_tools(
    state: State<'_, AppState>,
) -> Result<Vec<ToolDefinition>, String> {
    let state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
    let lang = state_mgr.get_language();
    drop(state_mgr);
    Ok(crate::builtin_tools::get_builtin_tools(&lang))
}

/// Save current application state to disk
#[command]
pub async fn save_state(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
    let manager = state.panel_manager.lock().map_err(|e| e.to_string())?;
    state_mgr.save(&app_handle, &manager).map_err(|e| e.to_string())
}

/// Load application state from disk
#[command]
pub async fn load_state(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
    state_mgr.load(&app_handle).map_err(|e| e.to_string())
}

/// Open the settings page in a new WebView window
#[command]
pub async fn open_settings(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
    let lang = state_mgr.get_language();
    drop(state_mgr);

    let url = format!("src/tools/settings/index.html?lang={}", lang);
    let label = "settings_window";

    // Check if settings window already exists
    if app_handle.get_webview_window(label).is_some() {
        // Focus existing window
        if let Some(w) = app_handle.get_webview_window(label) {
            let _ = w.show();
            let _ = w.set_focus();
        }
        return Ok(());
    }

    let _webview = tauri::WebviewWindowBuilder::new(
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

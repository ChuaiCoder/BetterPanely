use crate::state::SavedPanel;
use crate::thumbnail::SharedThumbnailManager;
use crate::window_embedder::enumerator;
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

#[derive(Serialize, Deserialize)]
pub struct WindowInfo {
    pub hwnd: isize,
    pub title: String,
    pub exe_path: String,
    pub class_name: String,
    pub is_compatible: bool,
    pub incompatibility_reason: Option<String>,
    pub pid: u32,
    pub rect: RectInfo,
}

#[derive(Serialize, Deserialize)]
pub struct RectInfo {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddThumbnailResult {
    pub panel_id: String,
    pub source_width: i32,
    pub source_height: i32,
}

fn to_window_info(w: enumerator::WindowInfo) -> WindowInfo {
    WindowInfo {
        hwnd: w.hwnd as isize,
        title: w.title,
        exe_path: w.exe_path,
        class_name: w.class_name,
        is_compatible: w.is_compatible,
        incompatibility_reason: w.incompatibility_reason,
        pid: w.pid,
        rect: RectInfo {
            left: w.rect.left,
            top: w.rect.top,
            right: w.rect.right,
            bottom: w.rect.bottom,
        },
    }
}

#[cfg(target_os = "windows")]
fn get_workbench_hwnd(app: &AppHandle) -> Result<isize, String> {
    let window = app
        .get_webview_window(crate::WORKBENCH_WINDOW_LABEL)
        .ok_or_else(|| "Workbench window not found".to_string())?;
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    Ok(hwnd.0 as isize)
}

fn finite_i32_arg(name: &str, value: f64) -> Result<i32, String> {
    if !value.is_finite() {
        return Err(format!("Thumbnail {} must be finite", name));
    }

    let rounded = value.round();
    if rounded < i32::MIN as f64 || rounded > i32::MAX as f64 {
        return Err(format!("Thumbnail {} is out of range", name));
    }

    Ok(rounded as i32)
}

fn positive_i32_arg(name: &str, value: f64) -> Result<i32, String> {
    let value = finite_i32_arg(name, value)?;
    if value <= 0 {
        return Err(format!("Thumbnail {} must be positive", name));
    }
    Ok(value)
}

struct ToolWindowConfig {
    title_key: &'static str,
    url: &'static str,
    width: f64,
    height: f64,
}

fn tool_window_config(tool_id: &str) -> Option<ToolWindowConfig> {
    match tool_id {
        "calculator" => Some(ToolWindowConfig {
            title_key: "tool.calculator",
            url: "src/tools/calculator/index.html",
            width: 320.0,
            height: 480.0,
        }),
        "notes" => Some(ToolWindowConfig {
            title_key: "tool.notes",
            url: "src/tools/notes/index.html",
            width: 480.0,
            height: 520.0,
        }),
        "timer" => Some(ToolWindowConfig {
            title_key: "tool.timer",
            url: "src/tools/timer/index.html",
            width: 360.0,
            height: 260.0,
        }),
        "weather" => Some(ToolWindowConfig {
            title_key: "tool.weather",
            url: "src/tools/weather/index.html",
            width: 380.0,
            height: 460.0,
        }),
        _ => None,
    }
}

pub fn refresh_tool_window_titles(app: &AppHandle, lang: &str) {
    for tool_id in ["calculator", "notes", "timer", "weather"] {
        let Some(config) = tool_window_config(tool_id) else {
            continue;
        };
        let label = format!("tool_{}_window", tool_id);
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.set_title(crate::locales::t(config.title_key, lang));
        }
    }
}

#[tauri::command]
pub fn wb_enumerate_windows() -> Result<Vec<WindowInfo>, String> {
    #[cfg(target_os = "windows")]
    {
        let windows = enumerator::enumerate_windows().map_err(|e| e.to_string())?;
        Ok(windows.into_iter().map(to_window_info).collect())
    }
    #[cfg(not(target_os = "windows"))]
    Ok(vec![])
}

#[tauri::command]
pub fn wb_capture_focused_window(app: AppHandle) -> Result<Option<WindowInfo>, String> {
    #[cfg(target_os = "windows")]
    {
        let Some((source_hwnd, _title)) = crate::drag_capture::hotkey::get_focused_window() else {
            return Ok(None);
        };
        let workbench_hwnd = get_workbench_hwnd(&app)?;
        if source_hwnd == workbench_hwnd {
            return Ok(None);
        }

        let windows = enumerator::enumerate_windows().map_err(|e| e.to_string())?;
        let Some(info) = windows.into_iter().find(|w| w.hwnd == source_hwnd) else {
            return Err("Focused window is not eligible for capture".to_string());
        };

        if !info.is_compatible {
            return Err(format!(
                "Focused window is not compatible: {}",
                info.incompatibility_reason
                    .as_deref()
                    .unwrap_or("Unknown reason")
            ));
        }

        Ok(Some(to_window_info(info)))
    }
    #[cfg(not(target_os = "windows"))]
    Ok(None)
}

#[tauri::command]
pub fn wb_add_thumbnail(
    source_hwnd: isize,
    app: AppHandle,
    thumbnail_manager: State<'_, SharedThumbnailManager>,
) -> Result<AddThumbnailResult, String> {
    #[cfg(target_os = "windows")]
    let dest_hwnd = get_workbench_hwnd(&app)?;
    #[cfg(not(target_os = "windows"))]
    let dest_hwnd = 0;

    if source_hwnd == 0 {
        return Err("Source HWND is invalid".to_string());
    }
    if source_hwnd == dest_hwnd {
        return Err("Cannot capture the workbench window itself".to_string());
    }

    let panel_id = thumbnail_manager
        .next_panel_id()
        .map_err(|e| e.to_string())?;
    let source_size = thumbnail_manager
        .register(dest_hwnd, source_hwnd, &panel_id)
        .map_err(|e| e.to_string())?;
    Ok(AddThumbnailResult {
        panel_id,
        source_width: source_size.width,
        source_height: source_size.height,
    })
}

#[tauri::command]
pub fn wb_update_thumbnail_rect(
    panel_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    thumbnail_manager: State<'_, SharedThumbnailManager>,
) -> Result<(), String> {
    let x = finite_i32_arg("x", x)?;
    let y = finite_i32_arg("y", y)?;
    let width = positive_i32_arg("width", width)?;
    let height = positive_i32_arg("height", height)?;

    thumbnail_manager
        .update_rect(&panel_id, x, y, width, height)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn wb_sync_thumbnail_stack(
    panel_ids: Vec<String>,
    thumbnail_manager: State<'_, SharedThumbnailManager>,
) -> Result<(), String> {
    thumbnail_manager
        .sync_stack_order(panel_ids)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn wb_remove_panel(
    panel_id: String,
    thumbnail_manager: State<'_, SharedThumbnailManager>,
) -> Result<(), String> {
    thumbnail_manager
        .unregister(&panel_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn wb_focus_source(source_hwnd: isize) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
        unsafe {
            let hwnd = windows::Win32::Foundation::HWND(source_hwnd as *mut _);
            let result = SetForegroundWindow(hwnd);
            if !result.as_bool() {
                return Err("Failed to set foreground window".to_string());
            }
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    Ok(())
}

#[tauri::command]
pub fn wb_get_workbench_hwnd(app: AppHandle) -> Result<isize, String> {
    #[cfg(target_os = "windows")]
    return get_workbench_hwnd(&app);

    #[cfg(not(target_os = "windows"))]
    Err("Workbench HWND is only available on Windows".to_string())
}

#[tauri::command]
pub fn wb_open_tool_window(
    tool_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let config =
        tool_window_config(&tool_id).ok_or_else(|| format!("Unknown tool: {}", tool_id))?;
    let lang = state
        .state_manager
        .lock()
        .map_err(|e| e.to_string())?
        .get_language();

    let label = format!("tool_{}_window", tool_id);
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.set_title(crate::locales::t(config.title_key, &lang));
        let _ = window.set_skip_taskbar(true);
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let url = format!("{}#lang={}", config.url, lang);
    WebviewWindowBuilder::new(&app, label, WebviewUrl::App(url.into()))
        .title(crate::locales::t(config.title_key, &lang))
        .inner_size(config.width, config.height)
        .center()
        .decorations(true)
        .resizable(true)
        .skip_taskbar(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn wb_save_layout(panels: Vec<SavedPanel>, app: AppHandle) -> Result<(), String> {
    crate::state::save_layout(app, &panels).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn wb_load_layout(app: AppHandle) -> Result<Vec<SavedPanel>, String> {
    crate::state::load_layout(app).map_err(|e| e.to_string())
}

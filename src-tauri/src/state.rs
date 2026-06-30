use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const DEFAULT_PANEL_POSITION: f64 = 100.0;
const DEFAULT_PANEL_TITLE: &str = "Untitled";
const DEFAULT_PANEL_Z_INDEX: i32 = 1;
const MIN_PANEL_WIDTH: f64 = 80.0;
const MIN_PANEL_HEIGHT: f64 = 64.0;

/// Persisted application state
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PersistedState {
    #[serde(default, rename = "panels")]
    pub _panels: Vec<serde_json::Value>,
    #[serde(default)]
    pub settings: AppSettings,
}

/// Workbench panel layout for persistence
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavedPanel {
    pub id: String,
    pub panel_type: String,
    pub source_hwnd: Option<isize>,
    pub tool_id: Option<String>,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub z_index: i32,
}

fn string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn finite_number_field(value: &serde_json::Value, key: &str) -> Option<f64> {
    let number = value.get(key)?.as_f64()?;
    number.is_finite().then_some(number)
}

fn panel_dimension_field(value: &serde_json::Value, key: &str, min: f64) -> Option<f64> {
    let number = finite_number_field(value, key)?;
    (number >= min).then_some(number)
}

fn positive_isize_field(value: &serde_json::Value, key: &str) -> Option<isize> {
    let number = value.get(key)?.as_i64()?;
    if number > 0 && number <= isize::MAX as i64 {
        Some(number as isize)
    } else {
        None
    }
}

fn z_index_field(value: &serde_json::Value) -> i32 {
    value
        .get("z_index")
        .and_then(serde_json::Value::as_i64)
        .and_then(|number| i32::try_from(number).ok())
        .filter(|number| *number >= 0)
        .unwrap_or(DEFAULT_PANEL_Z_INDEX)
}

fn parse_saved_panel(value: &serde_json::Value) -> Option<SavedPanel> {
    let id = string_field(value, "id")?;
    let panel_type = string_field(value, "panel_type")?;
    let title = string_field(value, "title").unwrap_or_else(|| DEFAULT_PANEL_TITLE.into());
    let x = finite_number_field(value, "x").unwrap_or(DEFAULT_PANEL_POSITION);
    let y = finite_number_field(value, "y").unwrap_or(DEFAULT_PANEL_POSITION);
    let width = panel_dimension_field(value, "width", MIN_PANEL_WIDTH)?;
    let height = panel_dimension_field(value, "height", MIN_PANEL_HEIGHT)?;
    let z_index = z_index_field(value);

    match panel_type.as_str() {
        "thumbnail" => {
            let source_hwnd = positive_isize_field(value, "source_hwnd")?;
            Some(SavedPanel {
                id,
                panel_type,
                source_hwnd: Some(source_hwnd),
                tool_id: None,
                title,
                x,
                y,
                width,
                height,
                z_index,
            })
        }
        "tool" => {
            let tool_id = string_field(value, "tool_id")?;
            Some(SavedPanel {
                id,
                panel_type,
                source_hwnd: None,
                tool_id: Some(tool_id),
                title,
                x,
                y,
                width,
                height,
                z_index,
            })
        }
        _ => None,
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_launch_on_startup", alias = "launch_on_startup")]
    pub launch_on_startup: bool,
    #[serde(default = "default_minimize_to_tray", alias = "minimize_to_tray")]
    pub minimize_to_tray: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_capture_hotkey", alias = "capture_hotkey")]
    pub capture_hotkey: String,
    #[serde(default = "default_language")]
    pub language: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            launch_on_startup: default_launch_on_startup(),
            minimize_to_tray: default_minimize_to_tray(),
            theme: default_theme(),
            capture_hotkey: default_capture_hotkey(),
            language: default_language(),
        }
    }
}

fn default_launch_on_startup() -> bool {
    false
}

fn default_minimize_to_tray() -> bool {
    true
}

fn default_theme() -> String {
    "dark".into()
}

fn default_capture_hotkey() -> String {
    "Ctrl+Shift+W".into()
}

fn default_language() -> String {
    "en".into()
}

impl AppSettings {
    pub fn normalized(mut self) -> Self {
        self.language = normalize_language(&self.language);
        self.theme = normalize_theme(&self.theme);
        self.capture_hotkey = normalize_capture_hotkey(&self.capture_hotkey);
        self
    }
}

fn normalize_language(lang: &str) -> String {
    match lang {
        "zh" | "zh-CN" | "zh_CN" | "chinese" => "zh".into(),
        _ => "en".into(),
    }
}

fn normalize_theme(theme: &str) -> String {
    match theme {
        "light" | "system" => theme.into(),
        _ => "dark".into(),
    }
}

fn normalize_capture_hotkey(hotkey: &str) -> String {
    let trimmed = hotkey.trim();
    if trimmed.is_empty() {
        AppSettings::default().capture_hotkey
    } else {
        trimmed.into()
    }
}

/// Manages state persistence to disk
pub struct AppStateManager {
    state_path: PathBuf,
    pub settings: AppSettings,
}

impl AppStateManager {
    pub fn new() -> Self {
        let state_path = get_state_path();
        Self {
            state_path,
            settings: AppSettings::default(),
        }
    }

    /// Get current settings
    pub fn get_settings(&self) -> &AppSettings {
        &self.settings
    }

    /// Replace all settings after normalizing external values.
    pub fn set_settings(&mut self, settings: AppSettings) -> AppSettings {
        self.settings = settings.normalized();
        self.settings.clone()
    }

    /// Set language and return the new language code
    pub fn set_language(&mut self, lang: &str) -> Result<String, Box<dyn std::error::Error>> {
        let normalized = normalize_language(lang);
        self.settings.language = normalized.clone();
        Ok(normalized)
    }

    /// Get current language
    pub fn get_language(&self) -> String {
        self.settings.language.clone()
    }

    /// Save current app settings to disk.
    pub fn save_settings(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create the directory if it doesn't exist
        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let state = PersistedState {
            _panels: Vec::new(),
            settings: self.settings.clone(),
        };

        let json = serde_json::to_string_pretty(&state)?;
        fs::write(&self.state_path, json)?;

        log::info!("Settings saved to {:?}", self.state_path);
        Ok(())
    }

    /// Load state from disk and restore settings
    pub fn load(&mut self, _app_handle: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        if !self.state_path.exists() {
            log::info!("No saved state found at {:?}", self.state_path);
            return Ok(());
        }

        let json = fs::read_to_string(&self.state_path)?;
        let persisted: PersistedState = serde_json::from_str(&json)?;

        // Restore settings
        self.settings = persisted.settings.normalized();

        log::info!("Loaded settings from {:?}", self.state_path);

        Ok(())
    }
}

/// Get the path to the state file in %APPDATA%
fn get_state_path() -> PathBuf {
    let mut path = dirs_appdata();
    path.push("BetterPanely");
    path.push("state.json");
    path
}

/// Get the %APPDATA% directory
fn dirs_appdata() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata);
        }
    }

    if let Ok(home) = std::env::var("USERPROFILE") {
        let mut path = PathBuf::from(home);
        path.push("AppData");
        path.push("Roaming");
        return path;
    }

    PathBuf::from(".")
}

pub fn save_layout(
    app: AppHandle,
    panels: &[SavedPanel],
) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().data_dir()?;
    let layout_path = data_dir.join("workbench_layout.json");

    if let Some(parent) = layout_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(panels)?;
    fs::write(&layout_path, json)?;

    log::info!("Workbench layout saved to {:?}", layout_path);
    Ok(())
}

pub fn load_layout(app: AppHandle) -> Result<Vec<SavedPanel>, Box<dyn std::error::Error>> {
    let data_dir = app.path().data_dir()?;
    let layout_path = data_dir.join("workbench_layout.json");

    if !layout_path.exists() {
        log::info!("No saved workbench layout found at {:?}", layout_path);
        return Ok(vec![]);
    }

    let json = fs::read_to_string(&layout_path)?;
    let raw_panels: Vec<serde_json::Value> = match serde_json::from_str(&json) {
        Ok(panels) => panels,
        Err(error) => {
            log::warn!(
                "Ignoring invalid workbench layout at {:?}: {}",
                layout_path,
                error
            );
            return Ok(vec![]);
        }
    };

    let mut skipped = 0;
    let mut panels = Vec::with_capacity(raw_panels.len());
    for raw_panel in raw_panels {
        if let Some(panel) = parse_saved_panel(&raw_panel) {
            panels.push(panel);
        } else {
            skipped += 1;
        }
    }

    if skipped > 0 {
        log::warn!(
            "Skipped {} invalid panels while loading workbench layout from {:?}",
            skipped,
            layout_path
        );
    }

    log::info!("Loaded {} panels from workbench layout", panels.len());
    Ok(panels)
}

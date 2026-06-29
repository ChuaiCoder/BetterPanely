use crate::panel_manager::PanelManager;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Persisted application state
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PersistedState {
    pub panels: Vec<PersistedPanel>,
    pub settings: AppSettings,
}

/// A simplified panel for persistence (no runtime-only fields)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersistedPanel {
    pub id: String,
    pub title: String,
    pub tool_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub always_on_top: bool,
    pub opacity: f64,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub launch_on_startup: bool,
    pub minimize_to_tray: bool,
    pub theme: String,
    pub capture_hotkey: String,
    pub language: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            launch_on_startup: false,
            minimize_to_tray: true,
            theme: "dark".into(),
            capture_hotkey: "Ctrl+Shift+W".into(),
            language: "en".into(),
        }
    }
}

/// Manages state persistence to disk
pub struct AppStateManager {
    state_path: PathBuf,
    pub settings: AppSettings,
    loaded_panels: Vec<PersistedPanel>,
}

impl AppStateManager {
    pub fn new() -> Self {
        let state_path = get_state_path();
        Self {
            state_path,
            settings: AppSettings::default(),
            loaded_panels: Vec::new(),
        }
    }

    /// Get current settings
    pub fn get_settings(&self) -> &AppSettings {
        &self.settings
    }

    /// Set language and return the new language code
    pub fn set_language(&mut self, lang: &str) -> Result<String, Box<dyn std::error::Error>> {
        let normalized = match lang {
            "zh" | "zh-CN" | "zh_CN" | "chinese" => "zh",
            _ => "en",
        };
        self.settings.language = normalized.to_string();
        Ok(normalized.to_string())
    }

    /// Get current language
    pub fn get_language(&self) -> String {
        self.settings.language.clone()
    }

    /// Take loaded panels (consumes them, used once at startup)
    pub fn take_loaded_panels(&mut self) -> Vec<PersistedPanel> {
        std::mem::take(&mut self.loaded_panels)
    }

    /// Save current state to disk
    pub fn save(
        &self,
        _app_handle: &AppHandle,
        panel_manager: &PanelManager,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create the directory if it doesn't exist
        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let panels: Vec<PersistedPanel> = panel_manager
            .list()
            .iter()
            .map(|p| PersistedPanel {
                id: p.id.clone(),
                title: p.title.clone(),
                tool_id: match &p.panel_type {
                    crate::panel_manager::panel::PanelType::Tool { tool_id } => {
                        Some(tool_id.clone())
                    }
                    _ => None,
                },
                x: p.x,
                y: p.y,
                width: p.width,
                height: p.height,
                always_on_top: p.always_on_top,
                opacity: p.opacity,
            })
            .collect();

        let state = PersistedState {
            panels,
            settings: self.settings.clone(),
        };

        let json = serde_json::to_string_pretty(&state)?;
        fs::write(&self.state_path, json)?;

        log::info!("State saved to {:?}", self.state_path);
        Ok(())
    }

    /// Load state from disk and restore settings
    pub fn load(
        &mut self,
        _app_handle: &AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.state_path.exists() {
            log::info!("No saved state found at {:?}", self.state_path);
            return Ok(());
        }

        let json = fs::read_to_string(&self.state_path)?;
        let persisted: PersistedState = serde_json::from_str(&json)?;

        // Restore settings
        self.settings = persisted.settings;
        // Store loaded panels for restoration
        self.loaded_panels = persisted.panels;

        log::info!(
            "Loaded state with {} panels from {:?}",
            self.loaded_panels.len(),
            self.state_path
        );

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

pub fn save_layout(app: AppHandle, panels: &[SavedPanel]) -> Result<(), Box<dyn std::error::Error>> {
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
    let panels: Vec<SavedPanel> = serde_json::from_str(&json)?;

    log::info!("Loaded {} panels from workbench layout", panels.len());
    Ok(panels)
}

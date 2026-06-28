use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Panel type discriminator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PanelType {
    #[serde(rename = "tool")]
    Tool { tool_id: String },
    #[serde(rename = "embedded")]
    Embedded {
        embed_info: Option<EmbedInfo>,
    },
}

/// Information about an embedded window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedInfo {
    pub source_hwnd: isize,
    pub source_title: String,
    pub source_exe: String,
    pub original_style: u32,
    pub original_parent: isize,
    pub thread_id: u32,
}

/// A managed panel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Panel {
    pub id: String,
    pub title: String,
    pub panel_type: PanelType,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub always_on_top: bool,
    pub opacity: f64,
    pub click_through: bool,
    /// The Tauri WebView window label (for tool panels)
    #[serde(skip)]
    pub webview_label: Option<String>,
    /// The native container HWND (for embedded panels)
    #[serde(skip)]
    pub container_hwnd: Option<isize>,
}

impl Panel {
    pub fn new(
        id: String,
        title: String,
        panel_type: PanelType,
        width: Option<f64>,
        height: Option<f64>,
    ) -> Self {
        let (default_w, default_h) = match &panel_type {
            PanelType::Tool { tool_id } => match tool_id.as_str() {
                "calculator" => (280.0, 420.0),
                "notes" => (350.0, 400.0),
                "timer" => (300.0, 200.0),
                "weather" => (300.0, 350.0),
                _ => (400.0, 300.0),
            },
            PanelType::Embedded { .. } => (400.0, 300.0),
        };

        Self {
            id,
            title,
            panel_type,
            x: 100.0,
            y: 100.0,
            width: width.unwrap_or(default_w),
            height: height.unwrap_or(default_h),
            always_on_top: false,
            opacity: 1.0,
            click_through: false,
            webview_label: None,
            container_hwnd: None,
        }
    }

    /// Clean up panel resources
    pub fn cleanup(
        &mut self,
        app_handle: &AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match &self.panel_type {
            PanelType::Embedded { embed_info: Some(info) } => {
                // Restore embedded window before cleanup
                crate::window_embedder::release_window(info.source_hwnd, info)?;
            }
            _ => {}
        }

        // Close the WebView if there is one
        if let Some(ref label) = self.webview_label {
            if let Some(webview) = app_handle.get_webview_window(label) {
                let _ = webview.close();
            }
        }

        // Destroy the native container window if it exists
        #[cfg(target_os = "windows")]
        if let Some(hwnd) = self.container_hwnd {
            crate::panel_manager::container::destroy_container(hwnd);
        }

        Ok(())
    }

    /// Create the WebView window for a tool panel
    pub fn create_webview(
        &mut self,
        app_handle: &AppHandle,
        url: &str,
        lang: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let label = format!("panel_{}", self.id);

        let url_with_lang = format!("{}?lang={}", url, lang);

        let _webview = WebviewWindowBuilder::new(
            app_handle,
            &label,
            WebviewUrl::App(url_with_lang.into()),
        )
        .title(&self.title)
        .inner_size(self.width, self.height)
        .position(self.x, self.y)
        .always_on_top(self.always_on_top)
        .decorations(false)
        .resizable(true)
        .visible(true)
        .build()?;

        self.webview_label = Some(label);
        log::info!("Created WebView panel: {}", self.id);
        Ok(())
    }
}

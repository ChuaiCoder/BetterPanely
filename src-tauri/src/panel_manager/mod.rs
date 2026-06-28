pub mod panel;
pub mod container;

use std::collections::HashMap;
use panel::Panel;
use tauri::AppHandle;

/// Central registry for all panels
pub struct PanelManager {
    panels: HashMap<String, Panel>,
    counter: u32,
}

impl PanelManager {
    pub fn new() -> Self {
        Self {
            panels: HashMap::new(),
            counter: 0,
        }
    }

    fn next_id(&mut self) -> String {
        self.counter += 1;
        format!("panel_{}", self.counter)
    }

    pub fn create(
        &mut self,
        title: String,
        panel_type: panel::PanelType,
        width: Option<f64>,
        height: Option<f64>,
    ) -> &Panel {
        let id = self.next_id();
        let panel = Panel::new(id.clone(), title, panel_type, width, height);
        self.panels.insert(id.clone(), panel);
        self.panels.get(&id).unwrap()
    }

    pub fn get(&self, id: &str) -> Option<&Panel> {
        self.panels.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Panel> {
        self.panels.get_mut(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<Panel> {
        self.panels.remove(id)
    }

    pub fn list(&self) -> Vec<&Panel> {
        self.panels.values().collect()
    }

    /// Clean up all panels — restore embedded windows, close tool webviews
    pub fn cleanup_all(
        &mut self,
        app_handle: &AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for (_id, panel) in self.panels.iter_mut() {
            panel.cleanup(app_handle)?;
        }
        self.panels.clear();
        Ok(())
    }
}

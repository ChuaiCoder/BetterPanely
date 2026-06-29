use std::collections::HashMap;
use std::sync::Mutex;
use super::dwm::*;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Dwm::DWM_THUMBNAIL_PROPERTIES;
use windows::Win32::UI::WindowsAndMessaging::IsWindow;

pub struct ThumbnailManager {
    thumbnails: HashMap<String, ThumbnailHandle>,
    next_id: u32,
}

pub struct ThumbnailHandle {
    source_hwnd: isize,
    thumbnail_id: DwmThumbnailId,
    dest_rect: RECT,
    visible: bool,
    opacity: f32,
}

impl ThumbnailManager {
    pub fn new() -> Self {
        Self {
            thumbnails: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn next_panel_id(&mut self) -> String {
        self.next_id += 1;
        format!("wb_panel_{}", self.next_id)
    }

    #[cfg(target_os = "windows")]
    pub unsafe fn register(
        &mut self,
        dest_hwnd: isize,
        source_hwnd: isize,
        panel_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !IsWindow(windows::Win32::Foundation::HWND(dest_hwnd as *mut _)).as_bool() {
            return Err("Workbench window is no longer available".into());
        }
        if !IsWindow(windows::Win32::Foundation::HWND(source_hwnd as *mut _)).as_bool() {
            return Err("Thumbnail source window is no longer available".into());
        }

        if let Some(old_handle) = self.thumbnails.remove(panel_id) {
            let _ = unregister_thumbnail(old_handle.thumbnail_id);
        }

        let thumbnail_id = register_thumbnail(
            windows::Win32::Foundation::HWND(dest_hwnd as *mut _),
            windows::Win32::Foundation::HWND(source_hwnd as *mut _),
        )?;

        let handle = ThumbnailHandle {
            source_hwnd,
            thumbnail_id,
            dest_rect: RECT { left: 0, top: 0, right: 100, bottom: 100 },
            visible: true,
            opacity: 1.0,
        };

        self.thumbnails.insert(panel_id.to_string(), handle);

        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub unsafe fn update_rect(
        &mut self,
        panel_id: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if width <= 0 || height <= 0 {
            return Err("Thumbnail destination must have positive dimensions".into());
        }

        let source_hwnd = self
            .thumbnails
            .get(panel_id)
            .ok_or("Thumbnail not found")?
            .source_hwnd;
        if !IsWindow(windows::Win32::Foundation::HWND(source_hwnd as *mut _)).as_bool() {
            if let Some(handle) = self.thumbnails.remove(panel_id) {
                let _ = unregister_thumbnail(handle.thumbnail_id);
            }
            return Err("Thumbnail source window is no longer available".into());
        }

        let handle = self.thumbnails.get_mut(panel_id).ok_or("Thumbnail not found")?;

        handle.dest_rect = RECT {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        };

        const DWM_TNP_RECTDESTINATION: u32 = 0x00000001;
        const DWM_TNP_OPACITY: u32 = 0x00000004;
        const DWM_TNP_VISIBLE: u32 = 0x00000008;
        const DWM_TNP_SOURCECLIENTAREAONLY: u32 = 0x00000010;

        let props = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: DWM_TNP_RECTDESTINATION
                | DWM_TNP_OPACITY
                | DWM_TNP_VISIBLE
                | DWM_TNP_SOURCECLIENTAREAONLY,
            rcDestination: handle.dest_rect,
            rcSource: RECT { left: 0, top: 0, right: 0, bottom: 0 },
            opacity: (handle.opacity * 255.0) as u8,
            fVisible: handle.visible.into(),
            fSourceClientAreaOnly: true.into(),
        };

        update_thumbnail_properties(handle.thumbnail_id, &props)?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub unsafe fn unregister_by_panel_id(
        &mut self,
        panel_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(handle) = self.thumbnails.remove(panel_id) {
            let _ = unregister_thumbnail(handle.thumbnail_id);
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub unsafe fn unregister_all(&mut self) {
        for (_, handle) in self.thumbnails.drain() {
            let _ = unregister_thumbnail(handle.thumbnail_id);
        }
    }

}

#[derive(Clone)]
pub struct SharedThumbnailManager {
    inner: std::sync::Arc<Mutex<ThumbnailManager>>,
}

impl SharedThumbnailManager {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(Mutex::new(ThumbnailManager::new())),
        }
    }

    pub fn register(
        &self,
        dest_hwnd: isize,
        source_hwnd: isize,
        panel_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.inner.lock().unwrap().register(dest_hwnd, source_hwnd, panel_id)
        }
    }

    pub fn update_rect(
        &self,
        panel_id: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.inner.lock().unwrap().update_rect(panel_id, x, y, width, height)
        }
    }

    pub fn unregister(&self, panel_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.inner.lock().unwrap().unregister_by_panel_id(panel_id)
        }
    }

    pub fn unregister_all(&self) {
        unsafe {
            self.inner.lock().unwrap().unregister_all()
        }
    }

    pub fn next_panel_id(&self) -> String {
        self.inner.lock().unwrap().next_panel_id()
    }

}

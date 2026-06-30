use super::dwm::*;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Dwm::DWM_THUMBNAIL_PROPERTIES;
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::IsWindow;

const EVENT_OBJECT_DESTROY_ID: u32 = 0x8001;
const OBJID_WINDOW_ID: i32 = 0;
const CHILDID_SELF_ID: i32 = 0;
const WINEVENT_OUTOFCONTEXT_FLAG: u32 = 0x0000;
const WINEVENT_SKIPOWNPROCESS_FLAG: u32 = 0x0002;

static SOURCE_LIFECYCLE_APP: OnceLock<AppHandle> = OnceLock::new();
static SOURCE_LIFECYCLE_MANAGER: OnceLock<SharedThumbnailManager> = OnceLock::new();

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceClosedPayload {
    pub panel_id: String,
    pub source_hwnd: isize,
}

pub struct ThumbnailManager {
    thumbnails: HashMap<String, ThumbnailHandle>,
    next_id: u32,
    source_lifecycle_hook: Option<isize>,
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
            source_lifecycle_hook: None,
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
            dest_rect: RECT {
                left: 0,
                top: 0,
                right: 100,
                bottom: 100,
            },
            visible: true,
            opacity: 1.0,
        };

        self.thumbnails.insert(panel_id.to_string(), handle);

        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub unsafe fn ensure_source_lifecycle_hook(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.source_lifecycle_hook.is_some() {
            return Ok(());
        }

        let hook = SetWinEventHook(
            EVENT_OBJECT_DESTROY_ID,
            EVENT_OBJECT_DESTROY_ID,
            None,
            Some(source_destroyed_callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT_FLAG | WINEVENT_SKIPOWNPROCESS_FLAG,
        );

        if hook.0.is_null() {
            return Err("Failed to install source window lifecycle hook".into());
        }

        self.source_lifecycle_hook = Some(hook.0 as isize);
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
        let right = x
            .checked_add(width)
            .ok_or("Thumbnail destination rectangle is out of range")?;
        let bottom = y
            .checked_add(height)
            .ok_or("Thumbnail destination rectangle is out of range")?;

        let handle = self
            .thumbnails
            .get_mut(panel_id)
            .ok_or("Thumbnail not found")?;

        handle.dest_rect = RECT {
            left: x,
            top: y,
            right,
            bottom,
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
            rcSource: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
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

    #[cfg(target_os = "windows")]
    pub unsafe fn unregister_by_source_hwnd(
        &mut self,
        source_hwnd: isize,
    ) -> Vec<SourceClosedPayload> {
        let panel_ids: Vec<String> = self
            .thumbnails
            .iter()
            .filter_map(|(panel_id, handle)| {
                (handle.source_hwnd == source_hwnd).then(|| panel_id.clone())
            })
            .collect();

        panel_ids
            .into_iter()
            .filter_map(|panel_id| {
                self.thumbnails.remove(&panel_id).map(|handle| {
                    let _ = unregister_thumbnail(handle.thumbnail_id);
                    SourceClosedPayload {
                        panel_id,
                        source_hwnd: handle.source_hwnd,
                    }
                })
            })
            .collect()
    }
}

impl Drop for ThumbnailManager {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        if let Some(hook) = self.source_lifecycle_hook.take() {
            unsafe {
                let _ = UnhookWinEvent(HWINEVENTHOOK(hook as *mut _));
            }
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

    fn lock_manager(&self) -> Result<MutexGuard<'_, ThumbnailManager>, Box<dyn std::error::Error>> {
        self.inner
            .lock()
            .map_err(|_| "Thumbnail manager lock is poisoned".into())
    }

    pub fn register(
        &self,
        dest_hwnd: isize,
        source_hwnd: isize,
        panel_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.lock_manager()?
                .register(dest_hwnd, source_hwnd, panel_id)
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
            self.lock_manager()?
                .update_rect(panel_id, x, y, width, height)
        }
    }

    pub fn unregister(&self, panel_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        unsafe { self.lock_manager()?.unregister_by_panel_id(panel_id) }
    }

    pub fn unregister_all(&self) {
        match self.lock_manager() {
            Ok(mut manager) => unsafe {
                manager.unregister_all();
            },
            Err(error) => {
                log::error!("Failed to unregister thumbnails: {}", error);
            }
        }
    }

    pub fn next_panel_id(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(self.lock_manager()?.next_panel_id())
    }

    #[cfg(target_os = "windows")]
    pub fn install_source_lifecycle_hook(
        &self,
        app: AppHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _ = SOURCE_LIFECYCLE_APP.set(app);
        let _ = SOURCE_LIFECYCLE_MANAGER.set(self.clone());
        unsafe { self.lock_manager()?.ensure_source_lifecycle_hook() }
    }

    #[cfg(target_os = "windows")]
    pub fn unregister_closed_source(&self, source_hwnd: isize) -> Vec<SourceClosedPayload> {
        match self.lock_manager() {
            Ok(mut manager) => unsafe { manager.unregister_by_source_hwnd(source_hwnd) },
            Err(error) => {
                log::error!("Failed to unregister closed thumbnail source: {}", error);
                Vec::new()
            }
        }
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn source_destroyed_callback(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: windows::Win32::Foundation::HWND,
    object_id: i32,
    child_id: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if event != EVENT_OBJECT_DESTROY_ID
        || hwnd.0.is_null()
        || object_id != OBJID_WINDOW_ID
        || child_id != CHILDID_SELF_ID
    {
        return;
    }

    let Some(manager) = SOURCE_LIFECYCLE_MANAGER.get() else {
        return;
    };

    let payloads = manager.unregister_closed_source(hwnd.0 as isize);
    if payloads.is_empty() {
        return;
    }

    let Some(app) = SOURCE_LIFECYCLE_APP.get() else {
        return;
    };

    for payload in payloads {
        if let Some(window) = app.get_webview_window(crate::WORKBENCH_WINDOW_LABEL) {
            let _ = window.emit("thumb:source-closed", payload);
        }
    }
}

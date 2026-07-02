use super::dwm::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard, OnceLock};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Dwm::DWM_THUMBNAIL_PROPERTIES;
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, IsWindow};

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
    dest_hwnd: isize,
    source_hwnd: isize,
    source_size: ThumbnailSourceSize,
    segments: Vec<ThumbnailSegment>,
    opacity: f32,
}

pub struct ThumbnailSegment {
    thumbnail_id: DwmThumbnailId,
    dest_rect: RECT,
    source_rect: Option<RECT>,
    visible: bool,
}

#[cfg(target_os = "windows")]
unsafe fn source_client_size(
    source_hwnd: isize,
) -> Result<ThumbnailSourceSize, Box<dyn std::error::Error>> {
    let mut rect = RECT::default();
    GetClientRect(
        windows::Win32::Foundation::HWND(source_hwnd as *mut _),
        &mut rect,
    )
    .map_err(|e| format!("GetClientRect failed: {:?}", e))?;

    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    if width <= 0 || height <= 0 {
        return Err("Thumbnail source client size is invalid".into());
    }

    Ok(ThumbnailSourceSize { width, height })
}

#[cfg(target_os = "windows")]
unsafe fn apply_thumbnail_properties(
    handle: &ThumbnailHandle,
    segment: &ThumbnailSegment,
) -> Result<(), Box<dyn std::error::Error>> {
    const DWM_TNP_RECTDESTINATION: u32 = 0x00000001;
    const DWM_TNP_RECTSOURCE: u32 = 0x00000002;
    const DWM_TNP_OPACITY: u32 = 0x00000004;
    const DWM_TNP_VISIBLE: u32 = 0x00000008;
    const DWM_TNP_SOURCECLIENTAREAONLY: u32 = 0x00000010;
    let source_rect = segment.source_rect.unwrap_or(RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    });

    let props = DWM_THUMBNAIL_PROPERTIES {
        dwFlags: DWM_TNP_RECTDESTINATION
            | if segment.source_rect.is_some() {
                DWM_TNP_RECTSOURCE
            } else {
                0
            }
            | DWM_TNP_OPACITY
            | DWM_TNP_VISIBLE
            | DWM_TNP_SOURCECLIENTAREAONLY,
        rcDestination: segment.dest_rect,
        rcSource: source_rect,
        opacity: (handle.opacity * 255.0) as u8,
        fVisible: segment.visible.into(),
        fSourceClientAreaOnly: true.into(),
    };

    update_thumbnail_properties(segment.thumbnail_id, &props)
}

#[cfg(target_os = "windows")]
unsafe fn unregister_handle(handle: ThumbnailHandle) {
    for segment in handle.segments {
        let _ = unregister_thumbnail(segment.thumbnail_id);
    }
}

#[cfg(target_os = "windows")]
unsafe fn register_hidden_segment(
    dest_hwnd: isize,
    source_hwnd: isize,
) -> Result<ThumbnailSegment, Box<dyn std::error::Error>> {
    Ok(ThumbnailSegment {
        thumbnail_id: register_thumbnail(
            windows::Win32::Foundation::HWND(dest_hwnd as *mut _),
            windows::Win32::Foundation::HWND(source_hwnd as *mut _),
        )?,
        dest_rect: RECT {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        },
        source_rect: None,
        visible: false,
    })
}

fn rect_width(rect: &RECT) -> i32 {
    rect.right - rect.left
}

fn rect_height(rect: &RECT) -> i32 {
    rect.bottom - rect.top
}

fn intersect_rect(left: &RECT, right: &RECT) -> Option<RECT> {
    let rect = RECT {
        left: left.left.max(right.left),
        top: left.top.max(right.top),
        right: left.right.min(right.right),
        bottom: left.bottom.min(right.bottom),
    };

    (rect_width(&rect) > 0 && rect_height(&rect) > 0).then_some(rect)
}

fn clamp_source_coord(value: i32, max: i32) -> i32 {
    value.clamp(0, max)
}

fn source_rect_for_segment(
    full_dest_rect: &RECT,
    segment_dest_rect: &RECT,
    source_size: ThumbnailSourceSize,
) -> Result<RECT, Box<dyn std::error::Error>> {
    let dest_width = rect_width(full_dest_rect);
    let dest_height = rect_height(full_dest_rect);
    if dest_width <= 0 || dest_height <= 0 || source_size.width <= 0 || source_size.height <= 0 {
        return Err("Thumbnail layout rectangle is invalid".into());
    }

    let source_width = source_size.width as f64;
    let source_height = source_size.height as f64;
    let dest_width = dest_width as f64;
    let dest_height = dest_height as f64;

    let left = (((segment_dest_rect.left - full_dest_rect.left) as f64 / dest_width) * source_width)
        .floor() as i32;
    let top = (((segment_dest_rect.top - full_dest_rect.top) as f64 / dest_height) * source_height)
        .floor() as i32;
    let right = (((segment_dest_rect.right - full_dest_rect.left) as f64 / dest_width)
        * source_width)
        .ceil() as i32;
    let bottom = (((segment_dest_rect.bottom - full_dest_rect.top) as f64 / dest_height)
        * source_height)
        .ceil() as i32;

    let left = clamp_source_coord(left, source_size.width);
    let top = clamp_source_coord(top, source_size.height);
    let mut right = clamp_source_coord(right, source_size.width);
    let mut bottom = clamp_source_coord(bottom, source_size.height);
    if right <= left {
        right = (left + 1).min(source_size.width);
    }
    if bottom <= top {
        bottom = (top + 1).min(source_size.height);
    }

    Ok(RECT {
        left,
        top,
        right,
        bottom,
    })
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
    ) -> Result<ThumbnailSourceSize, Box<dyn std::error::Error>> {
        if !IsWindow(windows::Win32::Foundation::HWND(dest_hwnd as *mut _)).as_bool() {
            return Err("Workbench window is no longer available".into());
        }
        if !IsWindow(windows::Win32::Foundation::HWND(source_hwnd as *mut _)).as_bool() {
            return Err("Thumbnail source window is no longer available".into());
        }

        if let Some(old_handle) = self.thumbnails.remove(panel_id) {
            unregister_handle(old_handle);
        }

        let thumbnail_id = register_thumbnail(
            windows::Win32::Foundation::HWND(dest_hwnd as *mut _),
            windows::Win32::Foundation::HWND(source_hwnd as *mut _),
        )?;
        let source_size = source_client_size(source_hwnd)
            .or_else(|_| query_thumbnail_source_size(thumbnail_id))?;

        let handle = ThumbnailHandle {
            dest_hwnd,
            source_hwnd,
            source_size,
            segments: vec![ThumbnailSegment {
                thumbnail_id,
                dest_rect: RECT {
                    left: 0,
                    top: 0,
                    right: 100,
                    bottom: 100,
                },
                source_rect: None,
                visible: false,
            }],
            opacity: 1.0,
        };

        self.thumbnails.insert(panel_id.to_string(), handle);

        Ok(source_size)
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
                unregister_handle(handle);
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

        if handle.segments.is_empty() {
            handle.segments.push(register_hidden_segment(
                handle.dest_hwnd,
                handle.source_hwnd,
            )?);
        }

        for segment in handle.segments.drain(1..) {
            let _ = unregister_thumbnail(segment.thumbnail_id);
        }

        handle.segments[0].dest_rect = RECT {
            left: x,
            top: y,
            right,
            bottom,
        };
        handle.segments[0].source_rect = None;
        handle.segments[0].visible = true;

        apply_thumbnail_properties(handle, &handle.segments[0])?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub unsafe fn update_layout(
        &mut self,
        panel_id: &str,
        full_dest_rect: RECT,
        visible_dest_rects: Vec<RECT>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if rect_width(&full_dest_rect) <= 0 || rect_height(&full_dest_rect) <= 0 {
            return Err("Thumbnail destination must have positive dimensions".into());
        }

        let source_hwnd = self
            .thumbnails
            .get(panel_id)
            .ok_or("Thumbnail not found")?
            .source_hwnd;
        if !IsWindow(windows::Win32::Foundation::HWND(source_hwnd as *mut _)).as_bool() {
            if let Some(handle) = self.thumbnails.remove(panel_id) {
                unregister_handle(handle);
            }
            return Err("Thumbnail source window is no longer available".into());
        }

        let handle = self
            .thumbnails
            .get_mut(panel_id)
            .ok_or("Thumbnail not found")?;

        let visible_dest_rects: Vec<RECT> = visible_dest_rects
            .into_iter()
            .filter_map(|rect| intersect_rect(&full_dest_rect, &rect))
            .collect();

        if visible_dest_rects.is_empty() {
            for index in 0..handle.segments.len() {
                handle.segments[index].visible = false;
                let _ = apply_thumbnail_properties(handle, &handle.segments[index]);
            }
            return Ok(());
        }

        while handle.segments.len() < visible_dest_rects.len() {
            handle.segments.push(register_hidden_segment(
                handle.dest_hwnd,
                handle.source_hwnd,
            )?);
        }

        for segment in handle.segments.drain(visible_dest_rects.len()..) {
            let _ = unregister_thumbnail(segment.thumbnail_id);
        }

        for (index, dest_rect) in visible_dest_rects.into_iter().enumerate() {
            let source_rect =
                source_rect_for_segment(&full_dest_rect, &dest_rect, handle.source_size)?;
            handle.segments[index].dest_rect = dest_rect;
            handle.segments[index].source_rect = Some(source_rect);
            handle.segments[index].visible = true;
            apply_thumbnail_properties(handle, &handle.segments[index])?;
        }

        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub unsafe fn sync_stack_order(
        &mut self,
        ordered_panel_ids: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if ordered_panel_ids.is_empty() || self.thumbnails.len() <= 1 {
            return Ok(());
        }

        let mut remaining = std::mem::take(&mut self.thumbnails);
        let mut seen = HashSet::new();
        let mut ordered_handles = Vec::new();

        for panel_id in ordered_panel_ids {
            if !seen.insert(panel_id.clone()) {
                continue;
            }
            if let Some(handle) = remaining.remove(&panel_id) {
                ordered_handles.push((panel_id, handle));
            }
        }

        let mut next_stack: Vec<(String, ThumbnailHandle)> = remaining.into_iter().collect();
        next_stack.sort_by(|(left_id, _), (right_id, _)| left_id.cmp(right_id));
        next_stack.extend(ordered_handles);

        let mut first_error: Option<String> = None;

        for (panel_id, mut handle) in next_stack {
            if !IsWindow(windows::Win32::Foundation::HWND(handle.dest_hwnd as *mut _)).as_bool() {
                unregister_handle(handle);
                continue;
            }
            if !IsWindow(windows::Win32::Foundation::HWND(
                handle.source_hwnd as *mut _,
            ))
            .as_bool()
            {
                unregister_handle(handle);
                continue;
            }

            let old_segments = std::mem::take(&mut handle.segments);
            let mut next_segments = Vec::with_capacity(old_segments.len());

            for mut segment in old_segments {
                if let Err(error) = unregister_thumbnail(segment.thumbnail_id) {
                    if first_error.is_none() {
                        first_error = Some(error.to_string());
                    }
                    next_segments.push(segment);
                    continue;
                }

                let thumbnail_id = match register_thumbnail(
                    windows::Win32::Foundation::HWND(handle.dest_hwnd as *mut _),
                    windows::Win32::Foundation::HWND(handle.source_hwnd as *mut _),
                ) {
                    Ok(thumbnail_id) => thumbnail_id,
                    Err(error) => {
                        if first_error.is_none() {
                            first_error = Some(error.to_string());
                        }
                        continue;
                    }
                };

                segment.thumbnail_id = thumbnail_id;
                if segment.visible {
                    if let Err(error) = apply_thumbnail_properties(&handle, &segment) {
                        if first_error.is_none() {
                            first_error = Some(error.to_string());
                        }
                    }
                }
                next_segments.push(segment);
            }

            if next_segments.is_empty() {
                match register_hidden_segment(handle.dest_hwnd, handle.source_hwnd) {
                    Ok(segment) => next_segments.push(segment),
                    Err(error) => {
                        if first_error.is_none() {
                            first_error = Some(error.to_string());
                        }
                    }
                }
            }

            handle.segments = next_segments;
            self.thumbnails.insert(panel_id, handle);
        }

        if let Some(error) = first_error {
            Err(error.into())
        } else {
            Ok(())
        }
    }

    #[cfg(target_os = "windows")]
    pub unsafe fn unregister_by_panel_id(
        &mut self,
        panel_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(handle) = self.thumbnails.remove(panel_id) {
            unregister_handle(handle);
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub unsafe fn unregister_all(&mut self) {
        for (_, handle) in self.thumbnails.drain() {
            unregister_handle(handle);
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
                    let source_hwnd = handle.source_hwnd;
                    unregister_handle(handle);
                    SourceClosedPayload {
                        panel_id,
                        source_hwnd,
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
    ) -> Result<ThumbnailSourceSize, Box<dyn std::error::Error>> {
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

    pub fn update_layout(
        &self,
        panel_id: &str,
        full_dest_rect: RECT,
        visible_dest_rects: Vec<RECT>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.lock_manager()?
                .update_layout(panel_id, full_dest_rect, visible_dest_rects)
        }
    }

    pub fn sync_stack_order(
        &self,
        ordered_panel_ids: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe { self.lock_manager()?.sync_stack_order(ordered_panel_ids) }
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

pub mod setparent;
pub mod enumerator;

use crate::panel_manager::panel::EmbedInfo;

/// Embed a window into a container using SetParent
#[cfg(target_os = "windows")]
pub fn embed_window(
    source_hwnd: isize,
    container_hwnd: isize,
) -> Result<EmbedInfo, Box<dyn std::error::Error>> {
    unsafe { setparent::embed_with_setparent(source_hwnd, container_hwnd) }
}

/// Release an embedded window back to its original state
#[cfg(target_os = "windows")]
pub fn release_window(
    source_hwnd: isize,
    info: &EmbedInfo,
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe { setparent::release_from_setparent(source_hwnd, info) }
}

// Non-Windows stubs
#[cfg(not(target_os = "windows"))]
pub fn embed_window(
    _source_hwnd: isize,
    _container_hwnd: isize,
) -> Result<EmbedInfo, Box<dyn std::error::Error>> {
    Err("Window embedding is only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn release_window(
    _source_hwnd: isize,
    _info: &EmbedInfo,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("Window embedding is only supported on Windows".into())
}

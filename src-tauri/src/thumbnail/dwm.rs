#[cfg(target_os = "windows")]
use windows::{
    Win32::Foundation::*,
    Win32::Graphics::Dwm::*,
};

#[cfg(target_os = "windows")]
pub type DwmThumbnailId = isize;

#[cfg(target_os = "windows")]
#[derive(Clone, Copy, Debug)]
pub struct ThumbnailSourceSize {
    pub width: i32,
    pub height: i32,
}

#[cfg(target_os = "windows")]
pub unsafe fn register_thumbnail(
    dest_hwnd: HWND,
    source_hwnd: HWND,
) -> Result<DwmThumbnailId, Box<dyn std::error::Error>> {
    let thumbnail_id = DwmRegisterThumbnail(dest_hwnd, source_hwnd)
        .map_err(|e| format!("DwmRegisterThumbnail failed: {:?}", e))?;
    Ok(thumbnail_id)
}

#[cfg(target_os = "windows")]
pub unsafe fn query_thumbnail_source_size(
    thumbnail_id: DwmThumbnailId,
) -> Result<ThumbnailSourceSize, Box<dyn std::error::Error>> {
    let size = DwmQueryThumbnailSourceSize(thumbnail_id)
        .map_err(|e| format!("DwmQueryThumbnailSourceSize failed: {:?}", e))?;
    if size.cx <= 0 || size.cy <= 0 {
        return Err("DWM thumbnail source size is invalid".into());
    }

    Ok(ThumbnailSourceSize {
        width: size.cx,
        height: size.cy,
    })
}

#[cfg(target_os = "windows")]
pub unsafe fn update_thumbnail_properties(
    thumbnail_id: DwmThumbnailId,
    props: &DWM_THUMBNAIL_PROPERTIES,
) -> Result<(), Box<dyn std::error::Error>> {
    DwmUpdateThumbnailProperties(thumbnail_id, props)
        .map_err(|e| format!("DwmUpdateThumbnailProperties failed: {:?}", e))?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub unsafe fn unregister_thumbnail(
    thumbnail_id: DwmThumbnailId,
) -> Result<(), Box<dyn std::error::Error>> {
    DwmUnregisterThumbnail(thumbnail_id)
        .map_err(|e| format!("DwmUnregisterThumbnail failed: {:?}", e))?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn register_thumbnail(_: isize, _: isize) -> Result<isize, Box<dyn std::error::Error>> {
    Err("DWM thumbnails only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn update_thumbnail_properties(_: isize, _: &()) -> Result<(), Box<dyn std::error::Error>> {
    Err("DWM thumbnails only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn unregister_thumbnail(_: isize) -> Result<(), Box<dyn std::error::Error>> {
    Err("DWM thumbnails only supported on Windows".into())
}

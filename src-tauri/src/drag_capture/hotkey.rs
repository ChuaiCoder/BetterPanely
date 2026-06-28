#[cfg(target_os = "windows")]
use windows::{
    Win32::UI::WindowsAndMessaging::*,
    Win32::Foundation::*,
};

/// Start hotkey-based window capture mode
/// User presses Ctrl+Shift+W, then clicks on a window to capture it
#[cfg(target_os = "windows")]
pub fn start_hotkey_capture() -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Hotkey capture mode activated — click a window to capture");
    Ok(())
}

/// Find the window at a given screen point
#[cfg(target_os = "windows")]
pub fn find_window_at_point(x: i32, y: i32) -> Option<isize> {
    unsafe {
        let point = POINT { x, y };
        let hwnd = WindowFromPoint(point);
        if hwnd.0 != std::ptr::null_mut() {
            let root = GetAncestor(hwnd, GA_ROOT);
            if root.0 != std::ptr::null_mut() {
                return Some(root.0 as isize);
            }
            return Some(hwnd.0 as isize);
        }
    }
    None
}

/// Get information about the window currently under the cursor
#[cfg(target_os = "windows")]
pub fn get_window_under_cursor() -> Option<(isize, String)> {
    unsafe {
        let mut cursor_pos = POINT::default();
        if GetCursorPos(&mut cursor_pos).is_err() {
            return None;
        }

        let hwnd = WindowFromPoint(cursor_pos);
        if hwnd.0 == std::ptr::null_mut() {
            return None;
        }

        let root = GetAncestor(hwnd, GA_ROOT);
        let target = if root.0 != std::ptr::null_mut() { root } else { hwnd };

        let mut title_buf = [0u16; 256];
        let title_len = GetWindowTextW(target, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

        Some((target.0 as isize, title))
    }
}

// Non-Windows stubs
#[cfg(not(target_os = "windows"))]
pub fn start_hotkey_capture() -> Result<(), Box<dyn std::error::Error>> {
    Err("Hotkey capture is only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn find_window_at_point(_x: i32, _y: i32) -> Option<isize> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn get_window_under_cursor() -> Option<(isize, String)> {
    None
}

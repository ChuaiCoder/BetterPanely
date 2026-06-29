#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::*;

/// Get information about the currently focused foreground window.
#[cfg(target_os = "windows")]
pub fn get_focused_window() -> Option<(isize, String)> {
    unsafe {
        let hwnd = GetForegroundWindow();
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
pub fn get_focused_window() -> Option<(isize, String)> {
    None
}

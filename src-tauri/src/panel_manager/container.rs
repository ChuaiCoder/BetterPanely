#[cfg(target_os = "windows")]
use windows::{
    Win32::UI::WindowsAndMessaging::*,
    Win32::Foundation::*,
    Win32::Graphics::Gdi::*,
};

use std::sync::atomic::{AtomicBool, Ordering};

/// Thread-safe flag to track if the window class is registered
static CLASS_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Helper: unwrap a windows Result<HWND> to a raw pointer, returning null on error
#[cfg(target_os = "windows")]
unsafe fn unwrap_hwnd(r: windows::core::Result<HWND>) -> *mut std::ffi::c_void {
    r.map(|h| h.0).unwrap_or(std::ptr::null_mut())
}

/// Create a native Win32 container window for embedding other windows
#[cfg(target_os = "windows")]
pub fn create_container(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    title: &str,
) -> Result<isize, Box<dyn std::error::Error>> {
    unsafe {
        // Register window class once
        if !CLASS_REGISTERED.load(Ordering::Acquire) {
            let hinstance: HINSTANCE = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
                .map_err(|e| e.to_string())?
                .into();
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(container_wndproc),
                hInstance: hinstance,
                lpszClassName: windows::core::w!("BetterPanelyContainer"),
                hbrBackground: HBRUSH(GetStockObject(BLACK_BRUSH).0),
                ..Default::default()
            };
            if RegisterClassExW(&wc) == 0 {
                return Err("Failed to register container window class".into());
            }
            CLASS_REGISTERED.store(true, Ordering::Release);
        }

        let hinstance: HINSTANCE = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
            .map_err(|e| e.to_string())?
            .into();
        let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            windows::core::w!("BetterPanelyContainer"),
            windows::core::PCWSTR(title_wide.as_ptr()),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            x as i32, y as i32, width as i32, height as i32,
            None, None, hinstance, None,
        ).map_err(|e| e.to_string())?;

        if hwnd.0.is_null() {
            return Err("Failed to create container window".into());
        }

        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);

        Ok(hwnd.0 as isize)
    }
}

/// Window procedure for container windows
#[cfg(target_os = "windows")]
unsafe extern "system" fn container_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            // Draw hint text when container is empty (no child window)
            let child = GetWindow(hwnd, GW_CHILD);
            let child_ptr = unwrap_hwnd(child);
            if child_ptr.is_null() {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                if !hdc.0.is_null() {
                    let mut rect = RECT::default();
                    GetClientRect(hwnd, &mut rect).ok();

                    // Set up text drawing
                    let _ = SetBkMode(hdc, windows::Win32::Graphics::Gdi::TRANSPARENT);
                    let _ = SetTextColor(hdc, windows::Win32::Foundation::COLORREF(0x888888));
                    // Use a simple system font
                    let font = GetStockObject(DEFAULT_GUI_FONT);
                    let old_font = SelectObject(hdc, font);

                    let mut hint: Vec<u16> = "Drop a window here\nor click Embed Window"
                        .encode_utf16()
                        .chain(std::iter::once(0))
                        .collect();
                    let _ = DrawTextW(
                        hdc,
                        &mut hint,
                        &mut rect,
                        DT_CENTER | DT_VCENTER | DT_WORDBREAK,
                    );

                    SelectObject(hdc, old_font);
                    let _ = EndPaint(hwnd, &ps);
                }
            }
            LRESULT(0)
        }
        WM_SIZE => {
            let child_ptr = unwrap_hwnd(GetWindow(hwnd, GW_CHILD));
            if !child_ptr.is_null() {
                let width = (lparam.0 as u32 & 0xFFFF) as i32;
                let height = ((lparam.0 as u32 >> 16) & 0xFFFF) as i32;
                let _ = SetWindowPos(
                    HWND(child_ptr),
                    HWND(std::ptr::null_mut()),
                    0, 0, width, height,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            } else {
                // Repaint to show hint when resized and empty
                let _ = InvalidateRect(hwnd, None, true);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // Don't call PostQuitMessage — that would quit the entire app.
            // The main Tauri event loop handles thread shutdown.
            LRESULT(0)
        }
        WM_CLOSE => {
            // Just hide the window; actual cleanup is done via our command
            let _ = ShowWindow(hwnd, SW_HIDE);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Destroy a native container window
#[cfg(target_os = "windows")]
pub fn destroy_container(hwnd: isize) {
    unsafe {
        let _ = DestroyWindow(HWND(hwnd as *mut std::ffi::c_void));
    }
}

/// Resize and reposition a container window
#[cfg(target_os = "windows")]
pub fn update_container(
    hwnd: isize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        SetWindowPos(
            HWND(hwnd as *mut std::ffi::c_void),
            HWND(std::ptr::null_mut()),
            x as i32, y as i32, width as i32, height as i32,
            SWP_NOZORDER | SWP_NOACTIVATE,
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Non-Windows stubs
#[cfg(not(target_os = "windows"))]
pub fn create_container(
    _x: f64, _y: f64, _width: f64, _height: f64, _title: &str,
) -> Result<isize, Box<dyn std::error::Error>> {
    Err("Container windows are only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn destroy_container(_hwnd: isize) {}

#[cfg(not(target_os = "windows"))]
pub fn update_container(
    _hwnd: isize, _x: f64, _y: f64, _width: f64, _height: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    Err("Container windows are only supported on Windows".into())
}

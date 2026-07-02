use serde::Serialize;
use windows::{
    core::PWSTR,
    Win32::Foundation::*,
    Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED},
    Win32::System::Threading::*,
    Win32::UI::WindowsAndMessaging::*,
};

/// Information about an enumerated window
#[derive(Debug, Clone, Serialize)]
pub struct WindowInfo {
    pub hwnd: isize,
    pub title: String,
    pub exe_path: String,
    pub class_name: String,
    pub is_compatible: bool,
    pub incompatibility_reason: Option<String>,
    pub pid: u32,
    pub rect: WindowRect,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Enumerate all visible top-level windows with compatibility info
pub fn enumerate_windows() -> Result<Vec<WindowInfo>, Box<dyn std::error::Error>> {
    let mut windows: Vec<WindowInfo> = Vec::new();

    unsafe {
        let _ = EnumWindows(
            Some(enumerate_callback),
            LPARAM(&mut windows as *mut Vec<WindowInfo> as isize),
        );
    }

    // Sort: compatible first, then by title
    windows.sort_by(|a, b| {
        b.is_compatible
            .cmp(&a.is_compatible)
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });

    Ok(windows)
}

/// Callback for EnumWindows
unsafe extern "system" fn enumerate_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);

    // Skip invisible windows
    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL::from(true);
    }

    if is_dwm_cloaked(hwnd) {
        return BOOL::from(true);
    }

    let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);

    let is_popup = (style & WS_POPUP.0 as isize) != 0;
    let is_tool = (ex_style & WS_EX_TOOLWINDOW.0 as isize) != 0;
    if is_tool && !is_popup {
        return BOOL::from(true);
    }

    // Get window title
    let mut title_buf = [0u16; 256];
    let title_len = GetWindowTextW(hwnd, &mut title_buf);
    let title = String::from_utf16_lossy(&title_buf[..title_len as usize])
        .trim()
        .to_string();

    if title.is_empty() {
        return BOOL::from(true);
    }

    // Get class name
    let mut class_buf = [0u16; 128];
    let class_len = GetClassNameW(hwnd, &mut class_buf);
    let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);

    // Get process info
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    let exe_path = get_process_path(pid).unwrap_or_default();

    // Get window rect
    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);
    if rect.right <= rect.left || rect.bottom <= rect.top {
        return BOOL::from(true);
    }

    // Compatibility detection
    let (is_compatible, reason) = check_compatibility(hwnd, &exe_path, &class_name);

    // Skip our own windows
    if class_name.contains("BetterPanely") || exe_path.to_lowercase().contains("better-panely") {
        return BOOL::from(true);
    }

    windows.push(WindowInfo {
        hwnd: hwnd.0 as isize,
        title,
        exe_path,
        class_name,
        is_compatible,
        incompatibility_reason: reason,
        pid,
        rect: WindowRect {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        },
    });

    BOOL::from(true)
}

unsafe fn is_dwm_cloaked(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED,
        &mut cloaked as *mut u32 as *mut std::ffi::c_void,
        std::mem::size_of::<u32>() as u32,
    )
    .is_ok()
        && cloaked != 0
}

/// Check if a window can be captured as a workbench thumbnail.
unsafe fn check_compatibility(
    hwnd: HWND,
    exe_path: &str,
    class_name: &str,
) -> (bool, Option<String>) {
    let exe_lower = exe_path.to_lowercase();

    // UWP apps use ApplicationFrameHost.exe as a proxy
    if exe_lower.contains("applicationframehost") {
        return (false, Some("UWP apps cannot be captured".into()));
    }

    // System shell windows
    if class_name == "Progman"
        || class_name == "WorkerW"
        || class_name == "Shell_TrayWnd"
        || class_name == "Button"
    {
        return (false, Some("System shell window".into()));
    }

    // Our own windows
    if class_name.contains("BetterPanely") {
        return (false, Some("BetterPanely window".into()));
    }

    // Check if it's a child window already
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    if (style & WS_CHILD.0) != 0 {
        return (false, Some("Already a child window".into()));
    }

    // Check for elevated process
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if is_process_elevated(pid) {
        return (false, Some("Administrator process (UIPI blocked)".into()));
    }

    // Check for fullscreen / DirectX
    if is_fullscreen_game(hwnd) {
        return (false, Some("Fullscreen / DirectX application".into()));
    }

    (true, None)
}

/// Check if a process is running with elevated privileges
unsafe fn is_process_elevated(pid: u32) -> bool {
    if let Ok(handle) = OpenProcess(
        PROCESS_QUERY_LIMITED_INFORMATION,
        false,
        pid,
    ) {
        use windows::Win32::Security::{
            TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY, GetTokenInformation,
        };
        use windows::Win32::System::Threading::OpenProcessToken;

        let mut token = TOKEN_ELEVATION::default();
        let mut htoken = HANDLE::default();
        if OpenProcessToken(handle, TOKEN_QUERY, &mut htoken).is_ok() {
            let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
            let _ = GetTokenInformation(
                htoken,
                TokenElevation,
                Some(&mut token as *mut TOKEN_ELEVATION as *mut std::ffi::c_void),
                size,
                &mut size,
            );
            let _ = CloseHandle(htoken);
        }
        let _ = CloseHandle(handle);
        return token.TokenIsElevated != 0;
    }
    false
}

/// Check if a window is likely a fullscreen game (DirectX/Vulkan)
unsafe fn is_fullscreen_game(hwnd: HWND) -> bool {
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;

    if (ex_style & WS_EX_TOPMOST.0) != 0 && (style & WS_CAPTION.0) == 0 {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_ok() {
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;

            if w >= screen_w && h >= screen_h {
                return true;
            }
        }
    }
    false
}

/// Get the executable file path for a process ID
unsafe fn get_process_path(pid: u32) -> Result<String, Box<dyn std::error::Error>> {
    if pid == 0 {
        return Ok(String::new());
    }

    // Module-read access rejects many otherwise capturable processes. Limited
    // query access is enough for the executable path and keeps enumeration useful.
    if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
        let mut exe_buf = [0u16; 32768];
        let mut size = exe_buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            PWSTR(exe_buf.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);

        if result.is_ok() && size > 0 {
            return Ok(String::from_utf16_lossy(&exe_buf[..size as usize]));
        }
    }
    Ok(String::new())
}

use crate::panel_manager::panel::EmbedInfo;
use windows::{
    Win32::UI::WindowsAndMessaging::*,
    Win32::System::Threading::*,
    Win32::Foundation::*,
};

/// Embed a source window into a container using SetParent
pub unsafe fn embed_with_setparent(
    source_hwnd: isize,
    container_hwnd: isize,
) -> Result<EmbedInfo, Box<dyn std::error::Error>> {
    let src = HWND(source_hwnd as *mut std::ffi::c_void);
    let container = HWND(container_hwnd as *mut std::ffi::c_void);

    // 1. Validate
    if !IsWindow(src).as_bool() {
        return Err("Source window is not valid".into());
    }
    if !IsWindow(container).as_bool() {
        return Err("Container window is not valid".into());
    }

    // 2. Get source window info
    let mut title_buf = [0u16; 256];
    let title_len = GetWindowTextW(src, &mut title_buf);
    let source_title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

    let source_exe = get_window_exe_path(src).unwrap_or_default();

    // Save original style
    let original_style = GetWindowLongPtrW(src, GWL_STYLE) as u32;

    // Save original parent — GetParent returns Result<HWND> in windows 0.58
    let parent_result = GetParent(src);
    let original_parent = parent_result.map(|h| h.0 as isize).unwrap_or(0);

    // Get source thread ID — GetWindowThreadProcessId takes HWND, returns u32
    let source_thread_id = GetWindowThreadProcessId(src, None);

    // 3. Get container thread ID and attach input
    let container_thread_id = GetWindowThreadProcessId(container, None);
    let _attached = AttachThreadInput(source_thread_id, container_thread_id, true);

    // 4. Remove caption and border styles
    let new_style = original_style & !(WS_CAPTION.0 | WS_THICKFRAME.0);
    let _ = SetWindowLongPtrW(src, GWL_STYLE, new_style as isize);

    // Force style update — SetWindowPos returns Result
    SetWindowPos(
        src,
        HWND(std::ptr::null_mut()),
        0, 0, 0, 0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
    ).ok();

    // 5. SetParent — returns Result<HWND>
    let previous_parent = SetParent(src, container);
    let prev_hwnd = previous_parent.map(|h| h.0).unwrap_or(std::ptr::null_mut());
    if prev_hwnd.is_null() {
        let _ = AttachThreadInput(source_thread_id, container_thread_id, false);
        let _ = SetWindowLongPtrW(src, GWL_STYLE, original_style as isize);
        return Err("SetParent failed".into());
    }

    // 6. Resize source to fill container
    let mut rect = RECT::default();
    GetClientRect(container, &mut rect).ok();

    SetWindowPos(
        src,
        HWND(std::ptr::null_mut()),
        0, 0,
        rect.right, rect.bottom,
        SWP_NOZORDER | SWP_SHOWWINDOW,
    ).ok();

    let embed_info = EmbedInfo {
        source_hwnd,
        source_title,
        source_exe,
        original_style,
        original_parent,
        thread_id: source_thread_id,
    };

    log::info!("Successfully embedded window: hwnd={}", source_hwnd);
    Ok(embed_info)
}

/// Release an embedded window back to its original state
pub unsafe fn release_from_setparent(
    source_hwnd: isize,
    info: &EmbedInfo,
) -> Result<(), Box<dyn std::error::Error>> {
    let src = HWND(source_hwnd as *mut std::ffi::c_void);

    if !IsWindow(src).as_bool() {
        log::warn!("Source window no longer exists, skipping restore");
        return Ok(());
    }

    // Get current parent's thread
    let current_parent = GetParent(src);
    let container_thread_id = current_parent
        .map(|h| GetWindowThreadProcessId(h, None))
        .unwrap_or(0);

    // 1. Restore original style
    let _ = SetWindowLongPtrW(src, GWL_STYLE, info.original_style as isize);

    SetWindowPos(
        src, HWND(std::ptr::null_mut()),
        0, 0, 0, 0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
    ).ok();

    // 2. SetParent back to original
    let target_parent = if info.original_parent != 0
        && IsWindow(HWND(info.original_parent as *mut std::ffi::c_void)).as_bool()
    {
        HWND(info.original_parent as *mut std::ffi::c_void)
    } else {
        GetDesktopWindow()
    };
    SetParent(src, target_parent).ok();

    // 3. DetachThreadInput
    let source_thread_id = GetWindowThreadProcessId(src, None);
    if container_thread_id != 0 {
        let _ = AttachThreadInput(source_thread_id, container_thread_id, false);
    }

    let _ = ShowWindow(src, SW_SHOW);
    SetWindowPos(
        src, HWND(std::ptr::null_mut()),
        100, 100, 400, 300,
        SWP_NOZORDER | SWP_SHOWWINDOW,
    ).ok();

    log::info!("Successfully released window: hwnd={}", source_hwnd);
    Ok(())
}

/// Get the executable path of a window's process
unsafe fn get_window_exe_path(hwnd: HWND) -> Result<String, Box<dyn std::error::Error>> {
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    if pid == 0 {
        return Ok(String::new());
    }

    let process_handle = OpenProcess(
        PROCESS_QUERY_LIMITED_INFORMATION,
        false,
        pid,
    )?;

    let mut exe_buf = [0u16; 260];
    let len = windows::Win32::System::ProcessStatus::K32GetProcessImageFileNameW(
        process_handle,
        &mut exe_buf,
    );

    let _ = CloseHandle(process_handle);

    Ok(String::from_utf16_lossy(&exe_buf[..len as usize]))
}

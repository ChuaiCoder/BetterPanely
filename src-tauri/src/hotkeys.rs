use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const CAPTURE_HOTKEY_EVENT: &str = "tray:capture-hotkey";

pub fn register_capture_hotkey(app_handle: &AppHandle, hotkey: &str) -> Result<(), String> {
    let app_for_event = app_handle.clone();
    app_handle
        .global_shortcut()
        .on_shortcut(hotkey, move |_app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }

            if let Some(window) = app_for_event.get_webview_window(crate::WORKBENCH_WINDOW_LABEL) {
                let _ = window.emit(CAPTURE_HOTKEY_EVENT, ());
            }
        })
        .map_err(|e| e.to_string())
}

pub fn replace_capture_hotkey(
    app_handle: &AppHandle,
    old_hotkey: &str,
    new_hotkey: &str,
) -> Result<(), String> {
    if old_hotkey == new_hotkey {
        if app_handle.global_shortcut().is_registered(new_hotkey) {
            return Ok(());
        }
        return register_capture_hotkey(app_handle, new_hotkey);
    }

    let old_was_registered = app_handle.global_shortcut().is_registered(old_hotkey);
    if old_was_registered {
        app_handle
            .global_shortcut()
            .unregister(old_hotkey)
            .map_err(|e| e.to_string())?;
    }

    match register_capture_hotkey(app_handle, new_hotkey) {
        Ok(()) => Ok(()),
        Err(error) => {
            if old_was_registered {
                let _ = register_capture_hotkey(app_handle, old_hotkey);
            }
            Err(error)
        }
    }
}

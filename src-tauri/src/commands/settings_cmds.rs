use crate::state::AppSettings;
use crate::AppState;
use tauri::{command, AppHandle, Emitter, Manager, State};

/// Get the full application settings
#[command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
    Ok(state_mgr.get_settings().clone())
}

/// Replace, persist, and apply the full application settings.
#[command]
pub async fn set_settings(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    let settings = settings.normalized();

    let old_settings = {
        let state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
        state_mgr.get_settings().clone()
    };
    let startup_changed = old_settings.launch_on_startup != settings.launch_on_startup;
    let hotkey_changed = old_settings.capture_hotkey != settings.capture_hotkey;

    if startup_changed {
        apply_launch_on_startup(settings.launch_on_startup)?;
    }

    #[cfg(target_os = "windows")]
    if hotkey_changed {
        if let Err(error) = crate::hotkeys::replace_capture_hotkey(
            &app_handle,
            &old_settings.capture_hotkey,
            &settings.capture_hotkey,
        ) {
            rollback_launch_on_startup(&old_settings, startup_changed);
            return Err(error);
        }
    }

    let save_result = {
        let mut state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
        let saved_settings = state_mgr.set_settings(settings.clone());
        state_mgr.save_settings().map(|_| {
            let language_changed = old_settings.language != saved_settings.language;
            (saved_settings, language_changed)
        })
    };

    let (saved_settings, language_changed) = match save_result {
        Ok(result) => result,
        Err(error) => {
            #[cfg(target_os = "windows")]
            if hotkey_changed {
                let _ = crate::hotkeys::replace_capture_hotkey(
                    &app_handle,
                    &settings.capture_hotkey,
                    &old_settings.capture_hotkey,
                );
            }
            rollback_launch_on_startup(&old_settings, startup_changed);
            if let Ok(mut state_mgr) = state.state_manager.lock() {
                state_mgr.set_settings(old_settings);
            }
            return Err(error.to_string());
        }
    };

    if language_changed {
        if let Err(error) =
            crate::tray::refresh_tray_language(&app_handle, &saved_settings.language)
        {
            log::warn!("Failed to refresh tray language: {}", error);
        }
        let _ = app_handle.emit("language-changed", &saved_settings.language);
        log::info!("Language changed to: {}", saved_settings.language);
    }

    let _ = app_handle.emit("settings-changed", &saved_settings);
    log::info!("Settings changed");

    Ok(saved_settings)
}

/// Get only the language setting
#[command]
pub async fn get_language(state: State<'_, AppState>) -> Result<String, String> {
    let state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
    Ok(state_mgr.get_language())
}

/// Set the language and persist
#[command]
pub async fn set_language(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    lang: String,
) -> Result<String, String> {
    let (new_lang, saved_settings) = {
        let mut state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
        let lang = state_mgr.set_language(&lang).map_err(|e| e.to_string())?;
        state_mgr.save_settings().map_err(|e| e.to_string())?;
        let saved_settings = state_mgr.get_settings().clone();
        (lang, saved_settings)
    };

    if let Err(error) = crate::tray::refresh_tray_language(&app_handle, &new_lang) {
        log::warn!("Failed to refresh tray language: {}", error);
    }

    let _ = app_handle.emit("language-changed", &new_lang);
    let _ = app_handle.emit("settings-changed", &saved_settings);
    log::info!("Language changed to: {}", new_lang);
    Ok(new_lang)
}

/// Open the settings page as a standalone utility window.
#[command]
pub async fn open_settings(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let state_mgr = state.state_manager.lock().map_err(|e| e.to_string())?;
    let lang = state_mgr.get_language();
    drop(state_mgr);

    let url = format!("src/tools/settings/index.html#lang={}", lang);
    let label = "settings_window";

    if let Some(window) = app_handle.get_webview_window(label) {
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    tauri::WebviewWindowBuilder::new(&app_handle, label, tauri::WebviewUrl::App(url.into()))
        .title("Settings")
        .inner_size(420.0, 520.0)
        .center()
        .decorations(true)
        .resizable(false)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_launch_on_startup(enabled: bool) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        KEY_SET_VALUE, REG_SZ,
    };

    const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    const VALUE_NAME: &str = "BetterPanely";

    fn to_wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn wide_bytes(value: &[u16]) -> &[u8] {
        unsafe { std::slice::from_raw_parts(value.as_ptr() as *const u8, value.len() * 2) }
    }

    fn check(error: WIN32_ERROR, context: &str) -> Result<(), String> {
        if error.0 == ERROR_SUCCESS.0 {
            Ok(())
        } else {
            Err(format!(
                "{}: {}",
                context,
                std::io::Error::from_raw_os_error(error.0 as i32)
            ))
        }
    }

    unsafe {
        let subkey = to_wide(RUN_KEY);
        let name = to_wide(VALUE_NAME);
        let mut key = HKEY::default();

        check(
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey.as_ptr()),
                0,
                KEY_SET_VALUE,
                &mut key,
            ),
            "Failed to open Windows startup registry key",
        )?;

        let result = if enabled {
            let exe = std::env::current_exe()
                .map_err(|e| format!("Failed to resolve current executable: {}", e))?;
            let command = format!("\"{}\"", exe.display());
            let command = to_wide(&command);
            RegSetValueExW(
                key,
                PCWSTR(name.as_ptr()),
                0,
                REG_SZ,
                Some(wide_bytes(&command)),
            )
        } else {
            let error = RegDeleteValueW(key, PCWSTR(name.as_ptr()));
            if error.0 == ERROR_FILE_NOT_FOUND.0 {
                ERROR_SUCCESS
            } else {
                error
            }
        };

        let close_result = RegCloseKey(key);
        check(result, "Failed to update Windows startup registry value")?;
        check(close_result, "Failed to close Windows startup registry key")
    }
}

#[cfg(not(target_os = "windows"))]
fn apply_launch_on_startup(_enabled: bool) -> Result<(), String> {
    Ok(())
}

fn rollback_launch_on_startup(old_settings: &AppSettings, startup_changed: bool) {
    if !startup_changed {
        return;
    }

    if let Err(error) = apply_launch_on_startup(old_settings.launch_on_startup) {
        log::warn!("Failed to roll back launch-on-startup setting: {}", error);
    }
}

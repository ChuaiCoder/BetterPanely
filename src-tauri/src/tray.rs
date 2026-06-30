use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder,
};

const TRAY_ID: &str = "betterpanely-tray";

/// Create the system tray icon and menu with localized labels
pub fn create_tray<R: Runtime>(
    app: &AppHandle<R>,
    lang: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_tray_menu(app, lang)?;

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip(crate::locales::t("tray.tooltip", lang))
        .on_menu_event(move |app_handle, event| {
            handle_tray_menu(app_handle, event.id().as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray
                    .app_handle()
                    .get_webview_window(crate::WORKBENCH_WINDOW_LABEL)
                {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// Refresh tray labels after the persisted language changes.
pub fn refresh_tray_language<R: Runtime>(
    app: &AppHandle<R>,
    lang: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_tray_menu(app, lang)?;

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(menu))?;
        tray.set_tooltip(Some(crate::locales::t("tray.tooltip", lang)))?;
        Ok(())
    } else {
        create_tray(app, lang)
    }
}

fn build_tray_menu<R: Runtime>(
    app: &AppHandle<R>,
    lang: &str,
) -> Result<tauri::menu::Menu<R>, Box<dyn std::error::Error>> {
    use crate::locales::t;

    let new_panel = MenuItemBuilder::with_id("new_panel", t("menu.new_panel", lang)).build(app)?;
    let calculator =
        MenuItemBuilder::with_id("calculator", t("menu.calculator", lang)).build(app)?;
    let notes = MenuItemBuilder::with_id("notes", t("menu.notes", lang)).build(app)?;
    let timer = MenuItemBuilder::with_id("timer", t("menu.timer", lang)).build(app)?;
    let weather = MenuItemBuilder::with_id("weather", t("menu.weather", lang)).build(app)?;
    let show_main = MenuItemBuilder::with_id("show_main", t("menu.show_main", lang)).build(app)?;
    let settings = MenuItemBuilder::with_id("settings", t("menu.settings", lang)).build(app)?;

    let lang_en = MenuItemBuilder::with_id("lang_en", t("menu.lang_en", lang)).build(app)?;
    let lang_zh = MenuItemBuilder::with_id("lang_zh", t("menu.lang_zh", lang)).build(app)?;
    let quit = MenuItemBuilder::with_id("quit", t("menu.quit", lang)).build(app)?;

    let lang_submenu = SubmenuBuilder::new(app, t("menu.language", lang))
        .item(&lang_en)
        .item(&lang_zh)
        .build()?;

    let menu = MenuBuilder::new(app)
        .item(&new_panel)
        .item(&calculator)
        .item(&notes)
        .item(&timer)
        .item(&weather)
        .separator()
        .item(&show_main)
        .item(&settings)
        .item(&lang_submenu)
        .separator()
        .item(&quit)
        .build()?;

    Ok(menu)
}

/// Handle tray menu item clicks
fn handle_tray_menu<R: Runtime>(app_handle: &AppHandle<R>, menu_id: &str) {
    match menu_id {
        "new_panel" => {
            if let Some(window) = app_handle.get_webview_window(crate::WORKBENCH_WINDOW_LABEL) {
                let _ = window.show();
                let _ = window.set_focus();
                let _ = window.emit("tray:new-panel", ());
            }
        }
        "calculator" => {
            if let Some(window) = app_handle.get_webview_window(crate::WORKBENCH_WINDOW_LABEL) {
                let _ = window.show();
                let _ = window.emit("tray:launch-tool", "calculator");
            }
        }
        "notes" => {
            if let Some(window) = app_handle.get_webview_window(crate::WORKBENCH_WINDOW_LABEL) {
                let _ = window.show();
                let _ = window.emit("tray:launch-tool", "notes");
            }
        }
        "timer" => {
            if let Some(window) = app_handle.get_webview_window(crate::WORKBENCH_WINDOW_LABEL) {
                let _ = window.show();
                let _ = window.emit("tray:launch-tool", "timer");
            }
        }
        "weather" => {
            if let Some(window) = app_handle.get_webview_window(crate::WORKBENCH_WINDOW_LABEL) {
                let _ = window.show();
                let _ = window.emit("tray:launch-tool", "weather");
            }
        }
        "show_main" => {
            if let Some(window) = app_handle.get_webview_window(crate::WORKBENCH_WINDOW_LABEL) {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "settings" => {
            // Open settings directly
            let state = app_handle.state::<crate::AppState>();
            let lang = {
                let mgr = state.state_manager.lock().unwrap();
                mgr.get_language()
            };
            let label = "settings_window";
            let title = crate::commands::settings_cmds::settings_window_title(&lang);
            if app_handle.get_webview_window(label).is_some() {
                if let Some(w) = app_handle.get_webview_window(label) {
                    let _ = w.set_title(title);
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            } else {
                let url = format!("src/tools/settings/index.html#lang={}", lang);
                let _webview =
                    WebviewWindowBuilder::new(app_handle, label, WebviewUrl::App(url.into()))
                        .title(title)
                        .inner_size(420.0, 520.0)
                        .center()
                        .decorations(true)
                        .resizable(false)
                        .build();
            }
        }
        "lang_en" => {
            if let Some(window) = app_handle.get_webview_window(crate::WORKBENCH_WINDOW_LABEL) {
                let _ = window.show();
                let _ = window.emit("tray:set-language", "en");
            }
        }
        "lang_zh" => {
            if let Some(window) = app_handle.get_webview_window(crate::WORKBENCH_WINDOW_LABEL) {
                let _ = window.show();
                let _ = window.emit("tray:set-language", "zh");
            }
        }
        "quit" => {
            let thumbnails = app_handle.state::<crate::thumbnail::SharedThumbnailManager>();
            thumbnails.unregister_all();
            app_handle.exit(0);
        }
        _ => {}
    }
}

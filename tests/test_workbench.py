"""
Workbench architecture tests for BetterPanely.

These tests verify the current DWM-thumbnail workbench direction and guard
against reintroducing the removed SetParent/container runtime.
"""


class TestWorkbenchRuntimeShape:
    """Verify the Rust runtime exposes the workbench command surface."""

    def test_legacy_container_runtime_is_removed(self, repo_root):
        removed_files = [
            "src-tauri/src/window_embedder/setparent.rs",
            "src-tauri/src/commands/embed_cmds.rs",
            "src-tauri/src/commands/panel_cmds.rs",
            "src-tauri/src/commands/tool_cmds.rs",
            "src/components/PanelFrame.tsx",
            "src/components/WindowPicker.tsx",
            "src/lib/panel-api.ts",
        ]

        for relative_path in removed_files:
            assert not (repo_root / relative_path).exists(), relative_path

        panel_manager = repo_root / "src-tauri/src/panel_manager"
        if panel_manager.exists():
            assert list(panel_manager.glob("*.rs")) == []

    def test_registered_commands_are_workbench_or_settings_only(self, repo_root):
        lib_rs = (repo_root / "src-tauri/src/lib.rs").read_text(encoding="utf-8")
        commands_mod = (repo_root / "src-tauri/src/commands/mod.rs").read_text(
            encoding="utf-8"
        )

        assert "commands::workbench_cmds::wb_add_thumbnail" in lib_rs
        assert "commands::workbench_cmds::wb_update_thumbnail_rect" in lib_rs
        assert "commands::settings_cmds::set_settings" in lib_rs
        assert "commands::settings_cmds::open_settings" in lib_rs

        forbidden = [
            "create_panel",
            "embed_window",
            "release_window",
            "launch_tool",
            "save_state",
            "load_state",
            "panel_cmds",
            "embed_cmds",
            "tool_cmds",
        ]
        for token in forbidden:
            assert token not in lib_rs
            assert token not in commands_mod

    def test_dwm_thumbnail_backend_is_present(self, repo_root):
        dwm_rs = (repo_root / "src-tauri/src/thumbnail/dwm.rs").read_text(
            encoding="utf-8"
        )
        manager_rs = (repo_root / "src-tauri/src/thumbnail/manager.rs").read_text(
            encoding="utf-8"
        )
        workbench_cmds = (
            repo_root / "src-tauri/src/commands/workbench_cmds.rs"
        ).read_text(encoding="utf-8")
        compact_workbench_cmds = "".join(workbench_cmds.split())

        assert "DwmRegisterThumbnail" in dwm_rs
        assert "DwmUpdateThumbnailProperties" in dwm_rs
        assert "DwmUnregisterThumbnail" in dwm_rs
        assert "DWM_TNP_RECTDESTINATION" in manager_rs
        assert "get_webview_window(\"main\")" in workbench_cmds
        assert ".register(dest_hwnd,source_hwnd,&panel_id)" in compact_workbench_cmds

    def test_thumbnail_source_lifetime_is_checked(self, repo_root):
        manager_rs = (repo_root / "src-tauri/src/thumbnail/manager.rs").read_text(
            encoding="utf-8"
        )
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )
        workbench_api = (repo_root / "src/lib/workbench-api.ts").read_text(
            encoding="utf-8"
        )
        lib_rs = (repo_root / "src-tauri/src/lib.rs").read_text(encoding="utf-8")

        assert "source_hwnd" in manager_rs
        assert "IsWindow" in manager_rs
        assert "Thumbnail source window is no longer available" in manager_rs
        assert "SetWinEventHook" in manager_rs
        assert "EVENT_OBJECT_DESTROY_ID" in manager_rs
        assert "WINEVENT_SKIPOWNPROCESS" in manager_rs
        assert "SourceClosedPayload" in manager_rs
        assert '"thumb:source-closed"' in manager_rs
        assert "install_source_lifecycle_hook" in lib_rs
        assert "listen<SourceClosedPayload>" in canvas_tsx
        assert "removeClosedSourcePanel" in canvas_tsx
        assert "THUMBNAIL_HEALTH_INTERVAL_MS = 30000" in canvas_tsx
        assert "source window is no longer available" in canvas_tsx
        assert "thumb:source-closed" not in workbench_api

    def test_add_panel_dialog_blocks_incompatible_windows(self, repo_root):
        dialog_tsx = (repo_root / "src/components/AddPanelDialog.tsx").read_text(
            encoding="utf-8"
        )
        app_css = (repo_root / "src/App.css").read_text(encoding="utf-8")
        en_locale = (repo_root / "src/lib/locales/en.json").read_text(
            encoding="utf-8"
        )
        zh_locale = (repo_root / "src/lib/locales/zh.json").read_text(
            encoding="utf-8"
        )

        assert "windowInfo.incompatibilityReason" in dialog_tsx
        assert "onError?: (error: unknown) => void" in dialog_tsx
        assert "props.onError?.(error)" in dialog_tsx
        assert 't("app.windowNotCapturable")' in dialog_tsx
        assert "if (!windowInfo.isCompatible) return" in dialog_tsx
        assert "selectedHwnds().has(w.hwnd) && w.isCompatible" in dialog_tsx
        assert "checked={w.isCompatible && selectedHwnds().has(w.hwnd)}" in dialog_tsx
        assert "disabled={!w.isCompatible}" in dialog_tsx
        assert "window-item-disabled" in dialog_tsx
        assert "window-incompatible-reason" in dialog_tsx
        assert "window-item-disabled" in app_css
        assert "window-incompatible-reason" in app_css
        assert "app.windowNotCapturable" in en_locale
        assert "app.windowNotCapturable" in zh_locale

    def test_panel_drag_starts_from_header_and_thumbnail_content_focuses(self, repo_root):
        thumb_panel = (repo_root / "src/components/ThumbPanel.tsx").read_text(
            encoding="utf-8"
        )
        tool_panel = (repo_root / "src/components/ToolPanel.tsx").read_text(
            encoding="utf-8"
        )
        app_css = (repo_root / "src/App.css").read_text(encoding="utf-8")

        assert 'onMouseDown={handleMouseDown}' not in thumb_panel.split(
            '<div class="panel-header"'
        )[0]
        assert 'onMouseDown={handleMouseDown}' not in tool_panel.split(
            '<div class="panel-header"'
        )[0]
        assert '<div class="panel-header" onMouseDown={handleMouseDown}>' in thumb_panel
        assert '<div class="panel-header" onMouseDown={handleMouseDown}>' in tool_panel
        assert 'closest(".panel-card")' in thumb_panel
        assert 'closest(".panel-card")' in tool_panel
        assert 'class="panel-content panel-content-transparent panel-content-focusable"' in thumb_panel
        assert "onClick={handleFocus}" in thumb_panel
        assert ".panel-content-focusable" in app_css
        assert "cursor: move" in app_css

    def test_workbench_keyboard_shortcuts_use_selected_panel(self, repo_root):
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )
        thumb_panel = (repo_root / "src/components/ThumbPanel.tsx").read_text(
            encoding="utf-8"
        )
        tool_panel = (repo_root / "src/components/ToolPanel.tsx").read_text(
            encoding="utf-8"
        )
        app_css = (repo_root / "src/App.css").read_text(encoding="utf-8")

        assert "selectedPanelId" in canvas_tsx
        assert 'window.addEventListener("keydown", handleKeyDown)' in canvas_tsx
        assert 'window.removeEventListener("keydown", handleKeyDown)' in canvas_tsx
        assert 'key === "n"' in canvas_tsx
        assert 'key === "s"' in canvas_tsx
        assert 'key === "f"' in canvas_tsx
        assert 'e.key === "Delete"' in canvas_tsx
        assert "isEditableShortcutTarget" in canvas_tsx
        assert "isDialogOpen()" in canvas_tsx
        assert "focusSelectedPanel" in canvas_tsx
        assert "focusSource(panel.sourceHwnd)" in canvas_tsx
        assert "saveLayout(panels())" in canvas_tsx
        assert "app.toast.layoutSaved" in canvas_tsx
        assert "isSelected={selectedPanelId() === panel.id}" in canvas_tsx
        assert "onSelect={handleSelectPanel}" in canvas_tsx
        assert "isSelected: boolean" in thumb_panel
        assert "isSelected: boolean" in tool_panel
        assert "props.onSelect(props.panel.id)" in thumb_panel
        assert "props.onSelect(props.panel.id)" in tool_panel
        assert "panel-selected" in thumb_panel
        assert "panel-selected" in tool_panel
        assert ".panel-selected" in app_css

    def test_workbench_user_errors_are_reported_as_toasts(self, repo_root):
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )
        thumb_panel = (repo_root / "src/components/ThumbPanel.tsx").read_text(
            encoding="utf-8"
        )
        app_css = (repo_root / "src/App.css").read_text(encoding="utf-8")
        dialog_tsx = (repo_root / "src/components/AddPanelDialog.tsx").read_text(
            encoding="utf-8"
        )
        en_locale = (repo_root / "src/lib/locales/en.json").read_text(
            encoding="utf-8"
        )
        zh_locale = (repo_root / "src/lib/locales/zh.json").read_text(
            encoding="utf-8"
        )

        assert "interface WorkbenchNotice" in canvas_tsx
        assert "const [notices, setNotices]" in canvas_tsx
        assert "const showNotice" in canvas_tsx
        assert "NOTICE_TIMEOUT_MS" in canvas_tsx
        assert 'class="toast-stack"' in canvas_tsx
        assert "toast-${notice.type}" in canvas_tsx
        assert "app.toast.addThumbnailFailed" in canvas_tsx
        assert "app.toast.captureFailed" in canvas_tsx
        assert "app.toast.enumerateWindowsFailed" in canvas_tsx
        assert "app.toast.focusFailed" in canvas_tsx
        assert "app.toast.layoutSaved" in canvas_tsx
        assert "app.toast.loadLayoutFailed" in canvas_tsx
        assert "app.toast.removePanelFailed" in canvas_tsx
        assert "app.toast.sourceClosed" in canvas_tsx
        assert "app.toast.stalePanelSkipped" in canvas_tsx
        assert "onError={(error) =>" in canvas_tsx
        assert "focusSource" not in thumb_panel
        assert "props.onError?.(error)" in dialog_tsx
        assert "onFocus: (id: string) => void" in thumb_panel
        assert "props.onFocus(props.panel.id)" in thumb_panel
        assert ".toast-stack" in app_css
        assert ".toast-error" in app_css
        assert ".toast-success" in app_css
        assert ".toast-info" in app_css
        assert "app.toast.addThumbnailFailed" in en_locale
        assert "app.toast.addThumbnailFailed" in zh_locale
        assert "app.toast.enumerateWindowsFailed" in en_locale
        assert "app.toast.enumerateWindowsFailed" in zh_locale

    def test_blank_canvas_context_menu_offers_core_actions(self, repo_root):
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )
        app_css = (repo_root / "src/App.css").read_text(encoding="utf-8")
        en_locale = (repo_root / "src/lib/locales/en.json").read_text(
            encoding="utf-8"
        )
        zh_locale = (repo_root / "src/lib/locales/zh.json").read_text(
            encoding="utf-8"
        )

        assert "interface CanvasContextMenu" in canvas_tsx
        assert "const [contextMenu, setContextMenu]" in canvas_tsx
        assert "handleCanvasContextMenu" in canvas_tsx
        assert 'closest(".panel-card")' in canvas_tsx
        assert "handleContextAddPanel" in canvas_tsx
        assert "handleContextSaveLayout" in canvas_tsx
        assert "saveCurrentLayout" in canvas_tsx
        assert 'onContextMenu={handleCanvasContextMenu}' in canvas_tsx
        assert 'class="canvas-context-menu"' in canvas_tsx
        assert 't("app.saveLayout")' in canvas_tsx
        assert ".canvas-context-menu" in app_css
        assert "app.saveLayout" in en_locale
        assert "app.saveLayout" in zh_locale


class TestWorkbenchPersistenceShape:
    """Verify persistence belongs to the workbench layout, not legacy panels."""

    def test_state_manager_uses_workbench_layout_file(self, repo_root):
        state_rs = (repo_root / "src-tauri/src/state.rs").read_text(encoding="utf-8")

        assert "workbench_layout.json" in state_rs
        assert "SavedPanel" in state_rs
        assert "save_layout" in state_rs
        assert "load_layout" in state_rs
        assert "PanelManager" not in state_rs
        assert "PersistedPanel" not in state_rs

    def test_frontend_uses_workbench_and_settings_apis(self, repo_root):
        main_tsx = (repo_root / "src/main.tsx").read_text(encoding="utf-8")
        i18n_tsx = (repo_root / "src/lib/i18n.tsx").read_text(encoding="utf-8")
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )

        assert "./lib/settings-api" in main_tsx
        assert "./settings-api" in i18n_tsx
        assert "../lib/workbench-api" in canvas_tsx
        assert "saveLayout" in canvas_tsx
        assert "captureWindowUnderCursor" in canvas_tsx

    def test_settings_window_has_capability_and_camel_case_contract(self, repo_root):
        capabilities = (
            repo_root / "src-tauri/capabilities/default.json"
        ).read_text(encoding="utf-8")
        state_rs = (repo_root / "src-tauri/src/state.rs").read_text(encoding="utf-8")
        settings_cmds = (
            repo_root / "src-tauri/src/commands/settings_cmds.rs"
        ).read_text(encoding="utf-8")
        lib_rs = (repo_root / "src-tauri/src/lib.rs").read_text(encoding="utf-8")
        main_tsx = (repo_root / "src/main.tsx").read_text(encoding="utf-8")
        settings_api = (repo_root / "src/lib/settings-api.ts").read_text(
            encoding="utf-8"
        )
        theme_ts = (repo_root / "src/lib/theme.ts").read_text(encoding="utf-8")
        app_css = (repo_root / "src/App.css").read_text(encoding="utf-8")
        settings_html = (repo_root / "src/tools/settings/index.html").read_text(
            encoding="utf-8"
        )

        assert '"settings_window"' in capabilities
        assert '"panel_*"' not in capabilities
        assert 'rename_all = "camelCase"' in state_rs
        assert 'alias = "launch_on_startup"' in state_rs
        assert 'alias = "minimize_to_tray"' in state_rs
        assert 'alias = "capture_hotkey"' in state_rs
        assert "s.launchOnStartup" in settings_html
        assert "s.minimizeToTray" in settings_html
        assert "s.captureHotkey" in settings_html
        assert "set_settings" in settings_cmds
        assert "settings.normalized()" in settings_cmds
        assert "apply_launch_on_startup(settings.launch_on_startup)" in settings_cmds
        assert '"settings-changed"' in settings_cmds
        assert "RegSetValueExW" in settings_cmds
        assert "RegDeleteValueW" in settings_cmds
        assert "WindowEvent::CloseRequested" in lib_rs
        assert "minimize_to_tray" in lib_rs
        assert "window.hide()" in lib_rs
        assert "commands::settings_cmds::set_settings" in lib_rs
        assert "getSettings" in main_tsx
        assert "onSettingsChanged" in main_tsx
        assert "applyAppTheme" in main_tsx
        assert "setSettings(settings: AppSettings)" in settings_api
        assert "settings-changed" in settings_api
        assert "dataset.theme" in theme_ts
        assert 'theme === "system"' in theme_ts
        assert ':root[data-theme="light"]' in app_css
        assert "var(--app-bg-start)" in app_css
        assert "tauri.core || tauri" in settings_html
        assert 'tauriInvoke("set_settings", { settings: settings })' in settings_html
        assert "launchOnStartup:" in settings_html
        assert "minimizeToTray:" in settings_html
        assert "captureHotkey:" in settings_html

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
        assert "commands::workbench_cmds::wb_capture_focused_window" in lib_rs
        assert "commands::workbench_cmds::wb_capture_window_under_cursor" not in lib_rs
        assert "commands::workbench_cmds::wb_update_thumbnail_rect" in lib_rs
        assert "commands::workbench_cmds::wb_update_thumbnail_layout" in lib_rs
        assert "commands::workbench_cmds::wb_sync_thumbnail_stack" in lib_rs
        assert "commands::workbench_cmds::wb_open_tool_window" in lib_rs
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
        hotkey_rs = (
            repo_root / "src-tauri/src/drag_capture/hotkey.rs"
        ).read_text(encoding="utf-8")
        compact_workbench_cmds = "".join(workbench_cmds.split())

        assert "DwmRegisterThumbnail" in dwm_rs
        assert "DwmQueryThumbnailSourceSize" in dwm_rs
        assert "DwmUpdateThumbnailProperties" in dwm_rs
        assert "DwmUnregisterThumbnail" in dwm_rs
        assert "DWM_TNP_RECTDESTINATION" in manager_rs
        assert "DWM_TNP_RECTSOURCE" in manager_rs
        assert "source_rect_for_segment" in manager_rs
        assert "GetClientRect" in manager_rs
        assert "source_client_size(source_hwnd)" in manager_rs
        assert "query_thumbnail_source_size(thumbnail_id)" in manager_rs
        assert "AddThumbnailResult" in workbench_cmds
        assert "source_width: source_size.width" in workbench_cmds
        assert "source_height: source_size.height" in workbench_cmds
        assert "wb_sync_thumbnail_stack" in workbench_cmds
        assert "wb_update_thumbnail_layout" in workbench_cmds
        assert "ThumbnailRectInput" in workbench_cmds
        assert "visible_rects" in workbench_cmds
        assert "thumbnail_rect_arg" in workbench_cmds
        assert ".update_layout(&panel_id,full_rect,visible_rects)" in compact_workbench_cmds
        assert ".sync_stack_order(panel_ids)" in compact_workbench_cmds
        assert "get_webview_window(crate::WORKBENCH_WINDOW_LABEL)" in workbench_cmds
        assert ".register(dest_hwnd,source_hwnd,&panel_id)" in compact_workbench_cmds
        assert "wb_capture_focused_window" in workbench_cmds
        assert "get_focused_window" in workbench_cmds
        assert "wb_capture_window_under_cursor" not in workbench_cmds
        assert "Focused window is not eligible for capture" in workbench_cmds
        assert "GetForegroundWindow" in hotkey_rs
        assert "WindowFromPoint" not in hotkey_rs
        assert "get_window_under_cursor" not in hotkey_rs

    def test_runtime_state_locks_return_errors_instead_of_panicking(self, repo_root):
        lib_rs = (repo_root / "src-tauri/src/lib.rs").read_text(encoding="utf-8")
        tray_rs = (repo_root / "src-tauri/src/tray.rs").read_text(encoding="utf-8")
        manager_rs = (repo_root / "src-tauri/src/thumbnail/manager.rs").read_text(
            encoding="utf-8"
        )
        workbench_cmds = (
            repo_root / "src-tauri/src/commands/workbench_cmds.rs"
        ).read_text(encoding="utf-8")
        compact_manager = "".join(manager_rs.split())
        compact_workbench_cmds = "".join(workbench_cmds.split())

        for source in (lib_rs, tray_rs, manager_rs):
            assert "lock().unwrap()" not in source

        assert "fn lock_manager(" in manager_rs
        assert "Thumbnail manager lock is poisoned" in manager_rs
        assert "self.lock_manager()?.register" in compact_manager
        assert "self.lock_manager()?.update_rect" in compact_manager
        assert "self.lock_manager()?.sync_stack_order" in compact_manager
        assert "self.lock_manager()?.unregister_by_panel_id" in compact_manager
        assert "pub fn next_panel_id(&self) -> Result<String" in manager_rs
        assert "Ok(self.lock_manager()?.next_panel_id())" in manager_rs
        assert "Failed to unregister thumbnails" in manager_rs
        assert "Failed to unregister closed thumbnail source" in manager_rs
        assert ".next_panel_id()" in compact_workbench_cmds
        assert ".map_err(|e|e.to_string())?" in compact_workbench_cmds
        assert "Failed to lock application state during startup" in lib_rs
        assert "Application state lock is poisoned" in lib_rs
        assert "Failed to lock application state while closing" in lib_rs
        assert "unwrap_or_else(|error|" in lib_rs
        assert "Failed to lock application state from tray" in tray_rs

    def test_thumbnail_rect_commands_validate_numeric_bounds(self, repo_root):
        workbench_cmds = (
            repo_root / "src-tauri/src/commands/workbench_cmds.rs"
        ).read_text(encoding="utf-8")
        manager_rs = (repo_root / "src-tauri/src/thumbnail/manager.rs").read_text(
            encoding="utf-8"
        )
        compact_workbench_cmds = "".join(workbench_cmds.split())

        assert "fn finite_i32_arg(name: &str, value: f64) -> Result<i32, String>" in workbench_cmds
        assert "value.is_finite()" in workbench_cmds
        assert "Thumbnail {} must be finite" in workbench_cmds
        assert "Thumbnail {} is out of range" in workbench_cmds
        assert "fn positive_i32_arg(name: &str, value: f64) -> Result<i32, String>" in workbench_cmds
        assert "Thumbnail {} must be positive" in workbench_cmds
        assert 'finite_i32_arg("x", x)?' in workbench_cmds
        assert 'finite_i32_arg("y", y)?' in workbench_cmds
        assert 'positive_i32_arg("width", width)?' in workbench_cmds
        assert 'positive_i32_arg("height", height)?' in workbench_cmds
        assert "xasi32" not in compact_workbench_cmds
        assert "widthasi32" not in compact_workbench_cmds
        assert "checked_add(width)" in manager_rs
        assert "checked_add(height)" in manager_rs
        assert "Thumbnail destination rectangle is out of range" in manager_rs
        assert "right: x + width" not in manager_rs
        assert "bottom: y + height" not in manager_rs

    def test_thumbnail_source_lifetime_is_checked(self, repo_root):
        manager_rs = (repo_root / "src-tauri/src/thumbnail/manager.rs").read_text(
            encoding="utf-8"
        )
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )
        thumb_panel = (repo_root / "src/components/ThumbPanel.tsx").read_text(
            encoding="utf-8"
        )
        workbench_api = (repo_root / "src/lib/workbench-api.ts").read_text(
            encoding="utf-8"
        )
        lib_rs = (repo_root / "src-tauri/src/lib.rs").read_text(encoding="utf-8")

        assert "source_hwnd" in manager_rs
        assert "dest_hwnd" in manager_rs
        assert "IsWindow" in manager_rs
        assert "Thumbnail source window is no longer available" in manager_rs
        assert "SetWinEventHook" in manager_rs
        assert "EVENT_OBJECT_DESTROY_ID" in manager_rs
        assert "WINEVENT_SKIPOWNPROCESS" in manager_rs
        assert "SourceClosedPayload" in manager_rs
        assert '"thumb:source-closed"' in manager_rs
        assert "visible: false" in manager_rs
        assert "segments: Vec<ThumbnailSegment>" in manager_rs
        assert "pub struct ThumbnailSegment" in manager_rs
        assert "source_rect: Option<RECT>" in manager_rs
        assert "unregister_handle" in manager_rs
        assert "register_hidden_segment" in manager_rs
        assert "apply_thumbnail_properties" in manager_rs
        assert "pub unsafe fn sync_stack_order" in manager_rs
        assert "HashSet" in manager_rs
        assert "std::mem::take(&mut self.thumbnails)" in manager_rs
        assert "next_stack.extend(ordered_handles)" in manager_rs
        assert "let thumbnail_id = match register_thumbnail" in manager_rs
        assert "segment.thumbnail_id = thumbnail_id;" in manager_rs
        assert "apply_thumbnail_properties(&handle, &segment)" in manager_rs
        assert "install_source_lifecycle_hook" in lib_rs
        assert "listen<SourceClosedPayload>" in canvas_tsx
        assert 'data-thumbnail-panel-id={props.panel.id}' in thumb_panel
        assert 'data-panel-id={props.panel.id}' in thumb_panel
        assert "getThumbnailContentElement" in canvas_tsx
        assert "getPanelCardElement" in canvas_tsx
        assert "element.dataset.thumbnailPanelId === panelId" in canvas_tsx
        assert "element.dataset.panelId === panelId" in canvas_tsx
        assert "window.devicePixelRatio || 1" in canvas_tsx
        assert "const fullCssRect = getThumbnailCssRect(panel)" in canvas_tsx
        assert "const fullRect = cssRectToNative(fullCssRect)" in canvas_tsx
        assert "visibleThumbnailRects(panel, fullCssRect, items)" in canvas_tsx
        assert "updateThumbnailLayout(panel.id, fullRect, visibleRects)" in canvas_tsx
        assert "scheduleThumbnailRectsSync" in canvas_tsx
        assert "waitForNextFrame" in canvas_tsx
        assert "await waitForNextFrame()" in canvas_tsx
        assert 'syncThumbnailRect(newPanel, "add")' in canvas_tsx
        assert 'syncAllThumbnailRects("restore")' in canvas_tsx
        assert "removeClosedSourcePanel" in canvas_tsx
        assert "THUMBNAIL_HEALTH_INTERVAL_MS = 30000" in canvas_tsx
        assert "source window is no longer available" in canvas_tsx
        assert "thumb:source-closed" not in workbench_api
        assert "let panelId: string | null = null;" in canvas_tsx
        assert "const thumbnail = await addThumbnail(hwnd)" in canvas_tsx
        assert "panelId = thumbnail.panelId" in canvas_tsx
        assert "getThumbnailPanelSize(thumbnail)" in canvas_tsx
        assert 'Pick<ThumbnailRegistration, "sourceWidth" | "sourceHeight">' in canvas_tsx
        assert "thumbnail.sourceWidth / thumbnail.sourceHeight" in canvas_tsx
        assert "thumbnailPanelsInStackOrder" in canvas_tsx
        assert "left.zIndex - right.zIndex" in canvas_tsx
        assert "syncThumbnailStack(panelIds)" in canvas_tsx
        assert 'syncThumbnailStackOrder(panels(), "add")' in canvas_tsx
        assert 'syncThumbnailStackOrder(nextPanels, "top")' in canvas_tsx
        assert 'syncThumbnailStackOrder(panels(), "restore")' in canvas_tsx
        assert "await syncThumbnailRect(newPanel);" not in canvas_tsx
        assert canvas_tsx.index("setPanels((prev) => [...prev, newPanel])") < canvas_tsx.index(
            "const synced = await syncThumbnailRect(newPanel"
        )
        assert "if (panelId)" in canvas_tsx
        assert "await removePanel(panelId)" in canvas_tsx
        assert "Failed to clean up thumbnail after add failure" in canvas_tsx
        assert "Failed to clean up thumbnail after sync failure" in canvas_tsx

    def test_thumbnail_overlays_are_clipped_by_higher_panels(self, repo_root):
        manager_rs = (repo_root / "src-tauri/src/thumbnail/manager.rs").read_text(
            encoding="utf-8"
        )
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )
        thumb_panel = (repo_root / "src/components/ThumbPanel.tsx").read_text(
            encoding="utf-8"
        )
        tool_panel = (repo_root / "src/components/ToolPanel.tsx").read_text(
            encoding="utf-8"
        )
        workbench_api = (repo_root / "src/lib/workbench-api.ts").read_text(
            encoding="utf-8"
        )
        workbench_cmds = (
            repo_root / "src-tauri/src/commands/workbench_cmds.rs"
        ).read_text(encoding="utf-8")

        assert "export interface ThumbnailRect" in workbench_api
        assert "updateThumbnailLayout(" in workbench_api
        assert 'invoke("wb_update_thumbnail_layout", { panelId, fullRect, visibleRects })' in workbench_api
        assert "pub fn wb_update_thumbnail_layout" in workbench_cmds
        assert "visible_rects" in workbench_cmds
        assert 'data-panel-id={props.panel.id}' in thumb_panel
        assert 'data-panel-id={props.panel.id}' in tool_panel
        assert "const panelCardRect = (panel: PanelState): CssRect" in canvas_tsx
        assert "const subtractCssRect = (base: CssRect, occluder: CssRect): CssRect[]" in canvas_tsx
        assert "const visibleThumbnailRects = (panel: PanelState, fullRect: CssRect, items: PanelState[])" in canvas_tsx
        assert "item.zIndex > panel.zIndex" in canvas_tsx
        assert "item.zIndex === panel.zIndex && index > panelIndex" in canvas_tsx
        assert "return visible.flatMap((rect) => subtractCssRect(rect, occluder));" in canvas_tsx
        assert "const visibleRects: ThumbnailRect[] = visibleThumbnailRects(panel, fullCssRect, items)" in canvas_tsx
        assert "await updateThumbnailLayout(panel.id, fullRect, visibleRects)" in canvas_tsx
        assert 'scheduleThumbnailRectsSync("add-tool")' in canvas_tsx
        assert 'scheduleThumbnailRectsSync("drag")' in canvas_tsx
        assert "DWM_TNP_RECTSOURCE" in manager_rs
        assert "segment.source_rect.is_some()" in manager_rs
        assert "source_rect_for_segment(&full_dest_rect, &dest_rect, handle.source_size)" in manager_rs
        assert "handle.segments.len() < visible_dest_rects.len()" in manager_rs
        assert "handle.segments.drain(visible_dest_rects.len()..)" in manager_rs

    def test_add_panel_dialog_blocks_incompatible_windows(self, repo_root):
        dialog_tsx = (repo_root / "src/components/AddPanelDialog.tsx").read_text(
            encoding="utf-8"
        )
        app_css = (repo_root / "src/App.css").read_text(encoding="utf-8")
        enumerator_rs = (
            repo_root / "src-tauri/src/window_embedder/enumerator.rs"
        ).read_text(encoding="utf-8")
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
        assert "DwmGetWindowAttribute" in enumerator_rs
        assert "DWMWA_CLOAKED" in enumerator_rs
        assert "is_dwm_cloaked(hwnd)" in enumerator_rs
        assert ".trim()" in enumerator_rs
        assert "if title.is_empty()" in enumerator_rs
        assert "title.is_empty() && !is_popup" not in enumerator_rs
        assert "rect.right <= rect.left || rect.bottom <= rect.top" in enumerator_rs
        assert "QueryFullProcessImageNameW" in enumerator_rs
        assert "PROCESS_QUERY_LIMITED_INFORMATION, false, pid" in enumerator_rs
        assert "PROCESS_NAME_FORMAT(0)" in enumerator_rs
        assert "PWSTR(exe_buf.as_mut_ptr())" in enumerator_rs
        assert "result.is_ok() && size > 0" in enumerator_rs
        assert "K32GetModuleFileNameExW" not in enumerator_rs
        assert "PROCESS_VM_READ" not in enumerator_rs

    def test_add_panel_dialog_ignores_stale_window_enumeration(self, repo_root):
        dialog_tsx = (repo_root / "src/components/AddPanelDialog.tsx").read_text(
            encoding="utf-8"
        )
        effect_block = dialog_tsx.split("createEffect(() => {", 1)[1].split(
            "const filteredWindows = () =>", 1
        )[0]

        assert "let enumerateRequestId = 0" in dialog_tsx
        assert "const requestId = ++enumerateRequestId" in effect_block
        assert "setWindows([])" in effect_block
        assert "if (!props.isOpen)" in effect_block
        assert "if (requestId !== enumerateRequestId || !props.isOpen) return;" in effect_block
        assert effect_block.index("setWindows([])") < effect_block.index("enumerateWindows()")
        assert effect_block.index("if (requestId !== enumerateRequestId") < effect_block.index(
            "setWindows(windowList)"
        )
        assert effect_block.index("if (requestId !== enumerateRequestId") < effect_block.index(
            "props.onError?.(error)"
        )

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

    def test_tool_panel_focus_opens_standalone_tool_window(self, repo_root):
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )
        tool_panel = (repo_root / "src/components/ToolPanel.tsx").read_text(
            encoding="utf-8"
        )
        workbench_api = (repo_root / "src/lib/workbench-api.ts").read_text(
            encoding="utf-8"
        )
        workbench_cmds = (
            repo_root / "src-tauri/src/commands/workbench_cmds.rs"
        ).read_text(encoding="utf-8")
        en_locale = (repo_root / "src/lib/locales/en.json").read_text(
            encoding="utf-8"
        )
        zh_locale = (repo_root / "src/lib/locales/zh.json").read_text(
            encoding="utf-8"
        )

        assert "wb_open_tool_window" in workbench_cmds
        assert "tool_window_config" in workbench_cmds
        assert "refresh_tool_window_titles" in workbench_cmds
        assert "WebviewWindowBuilder::new" in workbench_cmds
        assert "tool_{}_window" in workbench_cmds
        assert ".skip_taskbar(true)" in workbench_cmds
        assert "window.set_skip_taskbar(true)" in workbench_cmds
        assert "window.set_title(crate::locales::t(config.title_key, lang))" in workbench_cmds
        assert "window.set_title(crate::locales::t(config.title_key, &lang))" in workbench_cmds
        assert "openToolWindow(toolId: string)" in workbench_api
        assert 'invoke("wb_open_tool_window"' in workbench_api
        assert "openToolWindow(panel.toolId)" in canvas_tsx
        assert "app.toast.openToolFailed" in canvas_tsx
        assert "onFocus={handleFocusPanel}" in canvas_tsx
        assert "const { t, lang } = useI18n();" in canvas_tsx
        assert "const toolTitle = (toolId: string)" in canvas_tsx
        assert "withLocalizedToolTitle" in canvas_tsx
        assert "syncToolPanelTitles" in canvas_tsx
        assert "lastToolTitleLang" in canvas_tsx
        assert "title: toolTitle(toolId)" in canvas_tsx
        assert "restored.push(constrainPanelPosition(withLocalizedToolTitle" in canvas_tsx
        assert "...getPanelInitialPosition(config.width, config.height)" in canvas_tsx
        assert "onFocus: (id: string) => void" in tool_panel
        assert 'closest(".panel-focus")' in tool_panel
        assert "props.onFocus(props.panel.id)" in tool_panel
        assert "createMemo" in tool_panel
        assert "const iframeUrl = createMemo" in tool_panel
        assert "TOOL_URLS[toolId]" in tool_panel
        assert "return url ? `${url}#lang=${lang()}` : \"\";" in tool_panel
        assert "<Show when={iframeUrl()}>" in tool_panel
        assert "src={iframeUrl()}" in tool_panel
        assert 'class="panel-btn panel-focus"' in tool_panel
        assert 't("app.openToolWindow")' in tool_panel
        assert "app.openToolWindow" in en_locale
        assert "app.openToolWindow" in zh_locale
        assert "app.toast.openToolFailed" in en_locale
        assert "app.toast.openToolFailed" in zh_locale

    def test_unknown_tool_ids_do_not_create_blank_tool_panels(self, repo_root):
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )
        add_tool_block = canvas_tsx.split(
            "const addToolPanel = (toolId: string) =>", 1
        )[1].split("const handleAddThumbnails", 1)[0]

        assert "const TOOL_CONFIG: Record<string, { width: number; height: number }>" in canvas_tsx
        assert "const config = TOOL_CONFIG[toolId]" in add_tool_block
        assert "if (!config)" in add_tool_block
        assert 'console.warn("Ignored unknown tool:", toolId)' in add_tool_block
        assert 't("error.unknownTool", { toolId })' in add_tool_block
        assert "return;" in add_tool_block
        assert '|| { width: 300, height: 300 }' not in add_tool_block
        assert "setPanels((prev) => [...prev, newPanel])" in add_tool_block
        assert add_tool_block.index("if (!config)") < add_tool_block.index(
            "const newPanel: PanelState"
        )

    def test_e2e_window_cleanup_is_process_scoped(self, repo_root):
        conftest_py = (repo_root / "tests/conftest.py").read_text(encoding="utf-8")
        win32_helper = (
            repo_root / "tests/helpers/win32_helper.py"
        ).read_text(encoding="utf-8")
        root_pytest_ini = (repo_root / "pytest.ini").read_text(encoding="utf-8")
        basic_tests = (repo_root / "tests/test_basic.py").read_text(encoding="utf-8")
        i18n_tests = (repo_root / "tests/test_i18n.py").read_text(encoding="utf-8")

        assert 'RUN_E2E_ENV = "BETTERPANELY_RUN_E2E"' in conftest_py
        assert "def pytest_collection_modifyitems(config, items):" in conftest_py
        assert "item.add_marker(skip_e2e)" in conftest_py
        assert 'pytest.skip(f"set {RUN_E2E_ENV}={RUN_E2E_VALUE} to run E2E tests")' in conftest_py
        assert "tmp_path_factory.mktemp(\"betterpanely-e2e-runtime\")" in conftest_py
        assert 'env["APPDATA"] = str(e2e_appdata_dir)' in conftest_py
        assert 'env["LOCALAPPDATA"] = str(local_appdata)' in conftest_py
        assert 'env["TEMP"] = str(temp_dir)' in conftest_py
        assert 'env["TMP"] = str(temp_dir)' in conftest_py
        assert "shell=False" in conftest_py
        assert 'creationflags=getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0)' in conftest_py
        assert "def close_owned_windows(pid: int, title_contains: str | None = None)" in conftest_py
        assert "find_process_windows(pid, title_contains=title_contains)" in conftest_py
        assert "close_owned_windows(proc.pid)" in conftest_py
        assert "wait_for_process_window(proc.pid" in conftest_py
        assert "wait_for_process_window(app_process.pid" in conftest_py
        assert "wait_for_window(" not in conftest_py
        assert "find_betterpanely_windows" not in conftest_py
        assert "QueryFullProcessImageNameW" in win32_helper
        assert "better-panely.exe" in win32_helper
        assert 'if "BetterPanely" in w["title"]' not in win32_helper
        assert 'elif "Settings" in w["title"]' not in win32_helper
        assert "find_process_windows(main_window[\"pid\"])" in basic_tests
        assert "find_window_by_class" not in basic_tests
        assert "find_betterpanely_windows" not in basic_tests
        assert "find_process_windows(main_window[\"pid\"])" in i18n_tests
        assert "find_betterpanely_windows" not in i18n_tests
        assert 'os.environ.get("APPDATA"' not in i18n_tests
        assert "\n        pass" not in i18n_tests
        assert '-m "not e2e"' in root_pytest_ini
        assert not (repo_root / "tests/pytest.ini").exists()

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
        assert "openToolWindow(panel.toolId)" in canvas_tsx
        assert "isSelected: boolean" in thumb_panel
        assert "isSelected: boolean" in tool_panel
        assert "onFocus: (id: string) => void" in tool_panel
        assert "props.onSelect(props.panel.id)" in thumb_panel
        assert "props.onSelect(props.panel.id)" in tool_panel
        assert "panel-selected" in thumb_panel
        assert "panel-selected" in tool_panel
        assert ".panel-selected" in app_css

    def test_tool_panel_close_does_not_depend_on_thumbnail_backend(self, repo_root):
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )
        close_block = canvas_tsx.split(
            "const handleClosePanel = async", 1
        )[1].split("const handleSelectPanel", 1)[0]

        assert "const panel = panels().find((p) => p.id === panelId)" in close_block
        assert 'if (panel?.type === "thumbnail")' in close_block
        assert "await removePanel(panelId)" in close_block
        assert "removePanelState(panelId)" in close_block
        assert close_block.index('if (panel?.type === "thumbnail")') < close_block.index(
            "await removePanel(panelId)"
        )
        assert close_block.index("await removePanel(panelId)") < close_block.index(
            "removePanelState(panelId)"
        )

    def test_thumbnail_close_removes_stale_frontend_panel(self, repo_root):
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )
        close_block = canvas_tsx.split(
            "const handleClosePanel = async", 1
        )[1].split("const handleSelectPanel", 1)[0]

        assert "const isStaleThumbnailError = (error: unknown)" in canvas_tsx
        assert 'message.includes("source window is no longer available")' in canvas_tsx
        assert 'message.includes("thumbnail not found")' in canvas_tsx
        assert "const removePanelState = (panelId: string)" in canvas_tsx
        assert "setDraggedExternalPanelId(null)" in canvas_tsx
        assert "nextPanels = prev.filter((p) => p.id !== panelId)" in canvas_tsx
        assert "return nextPanels;" in canvas_tsx
        assert "if (panel?.type === \"thumbnail\" && isStaleThumbnailError(e))" in close_block
        assert "removePanelState(panel.id)" in close_block
        assert close_block.index("isStaleThumbnailError(e)") < close_block.index(
            "showNotice(t(\"app.toast.removePanelFailed\""
        )

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
        assert "app.toast.openSettingsFailed" in canvas_tsx
        assert "app.toast.openToolFailed" in canvas_tsx
        assert "app.toast.removePanelFailed" in canvas_tsx
        assert "app.toast.eventListenerFailed" in canvas_tsx
        assert "app.toast.thumbnailSyncFailed" in canvas_tsx
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
        assert "app.toast.openSettingsFailed" in en_locale
        assert "app.toast.openSettingsFailed" in zh_locale
        assert "app.toast.openToolFailed" in en_locale
        assert "app.toast.openToolFailed" in zh_locale
        assert "app.toast.eventListenerFailed" in en_locale
        assert "app.toast.eventListenerFailed" in zh_locale
        assert "app.toast.thumbnailSyncFailed" in en_locale
        assert "app.toast.thumbnailSyncFailed" in zh_locale

    def test_workbench_event_listener_setup_is_failure_isolated(self, repo_root):
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )

        assert "const handleEventListenerError = (eventName: string, error: unknown)" in canvas_tsx
        assert "Failed to listen for ${eventName}:" in canvas_tsx
        assert "app.toast.eventListenerFailed" in canvas_tsx
        assert 'handleEventListenerError("tray:new-panel", error)' in canvas_tsx
        assert 'handleEventListenerError("tray:launch-tool", error)' in canvas_tsx
        assert 'handleEventListenerError("tray:capture-hotkey", error)' in canvas_tsx
        assert 'handleEventListenerError("thumb:source-closed", error)' in canvas_tsx
        assert 'handleEventListenerError("drag:entered-workbench", error)' in canvas_tsx
        assert 'handleEventListenerError("drag:moved-workbench", error)' in canvas_tsx
        assert 'handleEventListenerError("drag:ended-workbench", error)' in canvas_tsx
        assert "const unlistenNewPanel = await listen" not in canvas_tsx
        assert "addCleanup(await listen" in canvas_tsx

    def test_settings_button_reports_open_failures(self, repo_root):
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )
        en_locale = (repo_root / "src/lib/locales/en.json").read_text(
            encoding="utf-8"
        )
        zh_locale = (repo_root / "src/lib/locales/zh.json").read_text(
            encoding="utf-8"
        )
        settings_handler = canvas_tsx.split(
            "const handleOpenSettings = async () =>", 1
        )[1].split("const handleMouseUp", 1)[0]

        assert "await invoke(\"open_settings\")" in settings_handler
        assert 'console.error("Failed to open settings:", error)' in settings_handler
        assert "app.toast.openSettingsFailed" in settings_handler
        assert "{ reason: errorMessage(error) }" in settings_handler
        assert 'onClick={() => void handleOpenSettings()}' in canvas_tsx
        assert 'onClick={() => invoke("open_settings")}' not in canvas_tsx
        assert "app.toast.openSettingsFailed" in en_locale
        assert "app.toast.openSettingsFailed" in zh_locale

    def test_workbench_layout_save_failures_are_reported_without_autosave_spam(self, repo_root):
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )
        autosave_block = canvas_tsx.split(
            "saveTimer = window.setTimeout(() => {", 1
        )[1].split("}, 400);", 1)[0]
        save_current_block = canvas_tsx.split(
            "const saveCurrentLayout = () =>", 1
        )[1].split("const focusPanel = async", 1)[0]
        cleanup_block = canvas_tsx.split(
            "onCleanup(() => {", 1
        )[1].split("noticeTimers.forEach", 1)[0]

        assert "let autoSaveFailureNotified = false" in canvas_tsx
        assert "const markSaveLayoutSuccess = () =>" in canvas_tsx
        assert "autoSaveFailureNotified = false" in canvas_tsx
        assert "const reportSaveLayoutFailure = (" in canvas_tsx
        assert 'context: "manual" | "autosave" | "cleanup"' in canvas_tsx
        assert "Failed to save layout (${context}):" in canvas_tsx
        assert 'if (context === "autosave")' in canvas_tsx
        assert "if (autoSaveFailureNotified) return" in canvas_tsx
        assert "autoSaveFailureNotified = true" in canvas_tsx
        assert "app.toast.saveLayoutFailed" in canvas_tsx
        assert "saveLayout(snapshot)" in autosave_block
        assert ".then(markSaveLayoutSuccess)" in autosave_block
        assert 'reportSaveLayoutFailure("autosave", error, true)' in autosave_block
        assert "saveLayout(panels())" in save_current_block
        assert "showNotice(t(\"app.toast.layoutSaved\"), \"success\")" in save_current_block
        assert 'reportSaveLayoutFailure("manual", error, true)' in save_current_block
        assert "saveLayout(panels())" in cleanup_block
        assert ".then(markSaveLayoutSuccess)" in cleanup_block
        assert 'reportSaveLayoutFailure("cleanup", error)' in cleanup_block
        assert "saveLayout(snapshot).catch(console.error)" not in canvas_tsx
        assert 'console.error("Failed to save layout:", error)' not in canvas_tsx

    def test_thumbnail_rect_sync_failures_are_reported_without_drag_spam(self, repo_root):
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )
        en_locale = (repo_root / "src/lib/locales/en.json").read_text(
            encoding="utf-8"
        )
        zh_locale = (repo_root / "src/lib/locales/zh.json").read_text(
            encoding="utf-8"
        )
        sync_rect_block = canvas_tsx.split(
            "const syncThumbnailRect = async", 1
        )[1].split("const getPanelInitialPosition", 1)[0]

        assert "THUMBNAIL_SYNC_NOTICE_COOLDOWN_MS = 10000" in canvas_tsx
        assert "let lastThumbnailSyncFailureNoticeAt = 0" in canvas_tsx
        assert "const reportThumbnailSyncFailure = (context: string, error: unknown)" in canvas_tsx
        assert "Failed to sync thumbnail rect (${context}):" in canvas_tsx
        assert "Date.now()" in canvas_tsx
        assert (
            "now - lastThumbnailSyncFailureNoticeAt < THUMBNAIL_SYNC_NOTICE_COOLDOWN_MS"
            in canvas_tsx
        )
        assert "lastThumbnailSyncFailureNoticeAt = now" in canvas_tsx
        assert "app.toast.thumbnailSyncFailed" in canvas_tsx
        assert "items: PanelState[] = panels()" in sync_rect_block
        assert "reportThumbnailSyncFailure(context, e)" in sync_rect_block
        assert "throw e" not in sync_rect_block
        assert "syncThumbnailStack(panelIds)" in canvas_tsx
        assert "thumbnailPanelsInStackOrder(items).forEach" in canvas_tsx
        assert ".sort((left, right) => left.zIndex - right.zIndex)" in canvas_tsx
        assert "void syncThumbnailRect(panel, context, items)" in canvas_tsx
        assert 'scheduleThumbnailRectsSync("external-drop")' in canvas_tsx
        assert 'scheduleThumbnailRectsSync("drag")' in canvas_tsx
        assert 'syncAllThumbnailRects("drop")' in canvas_tsx
        assert 'scheduleThumbnailRectsSync("add-tool")' in canvas_tsx
        assert 'syncAllThumbnailRects("resize")' in canvas_tsx
        assert 'syncAllThumbnailRects("health-check")' in canvas_tsx
        assert "syncThumbnailRect(panel).catch(console.error)" not in canvas_tsx
        assert "syncThumbnailRect(movedPanel).catch(console.error)" not in canvas_tsx
        assert 'console.error("Failed to update thumbnail rect:", e)' not in canvas_tsx
        assert "app.toast.thumbnailSyncFailed" in en_locale
        assert "app.toast.thumbnailSyncFailed" in zh_locale

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

    def test_dragged_windows_entering_workbench_emit_add_panel_event(self, repo_root):
        drag_mod = (repo_root / "src-tauri/src/drag_capture/mod.rs").read_text(
            encoding="utf-8"
        )
        drag_monitor = (
            repo_root / "src-tauri/src/drag_capture/monitor.rs"
        ).read_text(encoding="utf-8")
        lib_rs = (repo_root / "src-tauri/src/lib.rs").read_text(encoding="utf-8")
        canvas_tsx = (repo_root / "src/components/WorkbenchCanvas.tsx").read_text(
            encoding="utf-8"
        )

        assert "pub mod monitor" in drag_mod
        assert "install_drag_capture_monitor" in lib_rs
        assert "SetWinEventHook" in drag_monitor
        assert "EVENT_SYSTEM_MOVESIZESTART_ID" in drag_monitor
        assert "EVENT_SYSTEM_MOVESIZEEND_ID" in drag_monitor
        assert "WINEVENT_SKIPOWNPROCESS" in drag_monitor
        assert "DragEnteredWorkbenchPayload" in drag_monitor
        assert "DragPositionPayload" in drag_monitor
        assert '"drag:entered-workbench"' in drag_monitor
        assert '"drag:moved-workbench"' in drag_monitor
        assert '"drag:ended-workbench"' in drag_monitor
        assert "cursor_position_in_workbench" in drag_monitor
        assert "payload_for_source" in drag_monitor
        assert "window.is_compatible" in drag_monitor
        assert "listen<DragEnteredWorkbenchPayload>" in canvas_tsx
        assert "listen<DragPositionPayload>" in canvas_tsx
        assert '"drag:entered-workbench"' in canvas_tsx
        assert '"drag:moved-workbench"' in canvas_tsx
        assert '"drag:ended-workbench"' in canvas_tsx
        assert "workbenchClientPositionToCanvas" in canvas_tsx
        assert "addThumbnailPanel(" in canvas_tsx
        assert "event.payload.sourceHwnd" in canvas_tsx
        assert "draggedExternalPanelId" in canvas_tsx
        assert "movePanelToPosition" in canvas_tsx
        assert "initialPosition?: PanelInitialPosition" in canvas_tsx
        assert "interface CanvasSize" in canvas_tsx
        assert "const constrainPanelPosition = (panel: PanelState, size: CanvasSize = canvasSize())" in canvas_tsx
        assert "size.width - panel.width - 8" in canvas_tsx
        assert "size.height - panel.height - 8" in canvas_tsx
        assert "const movedPanel = constrainPanelPosition({" in canvas_tsx
        assert "const nextSize = { width: canvas.clientWidth, height: canvas.clientHeight }" in canvas_tsx
        assert "const constrained = constrainPanelPosition(panel, nextSize)" in canvas_tsx


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
        workbench_api = (repo_root / "src/lib/workbench-api.ts").read_text(
            encoding="utf-8"
        )
        restore_block = canvas_tsx.split(
            "const restoreSavedPanels = async", 1
        )[1].split("onMount", 1)[0]
        compact_restore_block = "".join(restore_block.split())

        assert "./lib/settings-api" in main_tsx
        assert "./settings-api" in i18n_tsx
        assert "../lib/workbench-api" in canvas_tsx
        assert "saveLayout" in canvas_tsx
        assert "captureFocusedWindow" in canvas_tsx
        assert "constthumbnail=awaitaddThumbnail(panel.sourceHwnd)" in compact_restore_block
        assert "panelId=thumbnail.panelId" in compact_restore_block
        assert "...getThumbnailPanelSize(thumbnail,panel.width)" in compact_restore_block
        assert "id:panelId" in compact_restore_block
        assert "visible:true" in compact_restore_block
        assert (
            "await updateThumbnailRect(restoredPanel.id, rect.x, rect.y, rect.width, rect.height)"
            not in restore_block
        )
        assert 'syncAllThumbnailRects("restore");' in canvas_tsx
        assert 'syncThumbnailStackOrder(panels(), "restore");' in canvas_tsx
        assert "await syncThumbnailRect(restoredPanel)" not in restore_block
        assert "await removePanel(panelId)" in restore_block
        assert "Failed to clean up restored thumbnail" in restore_block
        assert "captureWindowUnderCursor" not in canvas_tsx
        assert 'invoke<WindowInfoRaw | null>("wb_capture_focused_window")' in workbench_api
        assert 'invoke<ThumbnailRegistration>("wb_add_thumbnail"' in workbench_api
        assert "syncThumbnailStack(panelIds: string[])" in workbench_api
        assert 'invoke("wb_sync_thumbnail_stack", { panelIds })' in workbench_api
        assert "wb_capture_window_under_cursor" not in workbench_api
        assert "openToolWindow" in canvas_tsx

    def test_main_settings_theme_listener_retries_and_falls_back(self, repo_root):
        main_tsx = (repo_root / "src/main.tsx").read_text(encoding="utf-8")

        assert "SETTINGS_LISTENER_RETRY_MS = 1000" in main_tsx
        assert "SETTINGS_LISTENER_MAX_ATTEMPTS = 3" in main_tsx
        assert "let settingsThemeFallbackInstalled = false" in main_tsx
        assert "async function loadSettingsForBootstrap(): Promise<Lang>" in main_tsx
        assert 'console.error("Failed to load settings during bootstrap:", error)' in main_tsx
        assert "async function refreshThemeFromSettings(context: string)" in main_tsx
        assert "Failed to refresh theme from settings (${context}):" in main_tsx
        assert "function installSettingsThemeFallback()" in main_tsx
        assert "if (settingsThemeFallbackInstalled) return" in main_tsx
        assert 'window.addEventListener("focus", () =>' in main_tsx
        assert 'void refreshThemeFromSettings("focus-fallback")' in main_tsx
        assert "function registerSettingsThemeListener(attempt = 1)" in main_tsx
        assert "onSettingsChanged((settings) => applyAppTheme(settings.theme)).catch((error) =>" in main_tsx
        assert "Failed to listen for settings changes (attempt ${attempt}):" in main_tsx
        assert "attempt < SETTINGS_LISTENER_MAX_ATTEMPTS" in main_tsx
        assert "window.setTimeout(" in main_tsx
        assert "() => registerSettingsThemeListener(attempt + 1)" in main_tsx
        assert "installSettingsThemeFallback()" in main_tsx
        assert "const initialLang = await loadSettingsForBootstrap()" in main_tsx
        assert "registerSettingsThemeListener()" in main_tsx
        assert ".catch(console.error)" not in main_tsx

    def test_saved_layout_deserialization_rejects_invalid_panel_data(self, repo_root):
        state_rs = (repo_root / "src-tauri/src/state.rs").read_text(encoding="utf-8")
        workbench_api = (repo_root / "src/lib/workbench-api.ts").read_text(
            encoding="utf-8"
        )

        assert "parse_saved_panel" in state_rs
        assert "serde_json::Value" in state_rs
        assert 'match panel_type.as_str()' in state_rs
        assert '"thumbnail" => {' in state_rs
        assert '"tool" => {' in state_rs
        assert 'positive_isize_field(value, "source_hwnd")?' in state_rs
        assert 'string_field(value, "tool_id")?' in state_rs
        assert "panel_dimension_field(value, \"width\", MIN_PANEL_WIDTH)?" in state_rs
        assert "panel_dimension_field(value, \"height\", MIN_PANEL_HEIGHT)?" in state_rs
        assert ".filter(|number| *number >= 0)" in state_rs
        assert "Ignoring invalid workbench layout" in state_rs
        assert "Skipped {} invalid panels" in state_rs

        assert "function mapSavedPanel(raw: unknown): PanelState | null" in workbench_api
        assert "function isPanelType(value: unknown): value is PanelType" in workbench_api
        assert 'value === "thumbnail" || value === "tool"' in workbench_api
        assert "panelDimensionField(raw, \"width\", MIN_PANEL_WIDTH)" in workbench_api
        assert "panelDimensionField(raw, \"height\", MIN_PANEL_HEIGHT)" in workbench_api
        assert "positiveNumberField(raw, \"source_hwnd\")" in workbench_api
        assert "value === null || value < 0" in workbench_api
        assert "VALID_TOOL_IDS.has(toolId)" in workbench_api
        assert "invoke<unknown>(\"wb_load_layout\")" in workbench_api
        assert "Array.isArray(result)" in workbench_api
        assert ".filter((panel): panel is PanelState => panel !== null)" in workbench_api
        assert 'as "thumbnail" | "tool"' not in workbench_api

    def test_settings_deserialization_defaults_missing_fields(self, repo_root):
        state_rs = (repo_root / "src-tauri/src/state.rs").read_text(encoding="utf-8")

        assert 'default = "default_launch_on_startup"' in state_rs
        assert 'default = "default_minimize_to_tray"' in state_rs
        assert 'default = "default_theme"' in state_rs
        assert 'default = "default_capture_hotkey"' in state_rs
        assert 'default = "default_language"' in state_rs
        assert "fn default_launch_on_startup() -> bool" in state_rs
        assert "fn default_minimize_to_tray() -> bool" in state_rs
        assert "fn default_theme() -> String" in state_rs
        assert "fn default_capture_hotkey() -> String" in state_rs
        assert "fn default_language() -> String" in state_rs
        assert "launch_on_startup: default_launch_on_startup()" in state_rs
        assert "minimize_to_tray: default_minimize_to_tray()" in state_rs
        assert "theme: default_theme()" in state_rs
        assert "capture_hotkey: default_capture_hotkey()" in state_rs
        assert "language: default_language()" in state_rs

    def test_settings_page_reports_load_failures_to_status_bar(self, repo_root):
        settings_html = (repo_root / "src/tools/settings/index.html").read_text(
            encoding="utf-8"
        )
        load_block = settings_html.split(
            "function loadSettings()", 1
        )[1].split("//", 1)[0]
        save_block = settings_html.split(
            'tauriInvoke("set_settings"', 1
        )[1].split("});", 1)[0]

        assert 'loadError: "Error loading settings"' in settings_html
        assert 'loadError: "加载设置失败"' in settings_html
        assert "function errorMessage(error)" in settings_html
        assert "error && error.message ? error.message : String(error)" in settings_html
        assert 'console.error("Failed to load settings:", e)' in load_block
        assert 'setStatus(t("loadError") + ": " + errorMessage(e), true)' in load_block
        assert 'setStatus(t("error") + ": " + errorMessage(e), true)' in save_block
        assert 'setStatus(t("error") + ": " + e, true)' not in settings_html

    def test_settings_window_has_capability_and_camel_case_contract(self, repo_root):
        capabilities = (
            repo_root / "src-tauri/capabilities/default.json"
        ).read_text(encoding="utf-8")
        state_rs = (repo_root / "src-tauri/src/state.rs").read_text(encoding="utf-8")
        settings_cmds = (
            repo_root / "src-tauri/src/commands/settings_cmds.rs"
        ).read_text(encoding="utf-8")
        lib_rs = (repo_root / "src-tauri/src/lib.rs").read_text(encoding="utf-8")
        hotkeys_rs = (repo_root / "src-tauri/src/hotkeys.rs").read_text(
            encoding="utf-8"
        )
        tauri_conf = (repo_root / "src-tauri/tauri.conf.json").read_text(
            encoding="utf-8"
        )
        workbench_cmds = (
            repo_root / "src-tauri/src/commands/workbench_cmds.rs"
        ).read_text(encoding="utf-8")
        drag_monitor = (
            repo_root / "src-tauri/src/drag_capture/monitor.rs"
        ).read_text(encoding="utf-8")
        thumbnail_manager = (
            repo_root / "src-tauri/src/thumbnail/manager.rs"
        ).read_text(encoding="utf-8")
        tray_rs = (repo_root / "src-tauri/src/tray.rs").read_text(encoding="utf-8")
        main_tsx = (repo_root / "src/main.tsx").read_text(encoding="utf-8")
        settings_api = (repo_root / "src/lib/settings-api.ts").read_text(
            encoding="utf-8"
        )
        theme_ts = (repo_root / "src/lib/theme.ts").read_text(encoding="utf-8")
        app_css = (repo_root / "src/App.css").read_text(encoding="utf-8")
        settings_html = (repo_root / "src/tools/settings/index.html").read_text(
            encoding="utf-8"
        )

        assert '"label": "workbench"' in tauri_conf
        assert '"workbench"' in capabilities
        assert '"main"' not in capabilities
        assert '"settings_window"' in capabilities
        assert '"tool_*_window"' in capabilities
        assert '"panel_*"' not in capabilities
        assert 'WORKBENCH_WINDOW_LABEL: &str = "workbench"' in lib_rs
        assert "window.label() != WORKBENCH_WINDOW_LABEL" in lib_rs
        assert "get_webview_window(crate::WORKBENCH_WINDOW_LABEL)" in workbench_cmds
        assert "get_webview_window(crate::WORKBENCH_WINDOW_LABEL)" in drag_monitor
        assert "get_webview_window(crate::WORKBENCH_WINDOW_LABEL)" in thumbnail_manager
        assert "get_webview_window(crate::WORKBENCH_WINDOW_LABEL)" in tray_rs
        assert "get_webview_window(crate::WORKBENCH_WINDOW_LABEL)" in hotkeys_rs
        assert 'TRAY_ID: &str = "betterpanely-tray"' in tray_rs
        assert "TrayIconBuilder::with_id(TRAY_ID)" in tray_rs
        assert "app.default_window_icon().cloned()" in tray_rs
        assert ".icon(icon)" in tray_rs
        assert ".show_menu_on_left_click(false)" in tray_rs
        assert "build_tray_menu" in tray_rs
        assert "refresh_tray_language" in tray_rs
        assert "app.tray_by_id(TRAY_ID)" in tray_rs
        assert "tray.set_menu(Some(menu))" in tray_rs
        assert "tray.set_tooltip" in tray_rs
        assert '"trayIcon"' not in tauri_conf
        assert 'rename_all = "camelCase"' in state_rs
        assert 'alias = "launch_on_startup"' in state_rs
        assert 'alias = "minimize_to_tray"' in state_rs
        assert 'alias = "capture_hotkey"' in state_rs
        assert "s.launchOnStartup" in settings_html
        assert "s.minimizeToTray" in settings_html
        assert "s.captureHotkey" in settings_html
        assert "set_settings" in settings_cmds
        assert "settings.normalized()" in settings_cmds
        assert (
            "let startup_changed = old_settings.launch_on_startup != settings.launch_on_startup"
            in settings_cmds
        )
        assert "let old_settings = state_mgr.get_settings().clone();" in settings_cmds
        assert "apply_launch_on_startup(settings.launch_on_startup)" in settings_cmds
        assert "rollback_launch_on_startup(&old_settings, startup_changed)" in settings_cmds
        assert "apply_launch_on_startup(old_settings.launch_on_startup)" in settings_cmds
        assert '"settings-changed"' in settings_cmds
        assert "RegSetValueExW" in settings_cmds
        assert "RegDeleteValueW" in settings_cmds
        assert "replace_capture_hotkey" in settings_cmds
        assert (
            "crate::tray::refresh_tray_language(&app_handle, &saved_settings.language)"
            in settings_cmds
        )
        assert "crate::tray::refresh_tray_language(&app_handle, &new_lang)" in settings_cmds
        assert "let lang = state_mgr.set_language(&lang)" in settings_cmds
        assert "if let Err(error) = state_mgr.save_settings()" in settings_cmds
        assert "state_mgr.set_settings(old_settings);" in settings_cmds
        assert "return Err(error.to_string());" in settings_cmds
        assert "settings_window_title" in settings_cmds
        assert "pub fn open_settings_window<R: Runtime>(" in settings_cmds
        assert "refresh_localized_window_titles" in settings_cmds
        assert 'crate::locales::t("menu.settings", lang)' in settings_cmds
        assert "window.set_title(settings_window_title(lang))" in settings_cmds
        assert "refresh_tool_window_titles(app_handle, lang)" in settings_cmds
        assert "refresh_localized_window_titles(&app_handle, &saved_settings.language)" in settings_cmds
        assert "refresh_localized_window_titles(&app_handle, &new_lang)" in settings_cmds
        assert ".skip_taskbar(true)" in settings_cmds
        assert "window.set_skip_taskbar(true)" in settings_cmds
        assert ".title(title)" in settings_cmds
        assert '.title("Settings")' not in settings_cmds
        assert "settings_cmds::open_settings_window(app_handle, &lang)" in tray_rs
        assert "Failed to open settings window from tray" in tray_rs
        assert "WebviewWindowBuilder" not in tray_rs
        assert '.title("Settings")' not in tray_rs
        assert "register_capture_hotkey(app.handle(), &capture_hotkey)" in lib_rs
        assert "ShortcutState::Pressed" in hotkeys_rs
        assert ".unregister(old_hotkey)" in hotkeys_rs
        assert "register_capture_hotkey(app_handle, new_hotkey)" in hotkeys_rs
        assert "register_capture_hotkey(app_handle, old_hotkey)" in hotkeys_rs
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
        assert 'id="hotkeyInput"' in settings_html
        assert 'document.getElementById("hotkeyInput").value' in settings_html

    def test_i18n_event_listeners_handle_failures_without_dynamic_import(self, repo_root):
        i18n_tsx = (repo_root / "src/lib/i18n.tsx").read_text(encoding="utf-8")

        assert 'import { listen } from "@tauri-apps/api/event";' in i18n_tsx
        assert 'import("@tauri-apps/api/event")' not in i18n_tsx
        assert "function isLang(value: string): value is Lang" in i18n_tsx
        assert "if (isLang(newLang))" in i18n_tsx
        assert 'console.error("Failed to listen for language changes:", error)' in i18n_tsx
        assert 'listen<string>("tray:set-language", (event) =>' in i18n_tsx
        assert "void setLang(newLang).catch((error) =>" in i18n_tsx
        assert 'console.error("Failed to set language from tray:", error)' in i18n_tsx
        assert 'console.error("Failed to listen for tray language changes:", error)' in i18n_tsx

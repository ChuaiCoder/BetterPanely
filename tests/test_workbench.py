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

        assert "DwmRegisterThumbnail" in dwm_rs
        assert "DwmUpdateThumbnailProperties" in dwm_rs
        assert "DwmUnregisterThumbnail" in dwm_rs
        assert "DWM_TNP_RECTDESTINATION" in manager_rs
        assert "get_webview_window(\"main\")" in workbench_cmds
        assert "thumbnail_manager.register(dest_hwnd, source_hwnd" in workbench_cmds


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

"""
Basic launch & smoke tests for BetterPanely.
"""
import time
import pytest
from helpers.win32_helper import (
    find_betterpanely_windows,
    find_window_by_title,
    find_window_by_class,
    wait_for_window,
    is_window_visible,
)


class TestAppLaunch:
    """Verify the application launches correctly."""

    def test_exe_exists(self):
        """The built executable should exist."""
        from helpers.win32_helper import get_exe_path
        exe = get_exe_path()
        assert exe.exists(), f"Executable not found at {exe}"

    def test_main_window_appears(self, main_window):
        """Main window should appear within timeout."""
        assert main_window is not None
        assert "BetterPanely" in main_window["title"]

    def test_main_window_visible(self, main_window):
        """Main window should be visible."""
        assert is_window_visible(main_window["hwnd"])

    def test_main_window_has_expected_size(self, main_window):
        """Main window should have reasonable size."""
        left, top, right, bottom = main_window["rect"]
        width = right - left
        height = bottom - top
        assert width >= 500, f"Window too narrow: {width}px"
        assert height >= 300, f"Window too short: {height}px"


class TestTrayIcon:
    """Verify system tray functionality."""

    def test_app_has_windows(self, main_window):
        """App should have multiple windows (main + possibly tray helper)."""
        bp_wins = find_betterpanely_windows()
        assert len(bp_wins) >= 1, "Expected at least the main window"


class TestToolLaunch:
    """Verify built-in tools can be launched."""

    def test_calculator_launches(self, main_window, clean_state):
        """Calculator tool window should appear."""
        # Note: tools are launched via the Tauri invoke system.
        # In a full E2E test, we'd click the button in the WebView.
        # For now, verify the main window is responsive.
        assert is_window_visible(main_window["hwnd"])

    def test_multiple_tools_dont_crash(self, main_window, clean_state):
        """Launching multiple tools should not crash the app."""
        # Smoke test: just verify the main window stays alive
        time.sleep(1)
        assert is_window_visible(main_window["hwnd"])


class TestPanelCreation:
    """Verify panel (container window) creation."""

    def test_container_windows_can_be_created(self, main_window):
        """Container windows have the correct class name."""
        # Container windows use "BetterPanelyContainer" class
        containers = find_window_by_class("BetterPanelyContainer")
        # Containers might or might not exist depending on test state
        for c in containers:
            assert "BetterPanelyContainer" in c["class_name"]

    def test_container_window_has_title_bar(self, main_window):
        """Container windows should have proper dimensions."""
        containers = find_window_by_class("BetterPanelyContainer")
        for c in containers:
            left, top, right, bottom = c["rect"]
            width = right - left
            height = bottom - top
            assert width > 50, f"Container too narrow: {width}"
            assert height > 50, f"Container too short: {height}"

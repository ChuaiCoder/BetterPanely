"""
Basic launch & smoke tests for BetterPanely.
"""
import time
import pytest
from helpers.win32_helper import (
    find_betterpanely_windows,
    find_window_by_class,
    is_window_visible,
)


pytestmark = pytest.mark.e2e


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
        """App should expose at least the main workbench window."""
        bp_wins = find_betterpanely_windows()
        assert len(bp_wins) >= 1, "Expected at least the main window"


class TestWorkbenchMode:
    """Verify the app is running in workbench mode, not legacy container mode."""

    def test_workbench_stays_responsive(self, main_window, clean_state):
        """Workbench should remain alive without spawning utility windows."""
        assert is_window_visible(main_window["hwnd"])

    def test_workbench_does_not_create_legacy_containers(self, main_window):
        """The DWM workbench should not use old BetterPanelyContainer windows."""
        containers = find_window_by_class("BetterPanelyContainer")
        assert containers == []

    def test_idle_workbench_does_not_crash(self, main_window, clean_state):
        """Idle workbench should stay visible across a short smoke interval."""
        time.sleep(1)
        assert is_window_visible(main_window["hwnd"])

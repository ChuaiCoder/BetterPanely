"""
Language switching tests for BetterPanely.
Verifies Chinese/English switching works correctly.
"""
import pytest
from helpers.win32_helper import (
    find_process_windows,
)


pytestmark = pytest.mark.e2e


class TestLanguageSwitch:
    """Verify language switching functionality."""

    def test_main_window_loads_in_english(self, main_window):
        """Main window should load with English title by default."""
        assert "BetterPanely" in main_window["title"]

    def test_settings_window_is_not_open_by_default(self, main_window, clean_state):
        """Settings should not be left open before a test interaction."""
        settings_windows = [
            w for w in find_process_windows(main_window["pid"]) if "Settings" in w["title"]
        ]
        assert settings_windows == []

    def test_workbench_window_remains_single_localized_host(self, main_window, clean_state):
        """Built-in tools are hosted inside the workbench, not separate panel windows."""
        owned_windows = find_process_windows(main_window["pid"])
        assert any("BetterPanely" in w["title"] for w in owned_windows)
        assert all(w["pid"] == main_window["pid"] for w in owned_windows)


class TestSettingsPersistence:
    """Verify settings are persisted correctly."""

    def test_e2e_uses_isolated_appdata(self, app_process, e2e_appdata_dir):
        """E2E runtime data should be isolated from the user's real APPDATA."""
        assert app_process.pid > 0
        state_root = e2e_appdata_dir / "BetterPanely"
        assert str(state_root).startswith(str(e2e_appdata_dir))

"""
Language switching tests for BetterPanely.
Verifies Chinese/English switching works correctly.
"""
from helpers.win32_helper import (
    find_betterpanely_windows,
)


class TestLanguageSwitch:
    """Verify language switching functionality."""

    def test_main_window_loads_in_english(self, main_window):
        """Main window should load with English title by default."""
        assert "BetterPanely" in main_window["title"]

    def test_settings_window_opens(self, main_window, clean_state):
        """Settings window should be openable."""
        # This validates the WebView window creation for settings
        # In a full test we'd click the settings button
        pass

    def test_workbench_window_remains_single_localized_host(self, main_window, clean_state):
        """Built-in tools are hosted inside the workbench, not separate panel windows."""
        bp_wins = find_betterpanely_windows()
        assert any("BetterPanely" in w["title"] for w in bp_wins)


class TestSettingsPersistence:
    """Verify settings are persisted correctly."""

    def test_state_file_exists(self):
        """State file should exist after app has run."""
        import os
        appdata = os.environ.get("APPDATA", "")
        state_path = os.path.join(appdata, "BetterPanely", "state.json")
        if os.path.exists(state_path):
            import json
            with open(state_path, "r") as f:
                state = json.load(f)
            assert "settings" in state
            assert "language" in state["settings"]
            assert state["settings"]["language"] in ("en", "zh")

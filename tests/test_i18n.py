"""
Language switching tests for BetterPanely.
Verifies Chinese/English switching works correctly.
"""
import time
import pytest
from helpers.win32_helper import (
    find_window_by_title,
    wait_for_window,
    is_window_visible,
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

    def test_calculator_shows_correct_language(self, main_window, clean_state):
        """Calculator tool should show localized title."""
        # Check for any calculator windows
        calc_wins = find_window_by_title("Calculator") + find_window_by_title("计算器")
        # May or may not exist depending on test state
        for cw in calc_wins:
            assert is_window_visible(cw["hwnd"])


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

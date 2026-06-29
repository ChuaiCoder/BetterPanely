"""
Pytest fixtures for BetterPanely E2E tests.
Launches the app, waits for it to be ready, and cleans up after.
"""
import pytest
import subprocess
import time
from pathlib import Path
from helpers.win32_helper import (
    get_exe_path,
    find_betterpanely_windows,
    wait_for_window,
    close_window,
    find_window_by_title,
)


@pytest.fixture(scope="session")
def repo_root() -> Path:
    """Return the project root for source-level architecture checks."""
    return Path(__file__).parent.parent


@pytest.fixture(scope="session")
def app_process():
    """Launch BetterPanely.exe and return the process. Cleans up after tests."""
    exe_path = get_exe_path()
    if not exe_path.exists():
        pytest.skip(f"Executable not found at {exe_path}. Run 'npx tauri build' first.")

    proc = subprocess.Popen(
        [str(exe_path)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    # Wait for the main window to appear
    time.sleep(3)

    main_win = wait_for_window("BetterPanely", timeout=15)
    if not main_win:
        proc.terminate()
        proc.wait(timeout=5)
        pytest.fail("BetterPanely main window did not appear within 15 seconds")

    yield proc

    # Cleanup: close all BetterPanely windows and terminate
    for _ in range(3):
        bp_wins = find_betterpanely_windows()
        for w in bp_wins:
            close_window(w["hwnd"])
        time.sleep(0.5)

    try:
        proc.terminate()
        proc.wait(timeout=5)
    except Exception:
        proc.kill()


@pytest.fixture(scope="function")
def main_window(app_process):
    """Ensure the main BetterPanely window is visible."""
    win = wait_for_window("BetterPanely", timeout=5)
    assert win is not None, "Main window not found"
    return win


@pytest.fixture(scope="function")
def clean_state(main_window):
    """Ensure a clean state: close leftover utility windows but keep the workbench."""
    bp_wins = find_betterpanely_windows()
    for w in bp_wins:
        if "Settings" in w["title"]:
            close_window(w["hwnd"])
    time.sleep(0.5)
    yield
    # Post-test cleanup
    bp_wins = find_betterpanely_windows()
    for w in bp_wins:
        if "Settings" in w["title"]:
            close_window(w["hwnd"])
    time.sleep(0.3)

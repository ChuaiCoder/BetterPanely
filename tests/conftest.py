"""
Pytest fixtures for BetterPanely tests.

E2E tests are intentionally double-gated: they are excluded by pytest.ini and
still skip unless BETTERPANELY_RUN_E2E=1 is set. When enabled, they only manage
the BetterPanely process launched by this fixture and use an isolated APPDATA.
"""
import os
import pytest
import subprocess
import time
from pathlib import Path
from helpers.win32_helper import (
    get_exe_path,
    find_process_windows,
    wait_for_process_window,
    close_window,
)

RUN_E2E_ENV = "BETTERPANELY_RUN_E2E"
RUN_E2E_VALUE = "1"


@pytest.fixture(scope="session")
def repo_root() -> Path:
    """Return the project root for source-level architecture checks."""
    return Path(__file__).parent.parent


def pytest_collection_modifyitems(config, items):
    """Skip E2E tests unless they are explicitly enabled for this process."""
    if os.environ.get(RUN_E2E_ENV) == RUN_E2E_VALUE:
        return

    reason = f"set {RUN_E2E_ENV}={RUN_E2E_VALUE} to run window-launching E2E tests"
    skip_e2e = pytest.mark.skip(reason=reason)
    for item in items:
        if "e2e" in item.keywords:
            item.add_marker(skip_e2e)


@pytest.fixture(scope="session")
def e2e_runtime_dir(tmp_path_factory) -> Path:
    """Isolated runtime root for app data written by E2E tests."""
    return tmp_path_factory.mktemp("betterpanely-e2e-runtime")


@pytest.fixture(scope="session")
def e2e_appdata_dir(e2e_runtime_dir: Path) -> Path:
    appdata = e2e_runtime_dir / "AppData" / "Roaming"
    appdata.mkdir(parents=True, exist_ok=True)
    return appdata


@pytest.fixture(scope="session")
def e2e_env(e2e_runtime_dir: Path, e2e_appdata_dir: Path) -> dict[str, str]:
    """Environment for the launched app process, isolated from the user profile."""
    local_appdata = e2e_runtime_dir / "AppData" / "Local"
    temp_dir = e2e_runtime_dir / "Temp"
    local_appdata.mkdir(parents=True, exist_ok=True)
    temp_dir.mkdir(parents=True, exist_ok=True)

    env = os.environ.copy()
    env["APPDATA"] = str(e2e_appdata_dir)
    env["LOCALAPPDATA"] = str(local_appdata)
    env["TEMP"] = str(temp_dir)
    env["TMP"] = str(temp_dir)
    return env


def close_owned_windows(pid: int, title_contains: str | None = None) -> None:
    """Close only top-level windows owned by the launched BetterPanely process."""
    for window in find_process_windows(pid, title_contains=title_contains):
        close_window(window["hwnd"])


def stop_process(proc: subprocess.Popen) -> None:
    """Stop only the subprocess created by app_process."""
    if proc.poll() is not None:
        return

    for _ in range(3):
        close_owned_windows(proc.pid)
        time.sleep(0.5)
        if proc.poll() is not None:
            return

    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)


@pytest.fixture(scope="session")
def app_process(e2e_env: dict[str, str]):
    """Launch BetterPanely.exe and return the process. Cleans up after tests."""
    if os.environ.get(RUN_E2E_ENV) != RUN_E2E_VALUE:
        pytest.skip(f"set {RUN_E2E_ENV}={RUN_E2E_VALUE} to run E2E tests")

    exe_path = get_exe_path()
    if not exe_path.exists():
        pytest.skip(f"Executable not found at {exe_path}. Run 'npx tauri build' first.")

    proc = subprocess.Popen(
        [str(exe_path)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        env=e2e_env,
        shell=False,
        creationflags=getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0),
    )

    try:
        main_win = wait_for_process_window(proc.pid, title_contains="BetterPanely", timeout=15)
        if not main_win:
            stop_process(proc)
            pytest.fail("BetterPanely main window did not appear within 15 seconds")

        yield proc
    finally:
        stop_process(proc)


@pytest.fixture(scope="function")
def main_window(app_process):
    """Ensure the main BetterPanely window is visible."""
    win = wait_for_process_window(app_process.pid, title_contains="BetterPanely", timeout=5)
    assert win is not None, "Main window not found"
    return win


@pytest.fixture(scope="function")
def clean_state(main_window):
    """Ensure a clean state: close leftover utility windows but keep the workbench."""
    owner_pid = main_window["pid"]
    close_owned_windows(owner_pid, title_contains="Settings")
    time.sleep(0.5)
    yield
    close_owned_windows(owner_pid, title_contains="Settings")
    time.sleep(0.3)

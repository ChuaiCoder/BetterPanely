"""
Win32 helper utilities for interacting with BetterPanely from Python tests.
Uses ctypes (stdlib) — no external dependencies.
"""
import ctypes
from ctypes import wintypes
import time
import subprocess
import os
from pathlib import Path

# ─── Win32 API bindings ─────────────────────────────────

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32

# Types
HWND = wintypes.HWND
RECT = ctypes.c_void_p  # We'll use our own RECT
LPARAM = wintypes.LPARAM
WPARAM = wintypes.WPARAM
UINT = wintypes.UINT
DWORD = wintypes.DWORD
BOOL = wintypes.BOOL
HANDLE = wintypes.HANDLE

# Constants
WM_CLOSE = 0x0010
SW_SHOW = 5
SW_RESTORE = 9

# Callback type for EnumWindows
EnumWindowsProc = ctypes.WINFUNCTYPE(BOOL, HWND, LPARAM)


class RECTStruct(ctypes.Structure):
    _fields_ = [
        ("left", ctypes.c_long),
        ("top", ctypes.c_long),
        ("right", ctypes.c_long),
        ("bottom", ctypes.c_long),
    ]


def enum_windows() -> list[dict]:
    """Enumerate all visible top-level windows."""
    windows = []

    def callback(hwnd, lparam):
        if not user32.IsWindowVisible(hwnd):
            return True
        length = user32.GetWindowTextLengthW(hwnd)
        title_buf = ctypes.create_unicode_buffer(length + 1)
        user32.GetWindowTextW(hwnd, title_buf, length + 1)
        title = title_buf.value or ""

        class_buf = ctypes.create_unicode_buffer(128)
        user32.GetClassNameW(hwnd, class_buf, 128)
        class_name = class_buf.value or ""

        rect = RECTStruct()
        user32.GetWindowRect(hwnd, ctypes.byref(rect))

        pid = ctypes.c_ulong()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))

        windows.append({
            "hwnd": hwnd,
            "title": title,
            "class_name": class_name,
            "rect": (rect.left, rect.top, rect.right, rect.bottom),
            "pid": pid.value,
        })
        return True

    proc = EnumWindowsProc(callback)
    user32.EnumWindows(proc, 0)
    return windows


def find_window_by_title(title_contains: str) -> list[dict]:
    """Find windows whose title contains the given string."""
    all_wins = enum_windows()
    return [w for w in all_wins if title_contains.lower() in w["title"].lower()]


def find_window_by_class(class_contains: str) -> list[dict]:
    """Find windows whose class name contains the given string."""
    all_wins = enum_windows()
    return [w for w in all_wins if class_contains.lower() in w["class_name"].lower()]


def wait_for_window(title_contains: str, timeout: float = 10.0) -> dict | None:
    """Wait for a window with the given title to appear."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        wins = find_window_by_title(title_contains)
        if wins:
            return wins[0]
        time.sleep(0.3)
    return None


def close_window(hwnd) -> bool:
    """Send WM_CLOSE to a window."""
    return user32.PostMessageW(hwnd, WM_CLOSE, 0, 0) != 0


def get_window_rect(hwnd) -> tuple[int, int, int, int]:
    """Get window rectangle as (left, top, right, bottom)."""
    rect = RECTStruct()
    user32.GetWindowRect(hwnd, ctypes.byref(rect))
    return (rect.left, rect.top, rect.right, rect.bottom)


def is_window_visible(hwnd) -> bool:
    return user32.IsWindowVisible(hwnd) != 0


def bring_window_to_front(hwnd):
    """Bring a window to the foreground."""
    user32.ShowWindow(hwnd, SW_RESTORE)
    user32.SetForegroundWindow(hwnd)


def find_betterpanely_windows() -> list[dict]:
    """Find all BetterPanely-related windows (main, panels, tray)."""
    all_wins = enum_windows()
    bp_wins = []
    for w in all_wins:
        if "BetterPanely" in w["title"] or "BetterPanelyContainer" in w["class_name"]:
            bp_wins.append(w)
        elif "Calculator" in w["title"] and w["class_name"] != "ApplicationFrameWindow":
            bp_wins.append(w)
        elif "Notes" in w["title"] or "Timer" in w["title"] or "Weather" in w["title"]:
            bp_wins.append(w)
        elif "Settings" in w["title"]:
            bp_wins.append(w)
    return bp_wins


def get_exe_path() -> Path:
    """Get path to the built executable."""
    project_root = Path(__file__).parent.parent.parent
    exe_path = project_root / "src-tauri" / "target" / "release" / "better-panely.exe"
    if not exe_path.exists():
        exe_path = project_root / "src-tauri" / "target" / "debug" / "better-panely.exe"
    return exe_path

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Panel, WindowInfo, ToolDefinition, PanelType } from "./types";

// ─── Panel CRUD ───────────────────────────────────────────────

export async function createPanel(
  title: string,
  panelType: PanelType,
  width?: number,
  height?: number
): Promise<Panel> {
  return invoke("create_panel", { title, panelType, width, height });
}

export async function destroyPanel(panelId: string): Promise<void> {
  return invoke("destroy_panel", { panelId });
}

export async function listPanels(): Promise<Panel[]> {
  return invoke("list_panels");
}

export async function getPanel(panelId: string): Promise<Panel> {
  return invoke("get_panel", { panelId });
}

export async function movePanel(
  panelId: string,
  x: number,
  y: number
): Promise<void> {
  return invoke("move_panel", { panelId, x, y });
}

export async function resizePanel(
  panelId: string,
  width: number,
  height: number
): Promise<void> {
  return invoke("resize_panel", { panelId, width, height });
}

export async function setPanelAlwaysOnTop(
  panelId: string,
  alwaysOnTop: boolean
): Promise<void> {
  return invoke("set_panel_always_on_top", { panelId, alwaysOnTop });
}

export async function setPanelOpacity(
  panelId: string,
  opacity: number
): Promise<void> {
  return invoke("set_panel_opacity", { panelId, opacity });
}

export async function setPanelClickThrough(
  panelId: string,
  clickThrough: boolean
): Promise<void> {
  return invoke("set_panel_click_through", { panelId, clickThrough });
}

// ─── Window Enumeration ───────────────────────────────────────

export async function enumerateWindows(): Promise<WindowInfo[]> {
  return invoke("enumerate_windows");
}

export async function refreshWindowList(): Promise<WindowInfo[]> {
  return invoke("refresh_window_list");
}

// ─── Embedding ────────────────────────────────────────────────

export async function embedWindow(
  panelId: string,
  sourceHwnd: number
): Promise<void> {
  return invoke("embed_window", { panelId, sourceHwnd });
}

export async function releaseWindow(panelId: string): Promise<void> {
  return invoke("release_window", { panelId });
}

// ─── Drag Capture ─────────────────────────────────────────────

export async function startDragCapture(): Promise<void> {
  return invoke("start_drag_capture");
}

export async function stopDragCapture(): Promise<void> {
  return invoke("stop_drag_capture");
}

export async function captureWindowViaHotkey(): Promise<void> {
  return invoke("capture_window_via_hotkey");
}

// ─── Built-in Tools ───────────────────────────────────────────

export async function launchTool(
  toolId: string,
  x?: number,
  y?: number
): Promise<Panel> {
  return invoke("launch_tool", { toolId, x, y });
}

export async function listTools(): Promise<ToolDefinition[]> {
  return invoke("list_tools");
}

// ─── State Persistence ───────────────────────────────────────

export async function saveState(): Promise<void> {
  return invoke("save_state");
}

export async function loadState(): Promise<void> {
  return invoke("load_state");
}

// ─── Events ──────────────────────────────────────────────────

export function onPanelCreated(
  callback: (panel: Panel) => void
): Promise<UnlistenFn> {
  return listen<Panel>("panel:created", (event) => callback(event.payload));
}

export function onPanelDestroyed(
  callback: (panelId: string) => void
): Promise<UnlistenFn> {
  return listen<string>("panel:destroyed", (event) => callback(event.payload));
}

export function onDragEnter(
  callback: (panelId: string) => void
): Promise<UnlistenFn> {
  return listen<string>("panel:drag-enter", (event) => callback(event.payload));
}

export function onDragLeave(
  callback: (panelId: string) => void
): Promise<UnlistenFn> {
  return listen<string>("panel:drag-leave", (event) => callback(event.payload));
}

export function onWindowEmbedded(
  callback: (data: { panelId: string; hwnd: number }) => void
): Promise<UnlistenFn> {
  return listen<{ panelId: string; hwnd: number }>(
    "panel:embedded",
    (event) => callback(event.payload)
  );
}

export function onWindowReleased(
  callback: (data: { panelId: string; hwnd: number }) => void
): Promise<UnlistenFn> {
  return listen<{ panelId: string; hwnd: number }>(
    "panel:released",
    (event) => callback(event.payload)
  );
}

export function onEmbedError(
  callback: (error: string) => void
): Promise<UnlistenFn> {
  return listen<string>("panel:embed-error", (event) => callback(event.payload));
}

// ─── Settings / Language ──────────────────────────────────────

export async function getSettings(): Promise<import("./types").AppSettings> {
  return invoke("get_settings");
}

export async function getLanguage(): Promise<string> {
  return invoke("get_language");
}

export async function setLanguage(lang: string): Promise<string> {
  return invoke("set_language", { lang });
}

export function onLanguageChanged(
  callback: (lang: string) => void
): Promise<UnlistenFn> {
  return listen<string>("language-changed", (event) => callback(event.payload));
}

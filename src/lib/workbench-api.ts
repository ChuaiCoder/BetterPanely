import { invoke } from "@tauri-apps/api/core";
import type { PanelState, WindowInfo } from "./types";

type PanelType = PanelState["type"];

const DEFAULT_PANEL_POSITION = 100;
const DEFAULT_PANEL_TITLE = "Untitled";
const DEFAULT_PANEL_Z_INDEX = 1;
const MIN_PANEL_WIDTH = 80;
const MIN_PANEL_HEIGHT = 64;
const VALID_TOOL_IDS = new Set(["calculator", "notes", "timer", "weather"]);

type WindowInfoRaw = {
  hwnd: number;
  title: string;
  exe_path: string;
  class_name: string;
  is_compatible: boolean;
  incompatibility_reason: string | null;
  pid: number;
  rect: { left: number; top: number; right: number; bottom: number };
};

export interface ThumbnailRegistration {
  panelId: string;
  sourceWidth: number;
  sourceHeight: number;
}

export interface ThumbnailRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

function mapWindowInfo(raw: WindowInfoRaw): WindowInfo {
  return {
    hwnd: raw.hwnd,
    title: raw.title,
    exePath: raw.exe_path,
    className: raw.class_name,
    isCompatible: raw.is_compatible,
    incompatibilityReason: raw.incompatibility_reason ?? undefined,
    pid: raw.pid,
    rect: raw.rect,
  };
}

/**
 * 枚举系统中可见的窗口列表
 * @returns 窗口信息列表
 */
export async function enumerateWindows(): Promise<WindowInfo[]> {
  const windows = await invoke<WindowInfoRaw[]>("wb_enumerate_windows");
  return windows.map(mapWindowInfo);
}

/**
 * 获取当前焦点下可捕获的窗口。
 */
export async function captureFocusedWindow(): Promise<WindowInfo | null> {
  const windowInfo = await invoke<WindowInfoRaw | null>("wb_capture_focused_window");
  return windowInfo ? mapWindowInfo(windowInfo) : null;
}

/**
 * 为指定窗口注册缩略图
 * @param sourceHwnd 源窗口的 HWND
 * @returns 面板 ID
 */
export async function addThumbnail(sourceHwnd: number): Promise<ThumbnailRegistration> {
  return invoke<ThumbnailRegistration>("wb_add_thumbnail", { sourceHwnd });
}

/**
 * 更新缩略图的目标矩形
 * @param panelId 面板 ID
 * @param x 目标 X 坐标
 * @param y 目标 Y 坐标
 * @param width 目标宽度
 * @param height 目标高度
 */
export async function updateThumbnailRect(
  panelId: string,
  x: number,
  y: number,
  width: number,
  height: number
): Promise<void> {
  return invoke("wb_update_thumbnail_rect", { panelId, x, y, width, height });
}

export async function updateThumbnailLayout(
  panelId: string,
  fullRect: ThumbnailRect,
  visibleRects: ThumbnailRect[]
): Promise<void> {
  return invoke("wb_update_thumbnail_layout", { panelId, fullRect, visibleRects });
}

export async function syncThumbnailStack(panelIds: string[]): Promise<void> {
  return invoke("wb_sync_thumbnail_stack", { panelIds });
}

/**
 * 移除面板
 * @param panelId 面板 ID
 */
export async function removePanel(panelId: string): Promise<void> {
  return invoke("wb_remove_panel", { panelId });
}

/**
 * 聚焦源窗口
 * @param sourceHwnd 源窗口的 HWND
 */
export async function focusSource(sourceHwnd: number): Promise<void> {
  return invoke("wb_focus_source", { sourceHwnd });
}

/**
 * 获取工作台窗口的 HWND
 * @returns 工作台窗口的 HWND
 */
export async function getWorkbenchHwnd(): Promise<number> {
  return invoke("wb_get_workbench_hwnd");
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function stringField(record: Record<string, unknown>, key: string): string | null {
  const value = record[key];
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function finiteNumberField(record: Record<string, unknown>, key: string): number | null {
  const value = record[key];
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function panelDimensionField(
  record: Record<string, unknown>,
  key: string,
  min: number
): number | null {
  const value = finiteNumberField(record, key);
  return value !== null && value >= min ? value : null;
}

function positiveNumberField(record: Record<string, unknown>, key: string): number | null {
  const value = finiteNumberField(record, key);
  return value !== null && value > 0 ? value : null;
}

function zIndexField(record: Record<string, unknown>): number {
  const value = finiteNumberField(record, "z_index");
  return value === null || value < 0 ? DEFAULT_PANEL_Z_INDEX : Math.trunc(value);
}

function isPanelType(value: unknown): value is PanelType {
  return value === "thumbnail" || value === "tool";
}

function mapSavedPanel(raw: unknown): PanelState | null {
  if (!isRecord(raw)) return null;

  const id = stringField(raw, "id");
  const panelType = stringField(raw, "panel_type");
  const width = panelDimensionField(raw, "width", MIN_PANEL_WIDTH);
  const height = panelDimensionField(raw, "height", MIN_PANEL_HEIGHT);

  if (!id || !isPanelType(panelType) || width === null || height === null) {
    return null;
  }

  const basePanel = {
    id,
    type: panelType,
    title: stringField(raw, "title") ?? DEFAULT_PANEL_TITLE,
    x: finiteNumberField(raw, "x") ?? DEFAULT_PANEL_POSITION,
    y: finiteNumberField(raw, "y") ?? DEFAULT_PANEL_POSITION,
    width,
    height,
    zIndex: zIndexField(raw),
    visible: true,
  };

  if (panelType === "thumbnail") {
    const sourceHwnd = positiveNumberField(raw, "source_hwnd");
    return sourceHwnd === null ? null : { ...basePanel, sourceHwnd };
  }

  const toolId = stringField(raw, "tool_id");
  return toolId && VALID_TOOL_IDS.has(toolId) ? { ...basePanel, toolId } : null;
}

/**
 * Open a built-in tool as a standalone utility window.
 * @param toolId Built-in tool ID
 */
export async function openToolWindow(toolId: string): Promise<void> {
  return invoke("wb_open_tool_window", { toolId });
}

/**
 * 保存当前布局
 * @param panels 面板状态列表
 */
export async function saveLayout(panels: PanelState[]): Promise<void> {
  const savedPanels = panels.map((p) => ({
    id: p.id,
    panel_type: p.type,
    source_hwnd: p.sourceHwnd,
    tool_id: p.toolId,
    title: p.title,
    x: p.x,
    y: p.y,
    width: p.width,
    height: p.height,
    z_index: p.zIndex,
  }));
  return invoke("wb_save_layout", { panels: savedPanels });
}

/**
 * 加载保存的布局
 * @returns 保存的面板状态列表
 */
export async function loadLayout(): Promise<PanelState[]> {
  const result = await invoke<unknown>("wb_load_layout");
  if (!Array.isArray(result)) {
    console.warn("Ignoring invalid workbench layout payload:", result);
    return [];
  }

  return result
    .map(mapSavedPanel)
    .filter((panel): panel is PanelState => panel !== null);
}

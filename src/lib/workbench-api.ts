import { invoke } from "@tauri-apps/api/core";
import { listen, type EventCallback } from "@tauri-apps/api/event";
import type { PanelState, WindowInfo } from "./types";

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
 * 获取当前鼠标下可捕获的窗口。
 */
export async function captureWindowUnderCursor(): Promise<WindowInfo | null> {
  const windowInfo = await invoke<WindowInfoRaw | null>("wb_capture_window_under_cursor");
  return windowInfo ? mapWindowInfo(windowInfo) : null;
}

/**
 * 为指定窗口注册缩略图
 * @param sourceHwnd 源窗口的 HWND
 * @returns 面板 ID
 */
export async function addThumbnail(sourceHwnd: number): Promise<string> {
  return invoke("wb_add_thumbnail", { sourceHwnd });
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
  const result = await invoke<Array<{
    id: string;
    panel_type: string;
    source_hwnd: number | null;
    tool_id: string | null;
    title: string;
    x: number;
    y: number;
    width: number;
    height: number;
    z_index: number;
  }>>("wb_load_layout");

  return result.map((p) => ({
    id: p.id,
    type: p.panel_type as "thumbnail" | "tool",
    sourceHwnd: p.source_hwnd ?? undefined,
    toolId: p.tool_id ?? undefined,
    title: p.title,
    x: p.x,
    y: p.y,
    width: p.width,
    height: p.height,
    zIndex: p.z_index,
    visible: true,
  }));
}

/**
 * 监听源窗口关闭事件
 * @param callback 回调函数
 * @returns 取消监听函数
 */
export async function onSourceClosed(
  callback: EventCallback<{ sourceHwnd: number }>
): Promise<() => void> {
  return listen("thumb:source-closed", callback);
}

/**
 * 监听拖拽进入工作台事件
 * @param callback 回调函数
 * @returns 取消监听函数
 */
export async function onDragEnteredWorkbench(
  callback: EventCallback<{ sourceHwnd: number; x: number; y: number }>
): Promise<() => void> {
  return listen("drag:entered-workbench", callback);
}

/** Panel type discriminator */
export type PanelType =
  | { type: "tool"; toolId: string }
  | { type: "embedded"; embedInfo: EmbedInfo | null };

/** Information about an embedded window */
export interface EmbedInfo {
  sourceHwnd: number;
  sourceTitle: string;
  sourceExe: string;
  originalStyle: number;
  originalParent: number;
  threadId: number;
}

/** A panel managed by the system */
export interface Panel {
  id: string;
  title: string;
  panelType: PanelType;
  x: number;
  y: number;
  width: number;
  height: number;
  alwaysOnTop: boolean;
  opacity: number;
  clickThrough: boolean;
}

/** Workbench panel state */
export interface PanelState {
  id: string;
  type: "thumbnail" | "tool";
  sourceHwnd?: number;
  toolId?: string;
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
  zIndex: number;
  visible: boolean;
}

/** Snap guide for magnetic alignment */
export interface SnapGuide {
  type: "vertical" | "horizontal";
  position: number;
  targetPanelId: string;
}

/** An enumerated window from the system */
export interface WindowInfo {
  hwnd: number;
  title: string;
  exePath: string;
  className: string;
  isCompatible: boolean;
  incompatibilityReason?: string;
  pid: number;
  rect: { left: number; top: number; right: number; bottom: number };
}

/** Available built-in tool definition */
export interface ToolDefinition {
  id: string;
  name: string;
  description: string;
  icon: string;
  defaultWidth: number;
  defaultHeight: number;
  url: string;
}

/** Event emitted when a window is being dragged */
export interface DragEvent {
  hwnd: number;
  mouseX: number;
  mouseY: number;
}

/** State persisted to disk */
export interface AppState {
  panels: Panel[];
  settings: AppSettings;
}

export interface AppSettings {
  launchOnStartup: boolean;
  minimizeToTray: boolean;
  theme: "light" | "dark" | "system";
  captureHotkey: string;
  language: "en" | "zh";
}
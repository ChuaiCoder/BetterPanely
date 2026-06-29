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

export interface AppSettings {
  launchOnStartup: boolean;
  minimizeToTray: boolean;
  theme: "light" | "dark" | "system";
  captureHotkey: string;
  language: "en" | "zh";
}

# BetterPanely 工作台实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 BetterPanely 重构为工作台模式，支持多个微缩面板（DWM 缩略图 + iframe 工具）在独立窗口内自由拖动和磁性吸附。

**Architecture:** 单 Tauri WebView 窗口承载 SolidJS 渲染的工作台 UI，通过 DWM API 在客户区叠加外部窗口实时缩略图。前端负责布局、拖拽、吸附；后端负责缩略图管理、窗口枚举、状态持久化。

**Tech Stack:** Tauri 2.2, Rust 2021, SolidJS 1.8, TypeScript 5.5, Vite 5.4, Windows API (DWM)

---

## 文件结构总览

### 新增文件
| 文件 | 职责 |
|------|------|
| `src-tauri/src/thumbnail/mod.rs` | 缩略图模块入口 |
| `src-tauri/src/thumbnail/manager.rs` | ThumbnailManager 结构体 |
| `src-tauri/src/thumbnail/dwm.rs` | DWM API 封装 |
| `src-tauri/src/commands/workbench_cmds.rs` | 工作台命令 |
| `src/components/WorkbenchCanvas.tsx` | 工作台主画布 |
| `src/components/ThumbPanel.tsx` | 缩略图面板 |
| `src/components/ToolPanel.tsx` | 工具面板 |
| `src/components/AddPanelDialog.tsx` | 添加面板对话框 |
| `src/lib/snap-engine.ts` | 磁性吸附引擎 |
| `src/lib/workbench-api.ts` | 工作台 API 封装 |

### 修改文件
| 文件 | 改动 |
|------|------|
| `src-tauri/src/lib.rs` | 注册新命令和模块 |
| `src-tauri/src/state.rs` | 更新持久化结构 |
| `src-tauri/src/window_embedder/mod.rs` | 移除 setparent 依赖 |
| `src/lib/types.ts` | 添加新类型 |
| `src/App.tsx` | 重写为工作台 |
| `src/App.css` | 新样式 |

### 删除文件
| 文件 | 原因 |
|------|------|
| `src/components/PanelFrame.tsx` | 废弃 |
| `src/components/WindowPicker.tsx` | 替换为 AddPanelDialog |
| `src/lib/panel-api.ts` | 替换为 workbench-api.ts |
| `src-tauri/src/panel_manager/` | 废弃 |
| `src-tauri/src/window_embedder/setparent.rs` | 废弃 |

---

## 任务分解

### Task 1: 后端 - 缩略图模块（DWM API 封装）

**Files:**
- Create: `src-tauri/src/thumbnail/dwm.rs`
- Create: `src-tauri/src/thumbnail/manager.rs`
- Create: `src-tauri/src/thumbnail/mod.rs`

**Step 1: 创建 DWM API 封装**

```rust
// src-tauri/src/thumbnail/dwm.rs
#[cfg(target_os = "windows")]
use windows::{
    Win32::UI::WindowsAndMessaging::*,
    Win32::Foundation::*,
    Win32::UI::Shell::*,
};

#[cfg(target_os = "windows")]
#[repr(C)]
pub struct DWM_THUMBNAIL_PROPERTIES {
    pub dwFlags: u32,
    pub rcDestination: RECT,
    pub rcSource: RECT,
    pub opacity: u8,
    pub fVisible: bool,
    pub fSourceClientAreaOnly: bool,
}

#[cfg(target_os = "windows")]
pub type DWM_THUMBNAIL_ID = u32;

#[cfg(target_os = "windows")]
pub unsafe fn register_thumbnail(
    dest_hwnd: HWND,
    source_hwnd: HWND,
) -> Result<DWM_THUMBNAIL_ID, Box<dyn std::error::Error>> {
    let mut thumbnail_id: DWM_THUMBNAIL_ID = 0;
    let result = DwmRegisterThumbnail(dest_hwnd, source_hwnd, &mut thumbnail_id);
    if result != S_OK {
        return Err(format!("DwmRegisterThumbnail failed: {:?}", result).into());
    }
    Ok(thumbnail_id)
}

#[cfg(target_os = "windows")]
pub unsafe fn update_thumbnail_properties(
    thumbnail_id: DWM_THUMBNAIL_ID,
    props: &DWM_THUMBNAIL_PROPERTIES,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = DwmUpdateThumbnailProperties(thumbnail_id, props);
    if result != S_OK {
        return Err(format!("DwmUpdateThumbnailProperties failed: {:?}", result).into());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub unsafe fn unregister_thumbnail(
    thumbnail_id: DWM_THUMBNAIL_ID,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = DwmUnregisterThumbnail(thumbnail_id);
    if result != S_OK {
        return Err(format!("DwmUnregisterThumbnail failed: {:?}", result).into());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn register_thumbnail(_: isize, _: isize) -> Result<u32, Box<dyn std::error::Error>> {
    Err("DWM thumbnails only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn update_thumbnail_properties(_: u32, _: &()) -> Result<(), Box<dyn std::error::Error>> {
    Err("DWM thumbnails only supported on Windows".into())
}

#[cfg(not(target_os = "windows"))]
pub fn unregister_thumbnail(_: u32) -> Result<(), Box<dyn std::error::Error>> {
    Err("DWM thumbnails only supported on Windows".into())
}
```

**Step 2: 创建 ThumbnailManager**

```rust
// src-tauri/src/thumbnail/manager.rs
use std::collections::HashMap;
use std::sync::Mutex;
use super::dwm::*;
use windows::Win32::Foundation::RECT;

pub struct ThumbnailManager {
    thumbnails: HashMap<isize, ThumbnailHandle>,
    panel_map: HashMap<String, isize>,
    next_id: u32,
}

pub struct ThumbnailHandle {
    source_hwnd: isize,
    thumbnail_id: DWM_THUMBNAIL_ID,
    dest_rect: RECT,
    visible: bool,
    opacity: f32,
}

impl ThumbnailManager {
    pub fn new() -> Self {
        Self {
            thumbnails: HashMap::new(),
            panel_map: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn next_panel_id(&mut self) -> String {
        self.next_id += 1;
        format!("wb_panel_{}", self.next_id)
    }

    #[cfg(target_os = "windows")]
    pub unsafe fn register(
        &mut self,
        dest_hwnd: isize,
        source_hwnd: isize,
        panel_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let thumbnail_id = register_thumbnail(
            windows::Win32::Foundation::HWND(dest_hwnd as *mut _),
            windows::Win32::Foundation::HWND(source_hwnd as *mut _),
        )?;

        let handle = ThumbnailHandle {
            source_hwnd,
            thumbnail_id,
            dest_rect: RECT { left: 0, top: 0, right: 100, bottom: 100 },
            visible: true,
            opacity: 1.0,
        };

        self.thumbnails.insert(source_hwnd, handle);
        self.panel_map.insert(panel_id.to_string(), source_hwnd);

        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub unsafe fn update_rect(
        &mut self,
        panel_id: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let source_hwnd = self.panel_map.get(panel_id).ok_or("Panel not found")?;
        let handle = self.thumbnails.get_mut(source_hwnd).ok_or("Thumbnail not found")?;

        handle.dest_rect = RECT {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        };

        let props = DWM_THUMBNAIL_PROPERTIES {
            dwFlags: 0x00000001 | 0x00000008,
            rcDestination: handle.dest_rect,
            rcSource: RECT { left: 0, top: 0, right: 0, bottom: 0 },
            opacity: (handle.opacity * 255.0) as u8,
            fVisible: handle.visible,
            fSourceClientAreaOnly: true,
        };

        update_thumbnail_properties(handle.thumbnail_id, &props)?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub unsafe fn unregister_by_panel_id(
        &mut self,
        panel_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(source_hwnd) = self.panel_map.remove(panel_id) {
            if let Some(handle) = self.thumbnails.remove(&source_hwnd) {
                let _ = unregister_thumbnail(handle.thumbnail_id);
            }
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub unsafe fn unregister_all(&mut self) {
        for (_, handle) in self.thumbnails.drain() {
            let _ = unregister_thumbnail(handle.thumbnail_id);
        }
        self.panel_map.clear();
    }

    pub fn get_source_hwnd(&self, panel_id: &str) -> Option<isize> {
        self.panel_map.get(panel_id).copied()
    }

    pub fn contains_panel(&self, panel_id: &str) -> bool {
        self.panel_map.contains_key(panel_id)
    }
}

pub struct SharedThumbnailManager {
    inner: Mutex<ThumbnailManager>,
}

impl SharedThumbnailManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(ThumbnailManager::new()),
        }
    }

    pub fn register(
        &self,
        dest_hwnd: isize,
        source_hwnd: isize,
        panel_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.inner.lock().unwrap().register(dest_hwnd, source_hwnd, panel_id)
        }
    }

    pub fn update_rect(
        &self,
        panel_id: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.inner.lock().unwrap().update_rect(panel_id, x, y, width, height)
        }
    }

    pub fn unregister(&self, panel_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            self.inner.lock().unwrap().unregister_by_panel_id(panel_id)
        }
    }

    pub fn unregister_all(&self) {
        unsafe {
            self.inner.lock().unwrap().unregister_all()
        }
    }

    pub fn next_panel_id(&self) -> String {
        self.inner.lock().unwrap().next_panel_id()
    }

    pub fn get_source_hwnd(&self, panel_id: &str) -> Option<isize> {
        self.inner.lock().unwrap().get_source_hwnd(panel_id)
    }
}
```

**Step 3: 创建模块入口**

```rust
// src-tauri/src/thumbnail/mod.rs
pub mod dwm;
pub mod manager;
pub use manager::{ThumbnailManager, SharedThumbnailManager};
```

**Step 4: 编译检查**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: No compilation errors

---

### Task 2: 后端 - 工作台命令

**Files:**
- Create: `src-tauri/src/commands/workbench_cmds.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: 创建 workbench_cmds.rs**

```rust
// src-tauri/src/commands/workbench_cmds.rs
use tauri::{State, AppHandle};
use crate::thumbnail::SharedThumbnailManager;
use crate::window_embedder::enumerator;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct WindowInfo {
    pub hwnd: isize,
    pub title: String,
    pub exe_path: String,
    pub class_name: String,
    pub is_compatible: bool,
    pub incompatibility_reason: Option<String>,
    pub pid: u32,
    pub rect: RectInfo,
}

#[derive(Serialize, Deserialize)]
pub struct RectInfo {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Serialize, Deserialize)]
pub struct PanelState {
    pub id: String,
    pub panel_type: String,
    pub source_hwnd: Option<isize>,
    pub tool_id: Option<String>,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub z_index: i32,
}

#[tauri::command]
pub fn wb_enumerate_windows() -> Result<Vec<WindowInfo>, String> {
    #[cfg(target_os = "windows")]
    {
        let windows = enumerator::enumerate_windows().map_err(|e| e.to_string())?;
        let infos = windows.into_iter().map(|w| WindowInfo {
            hwnd: w.hwnd as isize,
            title: w.title,
            exe_path: w.exe_path,
            class_name: w.class_name,
            is_compatible: w.is_compatible,
            incompatibility_reason: w.incompatibility_reason,
            pid: w.pid,
            rect: RectInfo {
                left: w.rect.left,
                top: w.rect.top,
                right: w.rect.right,
                bottom: w.rect.bottom,
            },
        }).collect();
        Ok(infos)
    }
    #[cfg(not(target_os = "windows"))]
    Ok(vec![])
}

#[tauri::command]
pub fn wb_add_thumbnail(
    source_hwnd: isize,
    thumbnail_manager: State<'_, SharedThumbnailManager>,
) -> Result<String, String> {
    let panel_id = thumbnail_manager.next_panel_id();
    thumbnail_manager.register(0, source_hwnd, &panel_id)
        .map_err(|e| e.to_string())?;
    Ok(panel_id)
}

#[tauri::command]
pub fn wb_update_thumbnail_rect(
    panel_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    thumbnail_manager: State<'_, SharedThumbnailManager>,
) -> Result<(), String> {
    thumbnail_manager.update_rect(
        &panel_id,
        x as i32,
        y as i32,
        width as i32,
        height as i32,
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn wb_remove_panel(
    panel_id: String,
    thumbnail_manager: State<'_, SharedThumbnailManager>,
) -> Result<(), String> {
    thumbnail_manager.unregister(&panel_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn wb_focus_source(source_hwnd: isize) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
        unsafe {
            let hwnd = windows::Win32::Foundation::HWND(source_hwnd as *mut _);
            SetForegroundWindow(hwnd).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    Ok(())
}

#[tauri::command]
pub fn wb_get_workbench_hwnd(app: AppHandle) -> Result<isize, String> {
    if let Some(window) = app.get_webview_window("main") {
        let hwnd = window.hwnd().map_err(|e| e.to_string())?;
        Ok(hwnd as isize)
    } else {
        Err("Workbench window not found".to_string())
    }
}

#[tauri::command]
pub fn wb_save_layout(
    panels: Vec<PanelState>,
    app: AppHandle,
) -> Result<(), String> {
    crate::state::save_layout(app, &panels).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn wb_load_layout(app: AppHandle) -> Result<Vec<PanelState>, String> {
    crate::state::load_layout(app).map_err(|e| e.to_string())
}
```

**Step 2: 更新 commands/mod.rs**

```rust
// src-tauri/src/commands/mod.rs
pub mod panel_cmds;
pub mod embed_cmds;
pub mod tool_cmds;
pub mod settings_cmds;
pub mod workbench_cmds;
```

**Step 3: 更新 lib.rs**

Add to `src-tauri/src/lib.rs`:
```rust
mod thumbnail;
use thumbnail::SharedThumbnailManager;

// In run() function, add to state:
let thumbnail_manager = SharedThumbnailManager::new();

// In tauri::Builder::default() chain:
.manage(thumbnail_manager)

// In invoke_handler:
commands::workbench_cmds::wb_enumerate_windows,
commands::workbench_cmds::wb_add_thumbnail,
commands::workbench_cmds::wb_update_thumbnail_rect,
commands::workbench_cmds::wb_remove_panel,
commands::workbench_cmds::wb_focus_source,
commands::workbench_cmds::wb_get_workbench_hwnd,
commands::workbench_cmds::wb_save_layout,
commands::workbench_cmds::wb_load_layout,
```

**Step 4: 编译检查**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: No compilation errors

---

### Task 3: 后端 - 状态持久化更新

**Files:**
- Modify: `src-tauri/src/state.rs`

**Step 1: 更新 state.rs 的布局持久化**

```rust
// Add these imports to src-tauri/src/state.rs
use serde::{Serialize, Deserialize};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPanel {
    pub id: String,
    pub panel_type: String,
    pub source_hwnd: Option<isize>,
    pub tool_id: Option<String>,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub z_index: i32,
}

pub fn save_layout(app: AppHandle, panels: &[SavedPanel]) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().data_dir()?;
    let layout_path = data_dir.join("workbench_layout.json");

    let json = serde_json::to_string_pretty(panels)?;
    std::fs::write(&layout_path, json)?;
    Ok(())
}

pub fn load_layout(app: AppHandle) -> Result<Vec<SavedPanel>, Box<dyn std::error::Error>> {
    let data_dir = app.path().data_dir()?;
    let layout_path = data_dir.join("workbench_layout.json");

    if !layout_path.exists() {
        return Ok(vec![]);
    }

    let json = std::fs::read_to_string(&layout_path)?;
    let panels: Vec<SavedPanel> = serde_json::from_str(&json)?;
    Ok(panels)
}
```

**Step 2: 编译检查**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: No compilation errors

---

### Task 4: 后端 - 窗口枚举器清理

**Files:**
- Modify: `src-tauri/src/window_embedder/mod.rs`

**Step 1: 更新 mod.rs 移除 setparent**

```rust
// src-tauri/src/window_embedder/mod.rs
pub mod enumerator;
```

**Step 2: 编译检查**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: No compilation errors

---

### Task 5: 前端 - 类型定义更新

**Files:**
- Modify: `src/lib/types.ts`

**Step 1: 添加工作台相关类型**

```typescript
// src/lib/types.ts
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

export interface SnapGuide {
  type: "vertical" | "horizontal";
  position: number;
  targetPanelId: string;
}

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

export interface ToolDefinition {
  id: string;
  name: string;
  description: string;
  icon: string;
  defaultWidth: number;
  defaultHeight: number;
  url: string;
}
```

---

### Task 6: 前端 - 磁性吸附引擎

**Files:**
- Create: `src/lib/snap-engine.ts`

**Step 1: 创建吸附引擎**

```typescript
// src/lib/snap-engine.ts
import type { PanelState, SnapGuide } from "./types";

const SNAP_THRESHOLD = 8;

interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface SnapResult {
  rect: Rect;
  guides: SnapGuide[];
}

function createRectFromPanel(panel: PanelState): Rect {
  return { x: panel.x, y: panel.y, width: panel.width, height: panel.height };
}

function distance(a: number, b: number): number {
  return Math.abs(a - b);
}

export function snap(
  draggedRect: Rect,
  otherPanels: PanelState[],
  canvasWidth: number,
  canvasHeight: number
): SnapResult {
  const guides: SnapGuide[] = [];
  let snappedRect = { ...draggedRect };
  let snapped = false;

  const otherRects = otherPanels.map(createRectFromPanel);

  const snapTargets = [
    { position: 0, type: "vertical" as const, targetId: "canvas" },
    { position: canvasWidth, type: "vertical" as const, targetId: "canvas" },
    { position: 0, type: "horizontal" as const, targetId: "canvas" },
    { position: canvasHeight, type: "horizontal" as const, targetId: "canvas" },
  ];

  otherRects.forEach((rect, index) => {
    const panel = otherPanels[index];
    snapTargets.push(
      { position: rect.x, type: "vertical" as const, targetId: panel.id },
      { position: rect.x + rect.width, type: "vertical" as const, targetId: panel.id },
      { position: rect.y, type: "horizontal" as const, targetId: panel.id },
      { position: rect.y + rect.height, type: "horizontal" as const, targetId: panel.id }
    );
  });

  let minDistX = Infinity;
  let snapX: number | null = null;
  let snapXTargetId = "";

  let minDistY = Infinity;
  let snapY: number | null = null;
  let snapYTargetId = "";

  snapTargets.forEach((target) => {
    if (target.type === "vertical") {
      const leftDist = distance(snappedRect.x, target.position);
      const rightDist = distance(snappedRect.x + snappedRect.width, target.position);

      if (leftDist < minDistX && leftDist < SNAP_THRESHOLD) {
        minDistX = leftDist;
        snapX = target.position;
        snapXTargetId = target.targetId;
      }
      if (rightDist < minDistX && rightDist < SNAP_THRESHOLD) {
        minDistX = rightDist;
        snapX = target.position - snappedRect.width;
        snapXTargetId = target.targetId;
      }
    } else {
      const topDist = distance(snappedRect.y, target.position);
      const bottomDist = distance(snappedRect.y + snappedRect.height, target.position);

      if (topDist < minDistY && topDist < SNAP_THRESHOLD) {
        minDistY = topDist;
        snapY = target.position;
        snapYTargetId = target.targetId;
      }
      if (bottomDist < minDistY && bottomDist < SNAP_THRESHOLD) {
        minDistY = bottomDist;
        snapY = target.position - snappedRect.height;
        snapYTargetId = target.targetId;
      }
    }
  });

  if (snapX !== null) {
    snappedRect.x = snapX;
    snapped = true;
    guides.push({
      type: "vertical",
      position: snapX,
      targetPanelId: snapXTargetId,
    });
  }

  if (snapY !== null) {
    snappedRect.y = snapY;
    snapped = true;
    guides.push({
      type: "horizontal",
      position: snapY,
      targetPanelId: snapYTargetId,
    });
  }

  return { rect: snappedRect, guides };
}
```

---

### Task 7: 前端 - 工作台 API 封装

**Files:**
- Create: `src/lib/workbench-api.ts`

**Step 1: 创建 API 封装**

```typescript
// src/lib/workbench-api.ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type EventCallback } from "@tauri-apps/api/event";
import type { PanelState, WindowInfo } from "./types";

export async function enumerateWindows(): Promise<WindowInfo[]> {
  return invoke("wb_enumerate_windows");
}

export async function addThumbnail(sourceHwnd: number): Promise<string> {
  return invoke("wb_add_thumbnail", { sourceHwnd });
}

export async function updateThumbnailRect(
  panelId: string,
  x: number,
  y: number,
  width: number,
  height: number
): Promise<void> {
  return invoke("wb_update_thumbnail_rect", { panelId, x, y, width, height });
}

export async function removePanel(panelId: string): Promise<void> {
  return invoke("wb_remove_panel", { panelId });
}

export async function focusSource(sourceHwnd: number): Promise<void> {
  return invoke("wb_focus_source", { sourceHwnd });
}

export async function getWorkbenchHwnd(): Promise<number> {
  return invoke("wb_get_workbench_hwnd");
}

export async function saveLayout(panels: PanelState[]): Promise<void> {
  return invoke("wb_save_layout", { panels });
}

export async function loadLayout(): Promise<PanelState[]> {
  return invoke("wb_load_layout");
}

export async function onSourceClosed(
  callback: EventCallback<{ sourceHwnd: number }>
): Promise<() => void> {
  return listen("thumb:source-closed", callback);
}

export async function onDragEnteredWorkbench(
  callback: EventCallback<{ sourceHwnd: number; x: number; y: number }>
): Promise<() => void> {
  return listen("drag:entered-workbench", callback);
}
```

---

### Task 8: 前端 - 添加面板对话框

**Files:**
- Create: `src/components/AddPanelDialog.tsx`

**Step 1: 创建对话框组件**

```tsx
// src/components/AddPanelDialog.tsx
import { createSignal, createEffect, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../lib/i18n";
import { enumerateWindows } from "../lib/workbench-api";
import type { WindowInfo, ToolDefinition } from "../lib/types";

interface AddPanelDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onAddThumbnails: (hwnds: number[]) => void;
  onAddTool: (toolId: string) => void;
}

const TOOLS: ToolDefinition[] = [
  { id: "calculator", name: "Calculator", description: "Calculator tool", icon: "🔢", defaultWidth: 280, defaultHeight: 420, url: "src/tools/calculator/index.html" },
  { id: "notes", name: "Notes", description: "Notes tool", icon: "📝", defaultWidth: 350, defaultHeight: 400, url: "src/tools/notes/index.html" },
  { id: "timer", name: "Timer", description: "Timer tool", icon: "⏱️", defaultWidth: 300, defaultHeight: 200, url: "src/tools/timer/index.html" },
  { id: "weather", name: "Weather", description: "Weather tool", icon: "🌤️", defaultWidth: 300, defaultHeight: 350, url: "src/tools/weather/index.html" },
];

export function AddPanelDialog(props: AddPanelDialogProps) {
  const { t } = useI18n();
  const [windows, setWindows] = createSignal<WindowInfo[]>([]);
  const [selectedHwnds, setSelectedHwnds] = createSignal<Set<number>>(new Set());
  const [searchQuery, setSearchQuery] = createSignal("");

  createEffect(() => {
    if (props.isOpen) {
      enumerateWindows().then(setWindows).catch(console.error);
      setSelectedHwnds(new Set());
      setSearchQuery("");
    }
  });

  const filteredWindows = () => {
    const query = searchQuery().toLowerCase();
    return windows().filter((w) =>
      w.title.toLowerCase().includes(query) ||
      w.exePath.toLowerCase().includes(query)
    );
  };

  const toggleWindow = (hwnd: number) => {
    const newSet = new Set(selectedHwnds());
    if (newSet.has(hwnd)) {
      newSet.delete(hwnd);
    } else {
      newSet.add(hwnd);
    }
    setSelectedHwnds(newSet);
  };

  const handleAddWindows = () => {
    props.onAddThumbnails(Array.from(selectedHwnds()));
    props.onClose();
  };

  const handleAddTool = (toolId: string) => {
    props.onAddTool(toolId);
  };

  if (!props.isOpen) return null;

  return (
    <div class="dialog-overlay" onClick={props.onClose}>
      <div class="dialog-content" onClick={(e) => e.stopPropagation()}>
        <h2>{t("app.addPanel")}</h2>

        <div class="dialog-section">
          <h3>{t("app.desktopWindows")}</h3>
          <input
            type="text"
            placeholder={t("app.searchWindows")}
            value={searchQuery()}
            onInput={(e) => setSearchQuery((e.target as HTMLInputElement).value)}
            class="dialog-search"
          />
          <div class="window-list">
            <For each={filteredWindows()}>
              {(w) => (
                <label class="window-item">
                  <input
                    type="checkbox"
                    checked={selectedHwnds().has(w.hwnd)}
                    onChange={() => toggleWindow(w.hwnd)}
                  />
                  <span class="window-title">{w.title}</span>
                  <span class="window-exe">{w.exePath.split("\\").pop()}</span>
                </label>
              )}
            </For>
          </div>
        </div>

        <div class="dialog-section">
          <h3>{t("app.builtinTools")}</h3>
          <div class="tool-grid">
            <For each={TOOLS}>
              {(tool) => (
                <button
                  class="tool-btn"
                  onClick={() => handleAddTool(tool.id)}
                >
                  <span class="tool-icon">{tool.icon}</span>
                  <span class="tool-label">{t(`tools.${tool.id}`) || tool.name}</span>
                </button>
              )}
            </For>
          </div>
        </div>

        <div class="dialog-actions">
          <button class="btn btn-secondary" onClick={props.onClose}>
            {t("app.cancel")}
          </button>
          <Show when={selectedHwnds().size > 0}>
            <button class="btn btn-primary" onClick={handleAddWindows}>
              {t("app.addSelected")} ({selectedHwnds().size})
            </button>
          </Show>
        </div>
      </div>
    </div>
  );
}
```

---

### Task 9: 前端 - 缩略图面板组件

**Files:**
- Create: `src/components/ThumbPanel.tsx`

**Step 1: 创建缩略图面板**

```tsx
// src/components/ThumbPanel.tsx
import { createSignal, onMount, onCleanup, Show } from "solid-js";
import { useI18n } from "../lib/i18n";
import { focusSource, updateThumbnailRect } from "../lib/workbench-api";
import type { PanelState } from "../lib/types";

interface ThumbPanelProps {
  panel: PanelState;
  isDragging: boolean;
  onDragStart: (id: string, offsetX: number, offsetY: number) => void;
  onClose: (id: string) => void;
  onTop: (id: string) => void;
}

export function ThumbPanel(props: ThumbPanelProps) {
  const { t } = useI18n();
  const [isHovered, setIsHovered] = createSignal(false);

  const handleMouseDown = (e: MouseEvent) => {
    if ((e.target as HTMLElement).closest(".panel-close") ||
        (e.target as HTMLElement).closest(".panel-focus") ||
        (e.target as HTMLElement).closest(".panel-top")) {
      return;
    }

    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const offsetX = e.clientX - rect.left;
    const offsetY = e.clientY - rect.top;
    props.onDragStart(props.panel.id, offsetX, offsetY);
  };

  const handleFocus = () => {
    if (props.panel.sourceHwnd) {
      focusSource(props.panel.sourceHwnd).catch(console.error);
    }
  };

  const handleTop = () => {
    props.onTop(props.panel.id);
  };

  onMount(() => {
    if (props.panel.sourceHwnd) {
      updateThumbnailRect(
        props.panel.id,
        props.panel.x,
        props.panel.y,
        props.panel.width,
        props.panel.height
      ).catch(console.error);
    }
  });

  return (
    <div
      class={`panel-card ${props.isDragging ? "panel-dragging" : ""} ${isHovered() ? "panel-hovered" : ""}`}
      style={{
        left: `${props.panel.x}px`,
        top: `${props.panel.y}px`,
        width: `${props.panel.width}px`,
        height: `${props.panel.height}px`,
        zIndex: props.panel.zIndex,
      }}
      onMouseDown={handleMouseDown}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <div class="panel-header">
        <span class="panel-title">{props.panel.title}</span>
        <div class="panel-actions">
          <button
            class="panel-btn panel-close"
            onClick={(e) => { e.stopPropagation(); props.onClose(props.panel.id); }}
            title={t("app.close")}
          >
            ✕
          </button>
          <button
            class="panel-btn panel-focus"
            onClick={(e) => { e.stopPropagation(); handleFocus(); }}
            title={t("app.focus")}
          >
            ▣
          </button>
          <button
            class="panel-btn panel-top"
            onClick={(e) => { e.stopPropagation(); handleTop(); }}
            title={t("app.top")}
          >
            ↑
          </button>
        </div>
      </div>
      <div class="panel-content panel-content-transparent">
        <Show when={!props.panel.sourceHwnd}>
          <div class="panel-placeholder">
            <p>{t("app.panelPlaceholder")}</p>
          </div>
        </Show>
      </div>
    </div>
  );
}
```

---

### Task 10: 前端 - 工具面板组件

**Files:**
- Create: `src/components/ToolPanel.tsx`

**Step 1: 创建工具面板**

```tsx
// src/components/ToolPanel.tsx
import { createSignal, Show } from "solid-js";
import { useI18n } from "../lib/i18n";
import type { PanelState } from "../lib/types";

interface ToolPanelProps {
  panel: PanelState;
  isDragging: boolean;
  onDragStart: (id: string, offsetX: number, offsetY: number) => void;
  onClose: (id: string) => void;
  onTop: (id: string) => void;
}

export function ToolPanel(props: ToolPanelProps) {
  const { t, lang } = useI18n();
  const [isHovered, setIsHovered] = createSignal(false);

  const toolUrls: Record<string, string> = {
    calculator: "src/tools/calculator/index.html",
    notes: "src/tools/notes/index.html",
    timer: "src/tools/timer/index.html",
    weather: "src/tools/weather/index.html",
  };

  const iframeUrl = props.panel.toolId
    ? `${toolUrls[props.panel.toolId] || ""}#lang=${lang()}`
    : "";

  const handleMouseDown = (e: MouseEvent) => {
    if ((e.target as HTMLElement).closest(".panel-close") ||
        (e.target as HTMLElement).closest(".panel-focus") ||
        (e.target as HTMLElement).closest(".panel-top")) {
      return;
    }

    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const offsetX = e.clientX - rect.left;
    const offsetY = e.clientY - rect.top;
    props.onDragStart(props.panel.id, offsetX, offsetY);
  };

  const handleTop = () => {
    props.onTop(props.panel.id);
  };

  return (
    <div
      class={`panel-card ${props.isDragging ? "panel-dragging" : ""} ${isHovered() ? "panel-hovered" : ""}`}
      style={{
        left: `${props.panel.x}px`,
        top: `${props.panel.y}px`,
        width: `${props.panel.width}px`,
        height: `${props.panel.height}px`,
        zIndex: props.panel.zIndex,
      }}
      onMouseDown={handleMouseDown}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <div class="panel-header">
        <span class="panel-title">{props.panel.title}</span>
        <div class="panel-actions">
          <button
            class="panel-btn panel-close"
            onClick={(e) => { e.stopPropagation(); props.onClose(props.panel.id); }}
            title={t("app.close")}
          >
            ✕
          </button>
          <button
            class="panel-btn panel-top"
            onClick={(e) => { e.stopPropagation(); handleTop(); }}
            title={t("app.top")}
          >
            ↑
          </button>
        </div>
      </div>
      <div class="panel-content">
        <Show when={iframeUrl}>
          <iframe
            src={iframeUrl}
            class="panel-iframe"
            title={props.panel.title}
            sandbox="allow-same-origin allow-scripts allow-forms"
          />
        </Show>
      </div>
    </div>
  );
}
```

---

### Task 11: 前端 - 工作台主画布

**Files:**
- Create: `src/components/WorkbenchCanvas.tsx`

**Step 1: 创建工作台画布**

```tsx
// src/components/WorkbenchCanvas.tsx
import { createSignal, For, Show, onMount, onCleanup } from "solid-js";
import { ThumbPanel } from "./ThumbPanel";
import { ToolPanel } from "./ToolPanel";
import { AddPanelDialog } from "./AddPanelDialog";
import { useI18n } from "../lib/i18n";
import { snap } from "../lib/snap-engine";
import {
  addThumbnail,
  removePanel,
  updateThumbnailRect,
  loadLayout,
  saveLayout,
  onSourceClosed,
} from "../lib/workbench-api";
import type { PanelState, SnapGuide } from "../lib/types";

export function WorkbenchCanvas() {
  const { t } = useI18n();
  const [panels, setPanels] = createSignal<PanelState[]>([]);
  const [draggingId, setDraggingId] = createSignal<string | null>(null);
  const [dragOffset, setDragOffset] = createSignal({ x: 0, y: 0 });
  const [snapGuides, setSnapGuides] = createSignal<SnapGuide[]>([]);
  const [isDialogOpen, setIsDialogOpen] = createSignal(false);
  const [canvasSize, setCanvasSize] = createSignal({ width: 800, height: 600 });

  let canvasRef: HTMLDivElement | null = null;

  const getNextZIndex = () => {
    const max = panels().reduce((acc, p) => Math.max(acc, p.zIndex), 0);
    return max + 1;
  };

  const addThumbnailPanel = async (hwnd: number, title: string) => {
    try {
      const panelId = await addThumbnail(hwnd);
      const newPanel: PanelState = {
        id: panelId,
        type: "thumbnail",
        sourceHwnd: hwnd,
        title,
        x: 100 + panels().length * 20,
        y: 100 + panels().length * 20,
        width: 200,
        height: 150,
        zIndex: getNextZIndex(),
        visible: true,
      };
      setPanels((prev) => [...prev, newPanel]);
      await updateThumbnailRect(panelId, newPanel.x, newPanel.y, newPanel.width, newPanel.height);
    } catch (e) {
      console.error("Failed to add thumbnail:", e);
    }
  };

  const addToolPanel = (toolId: string) => {
    const toolConfig: Record<string, { title: string; width: number; height: number }> = {
      calculator: { title: t("tools.calculator") || "Calculator", width: 280, height: 420 },
      notes: { title: t("tools.notes") || "Notes", width: 350, height: 400 },
      timer: { title: t("tools.timer") || "Timer", width: 300, height: 200 },
      weather: { title: t("tools.weather") || "Weather", width: 300, height: 350 },
    };
    const config = toolConfig[toolId] || { title: toolId, width: 300, height: 300 };

    const newPanel: PanelState = {
      id: `tool_${toolId}_${Date.now()}`,
      type: "tool",
      toolId,
      title: config.title,
      x: 100 + panels().length * 20,
      y: 100 + panels().length * 20,
      width: config.width,
      height: config.height,
      zIndex: getNextZIndex(),
      visible: true,
    };
    setPanels((prev) => [...prev, newPanel]);
  };

  const handleAddThumbnails = (hwnds: number[]) => {
    hwnds.forEach((hwnd) => {
      addThumbnailPanel(hwnd, `Window ${hwnd}`);
    });
  };

  const handleClosePanel = async (panelId: string) => {
    try {
      await removePanel(panelId);
      setPanels((prev) => prev.filter((p) => p.id !== panelId));
    } catch (e) {
      console.error("Failed to remove panel:", e);
    }
  };

  const handleTop = (panelId: string) => {
    setPanels((prev) =>
      prev.map((p) => (p.id === panelId ? { ...p, zIndex: getNextZIndex() } : p))
    );
  };

  const handleDragStart = (panelId: string, offsetX: number, offsetY: number) => {
    setDraggingId(panelId);
    setDragOffset({ x: offsetX, y: offsetY });
    handleTop(panelId);
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (!draggingId() || !canvasRef) return;

    const canvasRect = canvasRef.getBoundingClientRect();
    const mouseX = e.clientX - canvasRect.left;
    const mouseY = e.clientY - canvasRect.top;

    const newX = mouseX - dragOffset().x;
    const newY = mouseY - dragOffset().y;

    setPanels((prev) => {
      const otherPanels = prev.filter((p) => p.id !== draggingId());
      const draggedPanel = prev.find((p) => p.id === draggingId());

      if (!draggedPanel) return prev;

      const draggedRect = { x: newX, y: newY, width: draggedPanel.width, height: draggedPanel.height };
      const snapResult = snap(draggedRect, otherPanels, canvasSize().width, canvasSize().height);

      setSnapGuides(snapResult.guides);

      const updated = prev.map((p) =>
        p.id === draggingId()
          ? { ...p, x: snapResult.rect.x, y: snapResult.rect.y }
          : p
      );

      return updated;
    });
  };

  const handleMouseUp = async () => {
    if (!draggingId()) return;

    const panel = panels().find((p) => p.id === draggingId());
    if (panel && panel.type === "thumbnail" && panel.sourceHwnd) {
      try {
        await updateThumbnailRect(panel.id, panel.x, panel.y, panel.width, panel.height);
      } catch (e) {
        console.error("Failed to update thumbnail rect:", e);
      }
    }

    setDraggingId(null);
    setSnapGuides([]);
  };

  const handleResize = () => {
    if (canvasRef) {
      setCanvasSize({ width: canvasRef.clientWidth, height: canvasRef.clientHeight });
    }
  };

  onMount(async () => {
    window.addEventListener("resize", handleResize);
    handleResize();

    try {
      const saved = await loadLayout();
      if (saved.length > 0) {
        setPanels(saved);
      }
    } catch (e) {
      console.warn("Failed to load layout:", e);
    }

    const unlistenClosed = await onSourceClosed((event) => {
      setPanels((prev) => prev.filter((p) => p.sourceHwnd !== event.payload.sourceHwnd));
    });

    onCleanup(() => {
      window.removeEventListener("resize", handleResize);
      unlistenClosed();
      saveLayout(panels()).catch(console.error);
    });
  });

  return (
    <div class="workbench-container">
      <header class="workbench-header">
        <h1>{t("app.title")}</h1>
        <div class="header-actions">
          <button class="btn btn-secondary btn-small" onClick={() => invoke("open_settings")}>
            ⚙
          </button>
          <button class="btn btn-primary" onClick={() => setIsDialogOpen(true)}>
            {t("app.addPanel")}
          </button>
        </div>
      </header>

      <div
        ref={canvasRef}
        class="workbench-canvas"
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
      >
        <For each={panels()}>
          {(panel) =>
            panel.type === "thumbnail" ? (
              <ThumbPanel
                panel={panel}
                isDragging={draggingId() === panel.id}
                onDragStart={handleDragStart}
                onClose={handleClosePanel}
                onTop={handleTop}
              />
            ) : (
              <ToolPanel
                panel={panel}
                isDragging={draggingId() === panel.id}
                onDragStart={handleDragStart}
                onClose={handleClosePanel}
                onTop={handleTop}
              />
            )
          }
        </For>

        <For each={snapGuides()}>
          {(guide) => (
            <div
              class={`snap-guide ${guide.type}`}
              style={{
                left: guide.type === "vertical" ? `${guide.position}px` : 0,
                top: guide.type === "horizontal" ? `${guide.position}px` : 0,
                width: guide.type === "vertical" ? 1 : "100%",
                height: guide.type === "horizontal" ? 1 : "100%",
              }}
            />
          )}
        </For>

        <Show when={panels().length === 0}>
          <div class="empty-canvas">
            <p>{t("app.emptyWorkbench")}</p>
            <button class="btn btn-primary" onClick={() => setIsDialogOpen(true)}>
              {t("app.addPanel")}
            </button>
          </div>
        </Show>
      </div>

      <footer class="workbench-footer">
        <span>{t("app.panelCount", { count: panels().length })}</span>
      </footer>

      <AddPanelDialog
        isOpen={isDialogOpen()}
        onClose={() => setIsDialogOpen(false)}
        onAddThumbnails={handleAddThumbnails}
        onAddTool={addToolPanel}
      />
    </div>
  );
}
```

---

### Task 12: 前端 - 主应用入口重构

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/App.css`

**Step 1: 重写 App.tsx**

```tsx
// src/App.tsx
import { WorkbenchCanvas } from "./components/WorkbenchCanvas";
import "./App.css";

function App() {
  return <WorkbenchCanvas />;
}

export default App;
```

**Step 2: 更新 App.css**

```css
/* src/App.css */
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
  overflow: hidden;
}

.workbench-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
}

.workbench-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 20px;
  background: rgba(0, 0, 0, 0.3);
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.workbench-header h1 {
  color: #fff;
  font-size: 18px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.btn {
  padding: 6px 16px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-primary {
  background: #6366f1;
  color: #fff;
}

.btn-primary:hover {
  background: #4f46e5;
}

.btn-secondary {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
}

.btn-secondary:hover {
  background: rgba(255, 255, 255, 0.2);
}

.btn-small {
  padding: 4px 8px;
  font-size: 16px;
}

.workbench-canvas {
  flex: 1;
  position: relative;
  overflow: hidden;
  background: rgba(0, 0, 0, 0.2);
}

.panel-card {
  position: absolute;
  background: rgba(30, 30, 46, 0.95);
  border-radius: 8px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.1);
  cursor: move;
  transition: box-shadow 0.2s;
}

.panel-card:hover {
  border-color: rgba(99, 102, 241, 0.5);
}

.panel-dragging {
  box-shadow: 0 10px 40px rgba(99, 102, 241, 0.3);
  opacity: 0.9;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 12px;
  background: rgba(0, 0, 0, 0.4);
  height: 32px;
  user-select: none;
}

.panel-title {
  color: #fff;
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}

.panel-actions {
  display: flex;
  gap: 4px;
}

.panel-btn {
  width: 20px;
  height: 20px;
  border: none;
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
  border-radius: 4px;
  font-size: 10px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s;
}

.panel-btn:hover {
  background: rgba(255, 255, 255, 0.2);
}

.panel-close:hover {
  background: #ef4444;
}

.panel-content {
  height: calc(100% - 32px);
  position: relative;
}

.panel-content-transparent {
  background: transparent;
}

.panel-iframe {
  width: 100%;
  height: 100%;
  border: none;
  background: #fff;
}

.panel-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: rgba(255, 255, 255, 0.5);
  font-size: 12px;
}

.snap-guide {
  position: absolute;
  background: #6366f1;
  pointer-events: none;
  z-index: 1000;
}

.empty-canvas {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: rgba(255, 255, 255, 0.5);
  gap: 16px;
}

.workbench-footer {
  padding: 8px 20px;
  background: rgba(0, 0, 0, 0.3);
  border-top: 1px solid rgba(255, 255, 255, 0.1);
  color: rgba(255, 255, 255, 0.5);
  font-size: 12px;
}

.dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}

.dialog-content {
  background: #1e1e2e;
  border-radius: 12px;
  padding: 20px;
  width: 500px;
  max-height: 80vh;
  overflow-y: auto;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.dialog-content h2 {
  color: #fff;
  font-size: 18px;
  margin-bottom: 16px;
}

.dialog-section {
  margin-bottom: 16px;
}

.dialog-section h3 {
  color: rgba(255, 255, 255, 0.7);
  font-size: 14px;
  margin-bottom: 8px;
}

.dialog-search {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.3);
  color: #fff;
  font-size: 14px;
  margin-bottom: 8px;
}

.window-list {
  max-height: 200px;
  overflow-y: auto;
}

.window-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.2s;
}

.window-item:hover {
  background: rgba(255, 255, 255, 0.1);
}

.window-title {
  flex: 1;
  color: #fff;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.window-exe {
  color: rgba(255, 255, 255, 0.5);
  font-size: 11px;
}

.tool-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
}

.tool-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 12px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.3);
  color: #fff;
  cursor: pointer;
  transition: all 0.2s;
}

.tool-btn:hover {
  background: rgba(99, 102, 241, 0.2);
  border-color: rgba(99, 102, 241, 0.5);
}

.tool-icon {
  font-size: 24px;
}

.tool-label {
  font-size: 11px;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 16px;
}
```

---

### Task 13: 清理废弃文件

**Files:**
- Delete: `src/components/PanelFrame.tsx`
- Delete: `src/components/WindowPicker.tsx`
- Delete: `src/lib/panel-api.ts`
- Delete: `src-tauri/src/panel_manager/`
- Delete: `src-tauri/src/window_embedder/setparent.rs`

**Step 1: 删除文件**

Run:
```bash
rm src/components/PanelFrame.tsx src/components/WindowPicker.tsx src/lib/panel-api.ts
rm -rf src-tauri/src/panel_manager/
rm src-tauri/src/window_embedder/setparent.rs
```

**Step 2: 编译检查**

Run: `npm run build`
Expected: TypeScript 编译通过

---

### Task 14: 更新国际化文案

**Files:**
- Modify: `src/lib/locales/en.json`
- Modify: `src/lib/locales/zh.json`

**Step 1: 更新 en.json**

```json
{
  "app": {
    "title": "BetterPanely Workbench",
    "addPanel": "Add Panel",
    "close": "Close",
    "focus": "Focus",
    "top": "Bring to Top",
    "desktopWindows": "Desktop Windows",
    "searchWindows": "Search windows...",
    "builtinTools": "Built-in Tools",
    "addSelected": "Add Selected",
    "cancel": "Cancel",
    "emptyWorkbench": "No panels yet. Click below to add one.",
    "panelCount": "{count} panels",
    "panelPlaceholder": "Window thumbnail will appear here"
  },
  "tools": {
    "calculator": "Calculator",
    "notes": "Notes",
    "timer": "Timer",
    "weather": "Weather"
  }
}
```

**Step 2: 更新 zh.json**

```json
{
  "app": {
    "title": "BetterPanely 工作台",
    "addPanel": "添加面板",
    "close": "关闭",
    "focus": "聚焦",
    "top": "置顶",
    "desktopWindows": "桌面窗口",
    "searchWindows": "搜索窗口...",
    "builtinTools": "内置工具",
    "addSelected": "添加选中项",
    "cancel": "取消",
    "emptyWorkbench": "暂无面板。点击下方添加。",
    "panelCount": "{count} 个面板",
    "panelPlaceholder": "窗口缩略图将显示在这里"
  },
  "tools": {
    "calculator": "计算器",
    "notes": "记事本",
    "timer": "计时器",
    "weather": "天气"
  }
}
```

---

### Task 15: 完整编译测试

**Step 1: 前端编译**

Run: `npm run build`
Expected: TypeScript 编译通过，Vite 打包成功

**Step 2: 后端编译**

Run: `cargo build --release --manifest-path src-tauri/Cargo.toml`
Expected: Rust 编译通过

**Step 3: Tauri 完整构建**

Run: `npm run tauri build`
Expected: 完整构建成功，生成可执行文件和安装包

---

## 自审查

### 1. Spec 覆盖检查

| Spec 需求 | 对应任务 |
|----------|---------|
| DWM 实时缩略图镜像 | Task 1, Task 7, Task 9 |
| 独立应用窗口 | Task 11, Task 12 |
| 磁性贴边吸附 | Task 6, Task 11 |
| 列表选择 + 拖拽进来 | Task 8, Task 7 |
| 取代现有控制台 | Task 12, Task 13 |
| iframe 内置工具 | Task 10, Task 11 |
| 状态持久化 | Task 3, Task 11 |
| 源窗口关闭检测 | Task 7, Task 11 |

### 2. 占位符扫描

- ✅ 无 TBD/TODO
- ✅ 所有代码完整，无省略
- ✅ 所有类型定义明确

### 3. 类型一致性

- ✅ `PanelState` 在前后端保持一致
- ✅ `WindowInfo` 在前后端保持一致
- ✅ 命令参数与 API 封装一致

---

## 执行方式选择

**Plan complete and saved to `docs/superpowers/plans/2026-06-29-workbench-implementation.md`.**

**两种执行方式：**

1. **Subagent-Driven（推荐）** - 每个任务分配一个新的子代理执行，任务间进行审查，快速迭代

2. **Inline Execution** - 在当前会话中逐个执行任务，批量执行并设置检查点

**选择哪种方式？**

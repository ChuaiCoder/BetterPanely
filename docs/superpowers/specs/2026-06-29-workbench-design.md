# BetterPanely 工作台设计文档

## 一、需求概述

用户需要一个类似工作台的桌面应用，核心特性：
- 多个微缩面板在一个窗口内展示
- 面板内容是实时运行中的窗口缩略图（DWM 镜像）
- 面板可自由拖动
- 面板之间支持磁性吸附（边对齐 + 贴边）

## 二、核心决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 微缩呈现方式 | DWM 实时缩略图镜像 | 轻量、原窗口保持正常运行 |
| 工作台形态 | 独立应用窗口 | 简洁可控，与现有架构兼容 |
| 吸附行为 | 磁性贴边 | 丝滑体验，不改变面板尺寸 |
| 面板来源 | 列表选择 + 拖拽进来 | 灵活满足不同使用场景 |
| 与现有关系 | 取代现有控制台 | 统一产品形态，简化架构 |
| 内置工具 | iframe 直接嵌入 | 轻量、可交互、与缩略图混排 |
| 实现方案 | 单 WebView + DWM 缩略图叠加 | 复用现有架构最多，开发效率最高 |

## 三、整体架构

### 3.1 双层渲染模型

**工作台主窗口**：单个 Tauri WebView 窗口（label = `workbench`），SolidJS 渲染整个工作台 UI。

| 层级 | 职责 | 技术实现 |
|------|------|----------|
| WebView 层 | 工作台背景、面板卡片外框（标题栏、边框、按钮）、吸附辅助线、添加面板 UI | SolidJS + CSS |
| 原生缩略图层 | DWM 缩略图叠加到工作台客户区指定矩形内 | `DwmRegisterThumbnail` |

### 3.2 前后端职责划分

**前端（SolidJS + TypeScript）**：
- 面板布局管理（位置、尺寸、z-index）
- 拖拽交互与磁性吸附算法
- iframe 工具渲染
- 坐标计算与同步
- UI 交互（添加面板、关闭面板、聚焦源窗口）

**后端（Rust）**：
- DWM 缩略图注册/更新/销毁
- 窗口枚举（复用现有 `enumerator.rs`）
- 源窗口生命周期监听
- 状态持久化
- Tauri 命令实现

### 3.3 通信机制

- **前端 → 后端**：Tauri invoke 命令（`wb_add_thumbnail`、`wb_update_thumbnail_rect`、`wb_remove_panel`、`wb_focus_source` 等）
- **后端 → 前端**：Tauri 事件（`thumb:source-closed`、`drag:entered-workbench`）

### 3.4 坐标同步约定

所有缩略图的 `rcDestination` 使用**工作台客户区坐标**（相对 WebView 左上角，逻辑像素）。前端在每次面板移动/缩放/工作台 resize 后，调用 `wb_update_thumbnail_rect(panel_id, x, y, w, h)` 同步到后端。

## 四、核心模块与组件

### 4.1 前端模块

#### 4.1.1 `App.tsx`（重构）
- 根组件，挂载 `WorkbenchCanvas`
- 初始化时加载持久化布局
- 注册全局事件监听（快捷键、窗口事件）

#### 4.1.2 `components/WorkbenchCanvas.tsx`（新增）
- 工作台主画布
- 管理所有面板的布局状态
- 处理画布空白处的右键菜单
- 监听工作台窗口 resize，触发缩略图坐标重算

#### 4.1.3 `components/ThumbPanel.tsx`（新增）
- 缩略图面板卡片
- 渲染标题栏（窗口标题 + 关闭 + 聚焦 + 置顶按钮）
- 内容区是透明 div，让 DWM 缩略图透出
- 点击内容区 → 聚焦源窗口

#### 4.1.4 `components/ToolPanel.tsx`（新增）
- 内置工具面板卡片
- 结构同 `ThumbPanel`，内容区是 iframe
- iframe 可直接交互

#### 4.1.5 `components/AddPanelDialog.tsx`（新增）
- 添加面板对话框
- 窗口列表选择 + 内置工具快捷按钮
- 支持搜索过滤和多选

#### 4.1.6 `lib/snap-engine.ts`（新增）
- 磁性吸附引擎（纯 TS 函数，无副作用）
- 输入：被拖面板 rect + 其他面板 rect 列表 + 阈值
- 输出：吸附后的 rect + 辅助线信息

#### 4.1.7 `lib/workbench-api.ts`（新增）
- 封装所有工作台相关的 Tauri invoke 调用
- 统一错误处理

#### 4.1.8 `lib/types.ts`（扩展）
- 添加 `PanelState`、`SnapGuide` 等新类型

### 4.2 后端模块

#### 4.2.1 `thumbnail/`（新增模块）
- `mod.rs`：对外接口（register、update_rect、update_visibility、unregister、unregister_all）
- `manager.rs`：`ThumbnailManager` 结构体，维护缩略图映射表
- `dwm.rs`：封装 DWM API 的 unsafe 调用

#### 4.2.2 `commands/workbench_cmds.rs`（新增）
- `wb_add_thumbnail(source_hwnd)` → 注册缩略图，返回 panel_id
- `wb_update_thumbnail_rect(panel_id, x, y, w, h)` → 更新缩略图位置
- `wb_remove_panel(panel_id)` → 移除面板
- `wb_focus_source(source_hwnd)` → 聚焦源窗口
- `wb_get_workbench_hwnd()` → 返回工作台窗口 HWND
- `wb_save_layout()` / `wb_load_layout()` → 持久化

#### 4.2.3 `window_embedder/`（重构）
- 保留 `enumerator.rs`（窗口枚举）
- 废弃 `setparent.rs`（不再使用 SetParent 嵌入）

#### 4.2.4 `commands/`（清理）
- 废弃 `panel_cmds.rs`（独立面板窗口模式淘汰）
- 废弃 `embed_cmds.rs` 中的 embed/release/drag_capture 命令
- 保留 `tool_cmds.rs` 和 `settings_cmds.rs`

#### 4.2.5 `state.rs`（扩展）
- 持久化结构改为工作台布局
- 保存：面板类型、源 HWND/tool_id、位置尺寸、z-index

### 4.3 现有代码处理清单

| 现有代码 | 处理 |
|---------|------|
| `src/App.tsx` | 重写为工作台挂载点 |
| `src/components/PanelFrame.tsx` | 废弃 |
| `src/components/WindowPicker.tsx` | 替换为 `AddPanelDialog.tsx` |
| `src/lib/panel-api.ts` | 替换为 `workbench-api.ts` |
| `src-tauri/src/panel_manager/` | 废弃 |
| `src-tauri/src/window_embedder/setparent.rs` | 废弃 |
| `src-tauri/src/window_embedder/enumerator.rs` | 保留 |
| `src-tauri/src/drag_capture/` | 简化，仅保留热键捕获 |
| `src-tauri/src/builtin_tools/` | 保留 |
| `src-tauri/src/locales/`、`tray.rs` | 保留，文案更新 |

## 五、数据流程与状态管理

### 5.1 前端状态

```typescript
interface PanelState {
  id: string;
  type: "thumbnail" | "tool";
  sourceHwnd?: number;        // type=thumbnail 时
  toolId?: string;            // type=tool 时
  title: string;
  x: number;
  y: number;
  width: number;
  height: number;
  zIndex: number;
  visible: boolean;
}

interface SnapGuide {
  type: "vertical" | "horizontal";
  position: number;
  targetPanelId: string;
}

// SolidJS Signals
const [panels, setPanels] = createSignal<PanelState[]>([]);
const [draggingId, setDraggingId] = createSignal<string | null>(null);
const [dragOffset, setDragOffset] = createSignal({ x: 0, y: 0 });
const [snapGuides, setSnapGuides] = createSignal<SnapGuide[]>([]);
```

### 5.2 后端状态

```rust
pub struct ThumbnailManager {
    thumbnails: HashMap<isize, ThumbnailHandle>,  // 源HWND → 缩略图
    panel_map: HashMap<String, isize>,             // panel_id → 源HWND
    next_id: u32,
}

pub struct ThumbnailHandle {
    source_hwnd: isize,
    thumbnail_id: DWM_THUMBNAIL_ID,
    dest_rect: RECT,
    visible: bool,
    opacity: f32,
}
```

### 5.3 关键数据流

#### 5.3.1 添加面板（列表选择）
1. 用户打开 `AddPanelDialog` → 调用 `enumerate_windows`
2. 后端返回窗口列表 → 前端渲染勾选列表
3. 用户确认 → 前端创建 `PanelState`，调用 `wb_add_thumbnail`
4. 后端调用 `DwmRegisterThumbnail`
5. 前端调用 `wb_update_thumbnail_rect` 同步初始位置

#### 5.3.2 添加面板（拖拽进来）
1. Rust `drag_capture` 检测到外部窗口 drag 开始
2. 鼠标进入工作台 → 发出 `drag:entered-workbench` 事件
3. 前端创建 `PanelState`，调用 `wb_add_thumbnail`
4. 拖拽结束 → 同步最终位置

#### 5.3.3 拖拽与吸附
1. 用户按下标题栏 → 记录 dragOffset
2. 鼠标移动 → 计算新位置 → 调用 `snapEngine.snap()`
3. 更新面板状态 → 调用 `wb_update_thumbnail_rect`
4. 后端 `DwmUpdateThumbnailProperties` 更新位置（~60fps）

#### 5.3.4 源窗口关闭
1. Rust 通过 `SetWinEventHook` 监听 `EVENT_OBJECT_DESTROY`
2. 检测到源 HWND 销毁 → 发出 `thumb:source-closed` 事件
3. 前端移除面板 → 调用 `wb_remove_panel`
4. 后端 `DwmUnregisterThumbnail` 清理

#### 5.3.5 状态持久化
1. 关闭工作台 → 调用 `wb_save_layout`
2. 下次启动 → `wb_load_layout` 返回布局
3. 前端恢复面板位置；缩略图面板尝试重新注册

## 六、UI 设计与交互细节

### 6.1 工作台主窗口

```
┌─────────────────────────────────────────────┐
│ BetterPanely            ⚙ [EN/ZH]          │  ← 标题栏
├─────────────────────────────────────────────┤
│                                             │
│     ┌─────────────────┐                     │
│     │ [标题] [✕][▣][↑]│                     │
│     ├─────────────────┤                     │  ← 画布区
│     │   DWM缩略图     │                     │
│     └─────────────────┘                     │
│                                             │
│     ┌─────────────────┐                     │
│     │ [标题] [✕][▣][↑]│                     │
│     ├─────────────────┤                     │
│     │    iframe工具    │                     │
│     └─────────────────┘                     │
│                                             │
├─────────────────────────────────────────────┤
│ 面板: 2  [添加面板]                          │  ← 状态栏
└─────────────────────────────────────────────┘
```

### 6.2 面板卡片结构

```
┌──────────────────────────────────┐
│ [标题]              [✕] [▣] [↑]  │  ← 标题栏（可拖拽）
├──────────────────────────────────┤
│                                  │
│      DWM缩略图 / iframe          │  ← 内容区（透明）
│                                  │
└──────────────────────────────────┘
```

**标题栏按钮**：
- `[✕]` 关闭：移除面板
- `[▣]` 聚焦/展开：缩略图→聚焦源窗口；工具→独立窗口
- `[↑]` 置顶：提升 z-index

### 6.3 磁性吸附规则

- **阈值**：8 像素（可配置）
- **吸附目标**：其他面板的四条边、工作台窗口内边缘
- **行为**：距离 < 阈值时自动对齐贴边，只改位置不改尺寸

### 6.4 AddPanelDialog 对话框

```
┌──────────────────────────────────┐
│      添加面板到工作台             │
├──────────────┬───────────────────┤
│   桌面窗口   │  [搜索框]         │
├──────────────┼───────────────────┤
│ ☐ 窗口标题1  │  ☐ 窗口标题2      │
│ ☐ 窗口标题3  │  ☐ 窗口标题4      │
│ ⋮            │  ⋮               │
├──────────────┴───────────────────┤
│   内置工具                       │
├──────────────────────────────────┤
│ [🔢计算器] [📝笔记] [⏱️计时] [🌤️天气] │
├──────────────────────────────────┤
│         [取消] [加入工作台]       │
└──────────────────────────────────┘
```

### 6.5 快捷键

| 快捷键 | 功能 |
|-------|------|
| `Ctrl+Shift+W` | 捕获当前焦点窗口到工作台 |
| `Ctrl+N` | 打开添加面板对话框 |
| `Ctrl+S` | 保存当前布局 |
| `Ctrl+Shift+F` | 聚焦/展开当前选中面板 |
| `Delete` | 删除当前选中面板 |

### 6.6 主题与样式

- 支持浅色/深色/系统主题
- 面板边框：圆角 8px，1px 灰色边框，悬停高亮
- 标题栏：背景略深，高度 32px，文字居中
- 内容区：透明（缩略图透出）

## 七、错误处理

| 场景 | 原因 | 处理 |
|------|------|------|
| 缩略图注册失败 | 源窗口已关闭、不支持、权限不足 | 前端显示 toast 提示 |
| 缩略图更新失败 | 工作台/源窗口已销毁 | 静默失败，下次检测清理 |
| 源窗口生命周期追踪 | 窗口关闭未检测 | WinEventHook + 定期检查（30s） |
| 拖拽坐标同步失败 | DPI 缩放不一致 | 后端统一使用逻辑像素 |
| 持久化恢复失败 | 格式损坏、源窗口不存在 | 跳过失效面板，记录日志 |

## 八、性能考虑

- **缩略图更新频率**：拖拽期间每帧更新（DWM API 轻量），非拖拽期间按需更新
- **内存管理**：每个缩略图约 1MB GPU 内存，建议限制 10-15 个同时显示
- **渲染优化**：WebView 只负责面板外框，内容由 DWM/iframe 处理

## 九、测试策略

- **前端单元测试**：吸附算法测试
- **后端集成测试**：窗口枚举、缩略图生命周期、源窗口关闭检测
- **E2E 测试**：添加面板、拖拽吸附、聚焦源窗口、状态持久化
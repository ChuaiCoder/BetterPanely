import { createSignal, createEffect, For, Show, onMount, onCleanup } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ThumbPanel } from "./ThumbPanel";
import { ToolPanel } from "./ToolPanel";
import { AddPanelDialog } from "./AddPanelDialog";
import { useI18n } from "../lib/i18n";
import { snap } from "../lib/snap-engine";
import {
  addThumbnail,
  captureFocusedWindow,
  focusSource,
  removePanel,
  syncThumbnailStack,
  updateThumbnailLayout,
  loadLayout,
  saveLayout,
  openToolWindow,
} from "../lib/workbench-api";
import type { ThumbnailRect, ThumbnailRegistration } from "../lib/workbench-api";
import type { PanelState, SnapGuide, WindowInfo } from "../lib/types";

const PANEL_HEADER_HEIGHT = 32;
const DEFAULT_THUMBNAIL_CONTENT_WIDTH = 240;
const DEFAULT_THUMBNAIL_CONTENT_HEIGHT = 150;
const MIN_THUMBNAIL_CONTENT_WIDTH = 160;
const MAX_THUMBNAIL_CONTENT_WIDTH = 360;
const MAX_THUMBNAIL_CONTENT_HEIGHT = 420;
const NOTICE_TIMEOUT_MS = 5000;
const THUMBNAIL_HEALTH_INTERVAL_MS = 30000;
const THUMBNAIL_SYNC_NOTICE_COOLDOWN_MS = 10000;
const TOOL_CONFIG: Record<string, { width: number; height: number }> = {
  calculator: { width: 280, height: 420 },
  notes: { width: 350, height: 400 },
  timer: { width: 300, height: 200 },
  weather: { width: 300, height: 350 },
};

interface WorkbenchNotice {
  id: number;
  type: "info" | "success" | "error";
  message: string;
}

interface CanvasContextMenu {
  x: number;
  y: number;
}

interface SourceClosedPayload {
  panelId: string;
  sourceHwnd: number;
}

interface DragEnteredWorkbenchPayload {
  sourceHwnd: number;
  title: string;
  x: number;
  y: number;
}

interface DragPositionPayload {
  sourceHwnd: number;
  x: number;
  y: number;
}

interface PanelInitialPosition {
  x: number;
  y: number;
}

interface CanvasSize {
  width: number;
  height: number;
}

interface ThumbnailPanelSize {
  width: number;
  height: number;
}

interface CssRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/**
 * 工作台主画布组件
 * 管理所有面板的布局、拖拽、磁性吸附、状态持久化等核心功能
 */
export function WorkbenchCanvas() {
  const { t, lang } = useI18n();
  const [panels, setPanels] = createSignal<PanelState[]>([]);
  const [draggingId, setDraggingId] = createSignal<string | null>(null);
  const [dragOffset, setDragOffset] = createSignal({ x: 0, y: 0 });
  const [snapGuides, setSnapGuides] = createSignal<SnapGuide[]>([]);
  const [isDialogOpen, setIsDialogOpen] = createSignal(false);
  const [canvasSize, setCanvasSize] = createSignal<CanvasSize>({ width: 800, height: 600 });
  const [layoutReady, setLayoutReady] = createSignal(false);
  const [selectedPanelId, setSelectedPanelId] = createSignal<string | null>(null);
  const [notices, setNotices] = createSignal<WorkbenchNotice[]>([]);
  const [contextMenu, setContextMenu] = createSignal<CanvasContextMenu | null>(null);
  const [draggedExternalPanelId, setDraggedExternalPanelId] = createSignal<string | null>(null);

  let canvasRef!: HTMLDivElement;
  let saveTimer: number | undefined;
  let thumbnailSyncFrame: number | undefined;
  let autoSaveFailureNotified = false;
  let lastThumbnailSyncFailureNoticeAt = 0;
  const noticeTimers: number[] = [];

  const getNextZIndex = () => {
    const max = panels().reduce((acc, p) => Math.max(acc, p.zIndex), 0);
    return max + 1;
  };

  const clamp = (value: number, min: number, max: number) =>
    Math.min(Math.max(value, min), max);

  const errorMessage = (error: unknown) =>
    error instanceof Error ? error.message : String(error);

  const isStaleThumbnailError = (error: unknown) => {
    const message = errorMessage(error).toLowerCase();
    return (
      message.includes("source window is no longer available") ||
      message.includes("thumbnail not found")
    );
  };

  const showNotice = (message: string, type: WorkbenchNotice["type"] = "error") => {
    const id = Date.now() + noticeTimers.length;
    setNotices((prev) => [...prev, { id, type, message }]);

    const timer = window.setTimeout(() => {
      setNotices((prev) => prev.filter((notice) => notice.id !== id));
    }, NOTICE_TIMEOUT_MS);
    noticeTimers.push(timer);
  };

  const handleEventListenerError = (eventName: string, error: unknown) => {
    console.error(`Failed to listen for ${eventName}:`, error);
    showNotice(t("app.toast.eventListenerFailed", { event: eventName, reason: errorMessage(error) }));
  };

  const markSaveLayoutSuccess = () => {
    autoSaveFailureNotified = false;
  };

  const reportSaveLayoutFailure = (
    context: "manual" | "autosave" | "cleanup",
    error: unknown,
    notify = false
  ) => {
    console.error(`Failed to save layout (${context}):`, error);
    if (!notify) return;
    if (context === "autosave") {
      if (autoSaveFailureNotified) return;
      autoSaveFailureNotified = true;
    }
    showNotice(t("app.toast.saveLayoutFailed", { reason: errorMessage(error) }));
  };

  const reportThumbnailSyncFailure = (context: string, error: unknown) => {
    console.error(`Failed to sync thumbnail rect (${context}):`, error);
    const now = Date.now();
    if (now - lastThumbnailSyncFailureNoticeAt < THUMBNAIL_SYNC_NOTICE_COOLDOWN_MS) return;
    lastThumbnailSyncFailureNoticeAt = now;
    showNotice(t("app.toast.thumbnailSyncFailed", { reason: errorMessage(error) }));
  };

  const toolTitle = (toolId: string) => {
    const key = `tools.${toolId}`;
    const translated = t(key);
    return translated === key ? toolId : translated;
  };

  const withLocalizedToolTitle = (panel: PanelState): PanelState => {
    if (panel.type !== "tool" || !panel.toolId) return panel;
    const title = toolTitle(panel.toolId);
    return panel.title === title ? panel : { ...panel, title };
  };

  const waitForNextFrame = () =>
    new Promise<void>((resolve) => {
      window.requestAnimationFrame(() => resolve());
    });

  const syncToolPanelTitles = () => {
    setPanels((prev) => {
      let changed = false;
      const next = prev.map((panel) => {
        const localized = withLocalizedToolTitle(panel);
        if (localized !== panel) changed = true;
        return localized;
      });
      return changed ? next : prev;
    });
  };

  const getThumbnailContentElement = (panelId: string) =>
    Array.from(canvasRef?.querySelectorAll<HTMLElement>("[data-thumbnail-panel-id]") ?? [])
      .find((element) => element.dataset.thumbnailPanelId === panelId) ?? null;

  const getPanelCardElement = (panelId: string) =>
    Array.from(canvasRef?.querySelectorAll<HTMLElement>("[data-panel-id]") ?? [])
      .find((element) => element.dataset.panelId === panelId) ?? null;

  const nativeScale = () => window.devicePixelRatio || 1;

  const cssRectToNative = (rect: CssRect) => {
    const scale = nativeScale();
    const left = Math.round(rect.left * scale);
    const top = Math.round(rect.top * scale);
    const right = Math.round(rect.right * scale);
    const bottom = Math.round(rect.bottom * scale);

    return {
      x: left,
      y: top,
      width: Math.max(1, right - left),
      height: Math.max(1, bottom - top),
    };
  };

  const panelCardRect = (panel: PanelState): CssRect => {
    const card = getPanelCardElement(panel.id);
    if (card) {
      const rect = card.getBoundingClientRect();
      return {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
      };
    }

    const canvasRect = canvasRef?.getBoundingClientRect();
    const left = panel.x + (canvasRect?.left ?? 0);
    const top = panel.y + (canvasRect?.top ?? 0);
    return {
      left,
      top,
      right: left + panel.width,
      bottom: top + panel.height,
    };
  };

  const intersectCssRect = (a: CssRect, b: CssRect): CssRect | null => {
    const rect = {
      left: Math.max(a.left, b.left),
      top: Math.max(a.top, b.top),
      right: Math.min(a.right, b.right),
      bottom: Math.min(a.bottom, b.bottom),
    };

    return rect.right > rect.left && rect.bottom > rect.top ? rect : null;
  };

  const subtractCssRect = (base: CssRect, occluder: CssRect): CssRect[] => {
    const intersection = intersectCssRect(base, occluder);
    if (!intersection) return [base];

    const pieces: CssRect[] = [
      { left: base.left, top: base.top, right: base.right, bottom: intersection.top },
      { left: base.left, top: intersection.bottom, right: base.right, bottom: base.bottom },
      { left: base.left, top: intersection.top, right: intersection.left, bottom: intersection.bottom },
      { left: intersection.right, top: intersection.top, right: base.right, bottom: intersection.bottom },
    ];

    return pieces.filter((piece) => piece.right > piece.left && piece.bottom > piece.top);
  };

  const visibleThumbnailRects = (panel: PanelState, fullRect: CssRect, items: PanelState[]) => {
    const panelIndex = items.findIndex((item) => item.id === panel.id);
    const occluders = items
      .map((item, index) => ({ item, index }))
      .filter(({ item, index }) => {
        if (item.id === panel.id) return false;
        return item.zIndex > panel.zIndex || (item.zIndex === panel.zIndex && index > panelIndex);
      })
      .sort((left, right) =>
        left.item.zIndex === right.item.zIndex
          ? left.index - right.index
          : left.item.zIndex - right.item.zIndex
      );

    return occluders.reduce<CssRect[]>((visible, { item }) => {
      const occluder = panelCardRect(item);
      return visible.flatMap((rect) => subtractCssRect(rect, occluder));
    }, [fullRect]);
  };

  const getThumbnailPanelSize = (
    thumbnail: Pick<ThumbnailRegistration, "sourceWidth" | "sourceHeight">,
    preferredContentWidth = DEFAULT_THUMBNAIL_CONTENT_WIDTH
  ): ThumbnailPanelSize => {
    if (
      !Number.isFinite(thumbnail.sourceWidth) ||
      !Number.isFinite(thumbnail.sourceHeight) ||
      thumbnail.sourceWidth <= 0 ||
      thumbnail.sourceHeight <= 0
    ) {
      return {
        width: DEFAULT_THUMBNAIL_CONTENT_WIDTH,
        height: DEFAULT_THUMBNAIL_CONTENT_HEIGHT + PANEL_HEADER_HEIGHT,
      };
    }

    const aspectRatio = thumbnail.sourceWidth / thumbnail.sourceHeight;
    let contentWidth = clamp(
      preferredContentWidth,
      MIN_THUMBNAIL_CONTENT_WIDTH,
      MAX_THUMBNAIL_CONTENT_WIDTH
    );
    let contentHeight = contentWidth / aspectRatio;

    if (contentHeight > MAX_THUMBNAIL_CONTENT_HEIGHT) {
      contentHeight = MAX_THUMBNAIL_CONTENT_HEIGHT;
      contentWidth = contentHeight * aspectRatio;
    }

    if (contentWidth > MAX_THUMBNAIL_CONTENT_WIDTH) {
      contentWidth = MAX_THUMBNAIL_CONTENT_WIDTH;
      contentHeight = contentWidth / aspectRatio;
    }

    if (contentWidth < MIN_THUMBNAIL_CONTENT_WIDTH && contentHeight < MAX_THUMBNAIL_CONTENT_HEIGHT) {
      contentWidth = Math.min(MIN_THUMBNAIL_CONTENT_WIDTH, MAX_THUMBNAIL_CONTENT_WIDTH);
      contentHeight = contentWidth / aspectRatio;
    }

    return {
      width: Math.max(1, Math.round(contentWidth)),
      height: Math.max(1, Math.round(contentHeight + PANEL_HEADER_HEIGHT)),
    };
  };

  const getThumbnailCssRect = (panel: PanelState): CssRect => {
    const content = getThumbnailContentElement(panel.id);
    if (content) {
      const rect = content.getBoundingClientRect();
      return {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
      };
    }

    const canvasRect = canvasRef?.getBoundingClientRect();
    const left = panel.x + (canvasRect?.left ?? 0);
    const top = panel.y + (canvasRect?.top ?? 0) + PANEL_HEADER_HEIGHT;
    return {
      left,
      top,
      right: left + panel.width,
      bottom: top + Math.max(1, panel.height - PANEL_HEADER_HEIGHT),
    };
  };

  const constrainPanelPosition = (panel: PanelState, size: CanvasSize = canvasSize()) => {
    const x = clamp(panel.x, 8, Math.max(8, size.width - panel.width - 8));
    const y = clamp(panel.y, 8, Math.max(8, size.height - panel.height - 8));
    return x === panel.x && y === panel.y ? panel : { ...panel, x, y };
  };

  const removePanelState = (panelId: string) => {
    if (selectedPanelId() === panelId) {
      setSelectedPanelId(null);
    }
    if (draggedExternalPanelId() === panelId) {
      setDraggedExternalPanelId(null);
    }
    let nextPanels: PanelState[] = [];
    setPanels((prev) => {
      nextPanels = prev.filter((p) => p.id !== panelId);
      return nextPanels;
    });
    return nextPanels;
  };

  const removeClosedSourcePanel = (panelId: string, sourceHwnd: number) => {
    const panel = panels().find(
      (p) => p.id === panelId || (p.type === "thumbnail" && p.sourceHwnd === sourceHwnd)
    );
    if (!panel) return;

    const nextPanels = removePanelState(panel.id);
    syncThumbnailStackOrder(nextPanels, "source-closed");
    syncAllThumbnailRects("source-closed", nextPanels);
    showNotice(t("app.toast.sourceClosed", { title: panel.title }), "info");
  };

  const syncThumbnailRect = async (
    panel: PanelState,
    context = "sync",
    items: PanelState[] = panels()
  ) => {
    if (panel.type !== "thumbnail" || !panel.sourceHwnd) return true;
    const fullCssRect = getThumbnailCssRect(panel);
    const fullRect = cssRectToNative(fullCssRect);
    const visibleRects: ThumbnailRect[] = visibleThumbnailRects(panel, fullCssRect, items)
      .map(cssRectToNative)
      .filter((rect) => rect.width > 0 && rect.height > 0);
    try {
      await updateThumbnailLayout(panel.id, fullRect, visibleRects);
      return true;
    } catch (e) {
      if (isStaleThumbnailError(e)) {
        removeClosedSourcePanel(panel.id, panel.sourceHwnd);
        return false;
      }
      reportThumbnailSyncFailure(context, e);
      return false;
    }
  };

  const thumbnailPanelsInStackOrder = (items: PanelState[] = panels()) =>
    [...items]
      .filter((panel) => panel.type === "thumbnail" && panel.sourceHwnd)
      .sort((left, right) => left.zIndex - right.zIndex);

  const syncThumbnailStackOrder = (items: PanelState[] = panels(), context = "stack") => {
    const panelIds = thumbnailPanelsInStackOrder(items).map((panel) => panel.id);
    if (panelIds.length <= 1) return;

    syncThumbnailStack(panelIds).catch((error) => reportThumbnailSyncFailure(context, error));
  };

  const syncAllThumbnailRects = (context: string, items: PanelState[] = panels()) => {
    thumbnailPanelsInStackOrder(items).forEach((panel) => {
      void syncThumbnailRect(panel, context, items);
    });
  };

  const scheduleThumbnailRectsSync = (context: string) => {
    if (thumbnailSyncFrame !== undefined) return;

    thumbnailSyncFrame = window.requestAnimationFrame(() => {
      thumbnailSyncFrame = undefined;
      syncAllThumbnailRects(context);
    });
  };

  const getPanelInitialPosition = (
    width: number,
    height: number,
    initialPosition?: PanelInitialPosition
  ) => {
    const x = initialPosition ? initialPosition.x - width / 2 : 100 + panels().length * 20;
    const y = initialPosition ? initialPosition.y - height / 2 : 100 + panels().length * 20;

    return {
      x: clamp(x, 8, Math.max(8, canvasSize().width - width - 8)),
      y: clamp(y, 8, Math.max(8, canvasSize().height - height - 8)),
    };
  };

  const workbenchClientPositionToCanvas = (
    payload: DragEnteredWorkbenchPayload | DragPositionPayload
  ): PanelInitialPosition => {
    const canvasRect = canvasRef.getBoundingClientRect();
    return {
      x: payload.x - canvasRect.left,
      y: payload.y - canvasRect.top,
    };
  };

  const movePanelToPosition = (panelId: string, initialPosition: PanelInitialPosition) => {
    const panel = panels().find((p) => p.id === panelId);
    if (!panel) return;

    const position = getPanelInitialPosition(panel.width, panel.height, initialPosition);
    const movedPanel = { ...panel, x: position.x, y: position.y };
    setPanels((prev) => prev.map((p) => (p.id === panelId ? movedPanel : p)));
    if (movedPanel.type === "thumbnail") {
      scheduleThumbnailRectsSync("external-drop");
    }
  };

  createEffect(() => {
    const ready = layoutReady();
    const snapshot = panels();
    if (!ready) return;

    if (saveTimer !== undefined) {
      window.clearTimeout(saveTimer);
    }
    saveTimer = window.setTimeout(() => {
      saveLayout(snapshot)
        .then(markSaveLayoutSuccess)
        .catch((error) => reportSaveLayoutFailure("autosave", error, true));
    }, 400);
  });

  const addThumbnailPanel = async (
    hwnd: number,
    title: string,
    initialPosition?: PanelInitialPosition
  ): Promise<PanelState | null> => {
    let panelId: string | null = null;
    try {
      const thumbnail = await addThumbnail(hwnd);
      panelId = thumbnail.panelId;
      const { width, height } = getThumbnailPanelSize(thumbnail);
      const position = getPanelInitialPosition(width, height, initialPosition);
      const newPanel: PanelState = {
        id: panelId,
        type: "thumbnail",
        sourceHwnd: hwnd,
        title,
        x: position.x,
        y: position.y,
        width,
        height,
        zIndex: getNextZIndex(),
        visible: true,
      };
      setPanels((prev) => [...prev, newPanel]);
      await waitForNextFrame();
      const synced = await syncThumbnailRect(newPanel, "add");
      if (!synced) {
        try {
          await removePanel(panelId);
        } catch (cleanupError) {
          console.error("Failed to clean up thumbnail after sync failure:", cleanupError);
        }
        const nextPanels = removePanelState(panelId);
        syncThumbnailStackOrder(nextPanels, "add-cleanup");
        syncAllThumbnailRects("add-cleanup", nextPanels);
        return null;
      }
      syncThumbnailStackOrder(panels(), "add");
      syncAllThumbnailRects("add", panels());
      return newPanel;
    } catch (e) {
      if (panelId) {
        try {
          await removePanel(panelId);
        } catch (cleanupError) {
          console.error("Failed to clean up thumbnail after add failure:", cleanupError);
        }
      }
      console.error("Failed to add thumbnail:", e);
      showNotice(t("app.toast.addThumbnailFailed", { reason: errorMessage(e) }));
      return null;
    }
  };

  const addToolPanel = (toolId: string) => {
    const config = TOOL_CONFIG[toolId];
    if (!config) {
      console.warn("Ignored unknown tool:", toolId);
      showNotice(t("app.toast.openToolFailed", { reason: t("error.unknownTool", { toolId }) }));
      return;
    }

    const newPanel: PanelState = {
      id: `tool_${toolId}_${Date.now()}`,
      type: "tool",
      toolId,
      title: toolTitle(toolId),
      ...getPanelInitialPosition(config.width, config.height),
      width: config.width,
      height: config.height,
      zIndex: getNextZIndex(),
      visible: true,
    };
    setPanels((prev) => [...prev, newPanel]);
    scheduleThumbnailRectsSync("add-tool");
  };

  const handleAddThumbnails = (windows: WindowInfo[]) => {
    windows.forEach((windowInfo) => {
      void addThumbnailPanel(windowInfo.hwnd, windowInfo.title);
    });
  };

  const handleClosePanel = async (panelId: string) => {
    const panel = panels().find((p) => p.id === panelId);
    try {
      if (panel?.type === "thumbnail") {
        await removePanel(panelId);
      }
      const nextPanels = removePanelState(panelId);
      syncThumbnailStackOrder(nextPanels, "remove");
      syncAllThumbnailRects("remove", nextPanels);
    } catch (e) {
      if (panel?.type === "thumbnail" && isStaleThumbnailError(e)) {
        const nextPanels = removePanelState(panel.id);
        syncThumbnailStackOrder(nextPanels, "remove-stale");
        syncAllThumbnailRects("remove-stale", nextPanels);
        return;
      }
      console.error("Failed to remove panel:", e);
      showNotice(t("app.toast.removePanelFailed", { reason: errorMessage(e) }));
    }
  };

  const handleSelectPanel = (panelId: string) => {
    setSelectedPanelId(panelId);
  };

  const handleTop = (panelId: string) => {
    let nextPanels: PanelState[] | null = null;
    setPanels((prev) => {
      const nextZIndex = prev.reduce((acc, panel) => Math.max(acc, panel.zIndex), 0) + 1;
      nextPanels = prev.map((p) => (p.id === panelId ? { ...p, zIndex: nextZIndex } : p));
      return nextPanels;
    });
    if (nextPanels) {
      syncThumbnailStackOrder(nextPanels, "top");
      syncAllThumbnailRects("top", nextPanels);
    }
  };

  const handleDragStart = (panelId: string, offsetX: number, offsetY: number) => {
    handleSelectPanel(panelId);
    setContextMenu(null);
    setDraggedExternalPanelId(null);
    setDraggingId(panelId);
    setDragOffset({ x: offsetX, y: offsetY });
    handleTop(panelId);
  };

  const isEditableShortcutTarget = (target: EventTarget | null) => {
    if (!(target instanceof HTMLElement)) return false;
    const tagName = target.tagName.toLowerCase();
    return (
      tagName === "input" ||
      tagName === "textarea" ||
      tagName === "select" ||
      target.isContentEditable
    );
  };

  const focusSelectedPanel = async () => {
    const panel = panels().find((p) => p.id === selectedPanelId());
    if (!panel) return;

    await focusPanel(panel);
  };

  const saveCurrentLayout = () =>
    saveLayout(panels())
      .then(() => {
        markSaveLayoutSuccess();
        showNotice(t("app.toast.layoutSaved"), "success");
      })
      .catch((error) => reportSaveLayoutFailure("manual", error, true));

  const focusPanel = async (panel: PanelState) => {
    if (panel.type === "thumbnail" && panel.sourceHwnd) {
      try {
        await focusSource(panel.sourceHwnd);
      } catch (e) {
        console.error("Failed to focus source window:", e);
        showNotice(t("app.toast.focusFailed", { reason: errorMessage(e) }));
      }
    } else if (panel.type === "tool" && panel.toolId) {
      try {
        await openToolWindow(panel.toolId);
      } catch (e) {
        console.error("Failed to open tool window:", e);
        showNotice(t("app.toast.openToolFailed", { reason: errorMessage(e) }));
      }
    } else {
      handleTop(panel.id);
    }
  };

  const handleFocusPanel = (panelId: string) => {
    const panel = panels().find((p) => p.id === panelId);
    if (!panel) return;

    void focusPanel(panel);
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (isEditableShortcutTarget(e.target)) return;

    const key = e.key.toLowerCase();

    if (e.ctrlKey && !e.shiftKey && key === "n") {
      e.preventDefault();
      setIsDialogOpen(true);
      return;
    }

    if (e.ctrlKey && !e.shiftKey && key === "s") {
      e.preventDefault();
      saveCurrentLayout();
      return;
    }

    if (isDialogOpen()) return;

    if (e.ctrlKey && e.shiftKey && key === "f") {
      e.preventDefault();
      focusSelectedPanel().catch((error) => {
        console.error("Failed to focus selected panel:", error);
        showNotice(t("app.toast.focusFailed", { reason: errorMessage(error) }));
      });
      return;
    }

    if (e.key === "Delete") {
      const panelId = selectedPanelId();
      if (!panelId) return;

      e.preventDefault();
      void handleClosePanel(panelId);
    }
  };

  const handleMouseMove = (e: MouseEvent) => {
    const activePanelId = draggingId();
    if (!activePanelId) return;
    const canvas = canvasRef;
    if (!canvas) return;

    const canvasRect = canvas.getBoundingClientRect();
    const mouseX = e.clientX - canvasRect.left;
    const mouseY = e.clientY - canvasRect.top;

    const newX = mouseX - dragOffset().x;
    const newY = mouseY - dragOffset().y;

    let didMovePanel = false;
    setPanels((prev) => {
      const otherPanels = prev.filter((p) => p.id !== activePanelId);
      const draggedPanel = prev.find((p) => p.id === activePanelId);

      if (!draggedPanel) return prev;

      const draggedRect = { x: newX, y: newY, width: draggedPanel.width, height: draggedPanel.height };
      const snapResult = snap(draggedRect, otherPanels, canvasSize().width, canvasSize().height);

      setSnapGuides(snapResult.guides);

      const movedPanel = constrainPanelPosition({
        ...draggedPanel,
        x: snapResult.rect.x,
        y: snapResult.rect.y,
      });
      didMovePanel = true;
      return prev.map((p) => (p.id === activePanelId ? movedPanel : p));
    });
    if (didMovePanel) {
      scheduleThumbnailRectsSync("drag");
    }
  };

  const handleCanvasContextMenu = (e: MouseEvent) => {
    e.preventDefault();

    if ((e.target as HTMLElement).closest(".panel-card")) {
      setContextMenu(null);
      return;
    }

    const canvasRect = canvasRef.getBoundingClientRect();
    const menuWidth = 180;
    const menuHeight = 88;
    const x = Math.min(
      Math.max(8, e.clientX - canvasRect.left),
      Math.max(8, canvasRect.width - menuWidth - 8)
    );
    const y = Math.min(
      Math.max(8, e.clientY - canvasRect.top),
      Math.max(8, canvasRect.height - menuHeight - 8)
    );

    setContextMenu({ x, y });
  };

  const handleCanvasClick = () => {
    if (contextMenu()) {
      setContextMenu(null);
    }
  };

  const handleContextAddPanel = (e: MouseEvent) => {
    e.stopPropagation();
    setContextMenu(null);
    setIsDialogOpen(true);
  };

  const handleContextSaveLayout = (e: MouseEvent) => {
    e.stopPropagation();
    setContextMenu(null);
    saveCurrentLayout();
  };

  const handleOpenSettings = async () => {
    try {
      await invoke("open_settings");
    } catch (error) {
      console.error("Failed to open settings:", error);
      showNotice(t("app.toast.openSettingsFailed", { reason: errorMessage(error) }));
    }
  };

  const handleMouseUp = async () => {
    if (!draggingId()) return;

    const panel = panels().find((p) => p.id === draggingId());
    if (panel) {
      syncAllThumbnailRects("drop");
    }

    setDraggingId(null);
    setSnapGuides([]);
  };

  const handleResize = () => {
    const canvas = canvasRef;
    if (canvas) {
      const nextSize = { width: canvas.clientWidth, height: canvas.clientHeight };
      setCanvasSize(nextSize);
      setPanels((prev) => {
        let changed = false;
        const next = prev.map((panel) => {
          const constrained = constrainPanelPosition(panel, nextSize);
          if (constrained !== panel) changed = true;
          return constrained;
        });
        return changed ? next : prev;
      });
      window.requestAnimationFrame(() => syncAllThumbnailRects("resize"));
    }
  };

  const restoreSavedPanels = async (savedPanels: PanelState[]) => {
    const restored: PanelState[] = [];

    for (const panel of savedPanels) {
      if (panel.type === "tool") {
        restored.push(constrainPanelPosition(withLocalizedToolTitle({ ...panel, visible: true })));
        continue;
      }

      if (!panel.sourceHwnd) continue;

      let panelId: string | null = null;
      try {
        const thumbnail = await addThumbnail(panel.sourceHwnd);
        panelId = thumbnail.panelId;
        const restoredPanel = constrainPanelPosition({
          ...panel,
          ...getThumbnailPanelSize(thumbnail, panel.width),
          id: panelId,
          visible: true,
        });
        restored.push(restoredPanel);
      } catch (e) {
        if (panelId) {
          try {
            await removePanel(panelId);
          } catch (cleanupError) {
            console.error("Failed to clean up restored thumbnail:", cleanupError);
          }
        }
        console.warn("Skipped stale thumbnail panel:", panel.title, e);
        showNotice(t("app.toast.stalePanelSkipped", { title: panel.title }), "info");
      }
    }

    return restored;
  };

  onMount(() => {
    const cleanupFns: Array<() => void> = [];
    let disposed = false;
    const addCleanup = (fn: () => void) => {
      if (disposed) {
        fn();
      } else {
        cleanupFns.push(fn);
      }
    };

    window.addEventListener("resize", handleResize);
    window.addEventListener("keydown", handleKeyDown);
    handleResize();

    (async () => {
      try {
        const saved = await loadLayout();
        if (saved.length > 0) {
          setPanels(await restoreSavedPanels(saved));
          window.requestAnimationFrame(() => {
            syncAllThumbnailRects("restore");
            syncThumbnailStackOrder(panels(), "restore");
          });
        }
      } catch (e) {
        console.warn("Failed to load layout:", e);
        showNotice(t("app.toast.loadLayoutFailed", { reason: errorMessage(e) }));
      } finally {
        setLayoutReady(true);
      }

      try {
        addCleanup(await listen("tray:new-panel", () => {
          setIsDialogOpen(true);
        }));
      } catch (error) {
        handleEventListenerError("tray:new-panel", error);
      }

      try {
        addCleanup(await listen<string>("tray:launch-tool", (event) => {
          addToolPanel(event.payload);
        }));
      } catch (error) {
        handleEventListenerError("tray:launch-tool", error);
      }

      try {
        addCleanup(await listen("tray:capture-hotkey", async () => {
          try {
            const windowInfo = await captureFocusedWindow();
            if (windowInfo) {
              await addThumbnailPanel(windowInfo.hwnd, windowInfo.title);
            }
          } catch (e) {
            console.error("Failed to capture focused window:", e);
            showNotice(t("app.toast.captureFailed", { reason: errorMessage(e) }));
          }
        }));
      } catch (error) {
        handleEventListenerError("tray:capture-hotkey", error);
      }

      try {
        addCleanup(await listen<SourceClosedPayload>(
          "thumb:source-closed",
          (event) => {
            removeClosedSourcePanel(event.payload.panelId, event.payload.sourceHwnd);
          }
        ));
      } catch (error) {
        handleEventListenerError("thumb:source-closed", error);
      }

      try {
        addCleanup(await listen<DragEnteredWorkbenchPayload>(
          "drag:entered-workbench",
          async (event) => {
            const position = workbenchClientPositionToCanvas(event.payload);
            const panel = await addThumbnailPanel(
              event.payload.sourceHwnd,
              event.payload.title,
              position
            );
            if (panel) {
              setDraggedExternalPanelId(panel.id);
            }
          }
        ));
      } catch (error) {
        handleEventListenerError("drag:entered-workbench", error);
      }

      try {
        addCleanup(await listen<DragPositionPayload>(
          "drag:moved-workbench",
          (event) => {
            const panelId = draggedExternalPanelId();
            if (!panelId) return;
            const panel = panels().find((p) => p.id === panelId);
            if (panel?.sourceHwnd !== event.payload.sourceHwnd) return;

            movePanelToPosition(panelId, workbenchClientPositionToCanvas(event.payload));
          }
        ));
      } catch (error) {
        handleEventListenerError("drag:moved-workbench", error);
      }

      try {
        addCleanup(await listen<DragPositionPayload>(
          "drag:ended-workbench",
          (event) => {
            const panelId = draggedExternalPanelId();
            if (!panelId) return;
            const panel = panels().find((p) => p.id === panelId);
            if (panel?.sourceHwnd === event.payload.sourceHwnd) {
              movePanelToPosition(panelId, workbenchClientPositionToCanvas(event.payload));
            }
            setDraggedExternalPanelId(null);
          }
        ));
      } catch (error) {
        handleEventListenerError("drag:ended-workbench", error);
      }
    })();

    const thumbnailHealthTimer = window.setInterval(
      () => syncAllThumbnailRects("health-check"),
      THUMBNAIL_HEALTH_INTERVAL_MS
    );

    onCleanup(() => {
      disposed = true;
      window.removeEventListener("resize", handleResize);
      window.removeEventListener("keydown", handleKeyDown);
      window.clearInterval(thumbnailHealthTimer);
      cleanupFns.forEach((cleanup) => cleanup());
      if (saveTimer !== undefined) {
        window.clearTimeout(saveTimer);
      }
      if (thumbnailSyncFrame !== undefined) {
        window.cancelAnimationFrame(thumbnailSyncFrame);
      }
      if (layoutReady()) {
        saveLayout(panels())
          .then(markSaveLayoutSuccess)
          .catch((error) => reportSaveLayoutFailure("cleanup", error));
      }
      noticeTimers.forEach((timer) => window.clearTimeout(timer));
    });
  });

  let lastToolTitleLang = lang();
  createEffect(() => {
    const currentLang = lang();
    if (currentLang === lastToolTitleLang) return;
    lastToolTitleLang = currentLang;
    syncToolPanelTitles();
  });

  return (
    <div class="workbench-container">
      <header class="workbench-header">
        <h1>{t("app.title")}</h1>
        <div class="header-actions">
          <button class="btn btn-secondary btn-small" onClick={() => void handleOpenSettings()}>
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
        onClick={handleCanvasClick}
        onContextMenu={handleCanvasContextMenu}
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
                isSelected={selectedPanelId() === panel.id}
                onDragStart={handleDragStart}
                onSelect={handleSelectPanel}
                onFocus={handleFocusPanel}
                onClose={handleClosePanel}
                onTop={handleTop}
              />
            ) : (
              <ToolPanel
                panel={panel}
                isDragging={draggingId() === panel.id}
                isSelected={selectedPanelId() === panel.id}
                onDragStart={handleDragStart}
                onSelect={handleSelectPanel}
                onFocus={handleFocusPanel}
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
                left: guide.type === "vertical" ? `${guide.position}px` : "0px",
                top: guide.type === "horizontal" ? `${guide.position}px` : "0px",
                width: guide.type === "vertical" ? "1px" : "100%",
                height: guide.type === "horizontal" ? "1px" : "100%",
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

        <Show when={contextMenu()}>
          {(menu) => (
            <div
              class="canvas-context-menu"
              style={{ left: `${menu().x}px`, top: `${menu().y}px` }}
              onClick={(e) => e.stopPropagation()}
            >
              <button type="button" onClick={handleContextAddPanel}>
                {t("app.addPanel")}
              </button>
              <button type="button" onClick={handleContextSaveLayout}>
                {t("app.saveLayout")}
              </button>
            </div>
          )}
        </Show>
      </div>

      <div class="toast-stack" aria-live="polite">
        <For each={notices()}>
          {(notice) => (
            <div class={`toast toast-${notice.type}`} role={notice.type === "error" ? "alert" : "status"}>
              {notice.message}
            </div>
          )}
        </For>
      </div>

      <footer class="workbench-footer">
        <span>{t("app.panelCount", { count: panels().length })}</span>
      </footer>

      <AddPanelDialog
        isOpen={isDialogOpen()}
        onClose={() => setIsDialogOpen(false)}
        onAddThumbnails={handleAddThumbnails}
        onAddTool={addToolPanel}
        onError={(error) =>
          showNotice(t("app.toast.enumerateWindowsFailed", { reason: errorMessage(error) }))
        }
      />
    </div>
  );
}

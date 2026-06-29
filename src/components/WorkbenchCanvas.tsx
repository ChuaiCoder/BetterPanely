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
  captureWindowUnderCursor,
  focusSource,
  removePanel,
  updateThumbnailRect,
  loadLayout,
  saveLayout,
} from "../lib/workbench-api";
import type { PanelState, SnapGuide, WindowInfo } from "../lib/types";

const PANEL_HEADER_HEIGHT = 32;
const NOTICE_TIMEOUT_MS = 5000;
const THUMBNAIL_HEALTH_INTERVAL_MS = 30000;

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

/**
 * 工作台主画布组件
 * 管理所有面板的布局、拖拽、磁性吸附、状态持久化等核心功能
 */
export function WorkbenchCanvas() {
  const { t } = useI18n();
  const [panels, setPanels] = createSignal<PanelState[]>([]);
  const [draggingId, setDraggingId] = createSignal<string | null>(null);
  const [dragOffset, setDragOffset] = createSignal({ x: 0, y: 0 });
  const [snapGuides, setSnapGuides] = createSignal<SnapGuide[]>([]);
  const [isDialogOpen, setIsDialogOpen] = createSignal(false);
  const [canvasSize, setCanvasSize] = createSignal({ width: 800, height: 600 });
  const [layoutReady, setLayoutReady] = createSignal(false);
  const [selectedPanelId, setSelectedPanelId] = createSignal<string | null>(null);
  const [notices, setNotices] = createSignal<WorkbenchNotice[]>([]);
  const [contextMenu, setContextMenu] = createSignal<CanvasContextMenu | null>(null);
  const [draggedExternalPanelId, setDraggedExternalPanelId] = createSignal<string | null>(null);

  let canvasRef!: HTMLDivElement;
  let saveTimer: number | undefined;
  const noticeTimers: number[] = [];

  const getNextZIndex = () => {
    const max = panels().reduce((acc, p) => Math.max(acc, p.zIndex), 0);
    return max + 1;
  };

  const clamp = (value: number, min: number, max: number) =>
    Math.min(Math.max(value, min), max);

  const errorMessage = (error: unknown) =>
    error instanceof Error ? error.message : String(error);

  const showNotice = (message: string, type: WorkbenchNotice["type"] = "error") => {
    const id = Date.now() + noticeTimers.length;
    setNotices((prev) => [...prev, { id, type, message }]);

    const timer = window.setTimeout(() => {
      setNotices((prev) => prev.filter((notice) => notice.id !== id));
    }, NOTICE_TIMEOUT_MS);
    noticeTimers.push(timer);
  };

  const getThumbnailRect = (panel: PanelState) => {
    const canvasRect = canvasRef?.getBoundingClientRect();
    return {
      x: panel.x + (canvasRect?.left ?? 0),
      y: panel.y + (canvasRect?.top ?? 0) + PANEL_HEADER_HEIGHT,
      width: panel.width,
      height: Math.max(1, panel.height - PANEL_HEADER_HEIGHT),
    };
  };

  const removeClosedSourcePanel = (panelId: string, sourceHwnd: number) => {
    const panel = panels().find(
      (p) => p.id === panelId || (p.type === "thumbnail" && p.sourceHwnd === sourceHwnd)
    );
    if (!panel) return;

    if (selectedPanelId() === panel.id) {
      setSelectedPanelId(null);
    }
    if (draggedExternalPanelId() === panel.id) {
      setDraggedExternalPanelId(null);
    }
    setPanels((prev) => prev.filter((p) => p.id !== panel.id));
    showNotice(t("app.toast.sourceClosed", { title: panel.title }), "info");
  };

  const syncThumbnailRect = async (panel: PanelState) => {
    if (panel.type !== "thumbnail" || !panel.sourceHwnd) return true;
    const rect = getThumbnailRect(panel);
    try {
      await updateThumbnailRect(panel.id, rect.x, rect.y, rect.width, rect.height);
      return true;
    } catch (e) {
      const message = String(e).toLowerCase();
      if (
        message.includes("source window is no longer available") ||
        message.includes("thumbnail not found")
      ) {
        removeClosedSourcePanel(panel.id, panel.sourceHwnd);
        return false;
      }
      throw e;
    }
  };

  const syncAllThumbnailRects = () => {
    panels().forEach((panel) => {
      if (panel.type === "thumbnail") {
        void syncThumbnailRect(panel).catch(console.error);
      }
    });
  };

  const getPanelInitialPosition = (
    width: number,
    height: number,
    initialPosition?: PanelInitialPosition
  ) => {
    if (!initialPosition) {
      return {
        x: 100 + panels().length * 20,
        y: 100 + panels().length * 20,
      };
    }

    return {
      x: clamp(initialPosition.x - width / 2, 8, Math.max(8, canvasSize().width - width - 8)),
      y: clamp(initialPosition.y - height / 2, 8, Math.max(8, canvasSize().height - height - 8)),
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
      void syncThumbnailRect(movedPanel).catch(console.error);
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
      saveLayout(snapshot).catch(console.error);
    }, 400);
  });

  const addThumbnailPanel = async (
    hwnd: number,
    title: string,
    initialPosition?: PanelInitialPosition
  ): Promise<PanelState | null> => {
    try {
      const panelId = await addThumbnail(hwnd);
      const width = 200;
      const height = 150;
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
      await syncThumbnailRect(newPanel);
      return newPanel;
    } catch (e) {
      console.error("Failed to add thumbnail:", e);
      showNotice(t("app.toast.addThumbnailFailed", { reason: errorMessage(e) }));
      return null;
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

  const handleAddThumbnails = (windows: WindowInfo[]) => {
    windows.forEach((windowInfo) => {
      void addThumbnailPanel(windowInfo.hwnd, windowInfo.title);
    });
  };

  const handleClosePanel = async (panelId: string) => {
    try {
      await removePanel(panelId);
      setPanels((prev) => prev.filter((p) => p.id !== panelId));
      if (selectedPanelId() === panelId) {
        setSelectedPanelId(null);
      }
    } catch (e) {
      console.error("Failed to remove panel:", e);
      showNotice(t("app.toast.removePanelFailed", { reason: errorMessage(e) }));
    }
  };

  const handleSelectPanel = (panelId: string) => {
    setSelectedPanelId(panelId);
  };

  const handleTop = (panelId: string) => {
    setPanels((prev) =>
      prev.map((p) => (p.id === panelId ? { ...p, zIndex: getNextZIndex() } : p))
    );
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
      .then(() => showNotice(t("app.toast.layoutSaved"), "success"))
      .catch((error) => {
        console.error("Failed to save layout:", error);
        showNotice(t("app.toast.saveLayoutFailed", { reason: errorMessage(error) }));
      });

  const focusPanel = async (panel: PanelState) => {
    if (panel.type === "thumbnail" && panel.sourceHwnd) {
      try {
        await focusSource(panel.sourceHwnd);
      } catch (e) {
        console.error("Failed to focus source window:", e);
        showNotice(t("app.toast.focusFailed", { reason: errorMessage(e) }));
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

    setPanels((prev) => {
      const otherPanels = prev.filter((p) => p.id !== activePanelId);
      const draggedPanel = prev.find((p) => p.id === activePanelId);

      if (!draggedPanel) return prev;

      const draggedRect = { x: newX, y: newY, width: draggedPanel.width, height: draggedPanel.height };
      const snapResult = snap(draggedRect, otherPanels, canvasSize().width, canvasSize().height);

      setSnapGuides(snapResult.guides);

      const movedPanel = { ...draggedPanel, x: snapResult.rect.x, y: snapResult.rect.y };
      if (movedPanel.type === "thumbnail") {
        void syncThumbnailRect(movedPanel).catch(console.error);
      }

      return prev.map((p) => (p.id === activePanelId ? movedPanel : p));
    });
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

  const handleMouseUp = async () => {
    if (!draggingId()) return;

    const panel = panels().find((p) => p.id === draggingId());
    if (panel && panel.type === "thumbnail" && panel.sourceHwnd) {
      try {
        await syncThumbnailRect(panel);
      } catch (e) {
        console.error("Failed to update thumbnail rect:", e);
      }
    }

    setDraggingId(null);
    setSnapGuides([]);
  };

  const handleResize = () => {
    const canvas = canvasRef;
    if (canvas) {
      setCanvasSize({ width: canvas.clientWidth, height: canvas.clientHeight });
      window.requestAnimationFrame(syncAllThumbnailRects);
    }
  };

  const restoreSavedPanels = async (savedPanels: PanelState[]) => {
    const restored: PanelState[] = [];

    for (const panel of savedPanels) {
      if (panel.type === "tool") {
        restored.push({ ...panel, visible: true });
        continue;
      }

      if (!panel.sourceHwnd) continue;

      try {
        const panelId = await addThumbnail(panel.sourceHwnd);
        const restoredPanel = { ...panel, id: panelId, visible: true };
        restored.push(restoredPanel);
        await syncThumbnailRect(restoredPanel);
      } catch (e) {
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
        }
      } catch (e) {
        console.warn("Failed to load layout:", e);
        showNotice(t("app.toast.loadLayoutFailed", { reason: errorMessage(e) }));
      } finally {
        setLayoutReady(true);
      }

      const unlistenNewPanel = await listen("tray:new-panel", () => {
        setIsDialogOpen(true);
      });
      const unlistenLaunchTool = await listen<string>("tray:launch-tool", (event) => {
        addToolPanel(event.payload);
      });
      const unlistenCaptureHotkey = await listen("tray:capture-hotkey", async () => {
        try {
          const windowInfo = await captureWindowUnderCursor();
          if (windowInfo) {
            await addThumbnailPanel(windowInfo.hwnd, windowInfo.title);
          }
        } catch (e) {
          console.error("Failed to capture window under cursor:", e);
          showNotice(t("app.toast.captureFailed", { reason: errorMessage(e) }));
        }
      });
      const unlistenSourceClosed = await listen<SourceClosedPayload>(
        "thumb:source-closed",
        (event) => {
          removeClosedSourcePanel(event.payload.panelId, event.payload.sourceHwnd);
        }
      );
      const unlistenDragEntered = await listen<DragEnteredWorkbenchPayload>(
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
      );
      const unlistenDragMoved = await listen<DragPositionPayload>(
        "drag:moved-workbench",
        (event) => {
          const panelId = draggedExternalPanelId();
          if (!panelId) return;
          const panel = panels().find((p) => p.id === panelId);
          if (panel?.sourceHwnd !== event.payload.sourceHwnd) return;

          movePanelToPosition(panelId, workbenchClientPositionToCanvas(event.payload));
        }
      );
      const unlistenDragEnded = await listen<DragPositionPayload>(
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
      );

      addCleanup(unlistenNewPanel);
      addCleanup(unlistenLaunchTool);
      addCleanup(unlistenCaptureHotkey);
      addCleanup(unlistenSourceClosed);
      addCleanup(unlistenDragEntered);
      addCleanup(unlistenDragMoved);
      addCleanup(unlistenDragEnded);
    })();

    const thumbnailHealthTimer = window.setInterval(
      syncAllThumbnailRects,
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
      if (layoutReady()) {
        saveLayout(panels()).catch((error) => {
          console.error("Failed to save layout:", error);
        });
      }
      noticeTimers.forEach((timer) => window.clearTimeout(timer));
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

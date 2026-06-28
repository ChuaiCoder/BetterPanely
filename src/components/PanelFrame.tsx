import { createSignal, onMount, onCleanup } from "solid-js";
import type { Panel } from "../lib/types";
import {
  movePanel,
  resizePanel,
  setPanelAlwaysOnTop,
} from "../lib/panel-api";
import { useI18n } from "../lib/i18n";

interface PanelFrameProps {
  panel: Panel;
  onClose: () => void;
}

export function PanelFrame(props: PanelFrameProps) {
  const { t } = useI18n();
  const [isDragging, setIsDragging] = createSignal(false);
  const [isResizing, setIsResizing] = createSignal(false);
  let dragOffset = { x: 0, y: 0 };
  let frameRef!: HTMLDivElement;

  function onMouseDown(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains("panel-titlebar")) {
      setIsDragging(true);
      dragOffset = {
        x: e.clientX - props.panel.x,
        y: e.clientY - props.panel.y,
      };
      e.preventDefault();
    }
  }

  function onMouseMove(e: MouseEvent) {
    if (isDragging()) {
      const newX = e.clientX - dragOffset.x;
      const newY = e.clientY - dragOffset.y;
      movePanel(props.panel.id, newX, newY).catch(console.error);
    }
    if (isResizing()) {
      const rect = frameRef.getBoundingClientRect();
      const newW = Math.max(200, e.clientX - rect.left);
      const newH = Math.max(150, e.clientY - rect.top);
      resizePanel(props.panel.id, newW, newH).catch(console.error);
    }
  }

  function onMouseUp() {
    setIsDragging(false);
    setIsResizing(false);
  }

  onMount(() => {
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  });

  onCleanup(() => {
    window.removeEventListener("mousemove", onMouseMove);
    window.removeEventListener("mouseup", onMouseUp);
  });

  return (
    <div
      ref={frameRef}
      class="panel-frame"
      style={{
        left: `${props.panel.x}px`,
        top: `${props.panel.y}px`,
        width: `${props.panel.width}px`,
        height: `${props.panel.height}px`,
        opacity: props.panel.opacity,
        "pointer-events": props.panel.clickThrough ? "none" : "auto",
      }}
      onMouseDown={onMouseDown}
    >
      <div class="panel-titlebar">
        <span class="panel-title">{props.panel.title}</span>
        <div class="panel-titlebar-actions">
          <button
            class="panel-btn"
            onClick={() =>
              setPanelAlwaysOnTop(props.panel.id, !props.panel.alwaysOnTop)
            }
            title={t("panelFrame.alwaysOnTop")}
          >
            📌
          </button>
          <button class="panel-btn" onClick={props.onClose} title={t("panelFrame.close")}>
            ✕
          </button>
        </div>
      </div>
      <div class="panel-content">
        <slot />
      </div>
      <div
        class="panel-resize-handle"
        onMouseDown={(e) => {
          setIsResizing(true);
          e.preventDefault();
          e.stopPropagation();
        }}
      />
    </div>
  );
}

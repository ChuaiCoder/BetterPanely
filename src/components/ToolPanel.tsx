import { createSignal, Show } from "solid-js";
import { useI18n } from "../lib/i18n";
import type { PanelState } from "../lib/types";

interface ToolPanelProps {
  panel: PanelState;
  isDragging: boolean;
  isSelected: boolean;
  onDragStart: (id: string, offsetX: number, offsetY: number) => void;
  onSelect: (id: string) => void;
  onFocus: (id: string) => void;
  onClose: (id: string) => void;
  onTop: (id: string) => void;
}

/**
 * 工具面板组件
 * 使用 iframe 嵌入内置工具（计算器、笔记、计时器、天气），支持拖拽和置顶
 */
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
    if (
      (e.target as HTMLElement).closest(".panel-close") ||
      (e.target as HTMLElement).closest(".panel-focus") ||
      (e.target as HTMLElement).closest(".panel-top")
    ) {
      return;
    }

    const card = (e.currentTarget as HTMLElement).closest(".panel-card");
    if (!card) return;

    const rect = card.getBoundingClientRect();
    const offsetX = e.clientX - rect.left;
    const offsetY = e.clientY - rect.top;
    props.onSelect(props.panel.id);
    props.onDragStart(props.panel.id, offsetX, offsetY);
  };

  const handleTop = () => {
    props.onTop(props.panel.id);
  };

  const handleFocus = () => {
    props.onSelect(props.panel.id);
    props.onFocus(props.panel.id);
  };

  return (
    <div
      class={`panel-card ${props.isDragging ? "panel-dragging" : ""} ${props.isSelected ? "panel-selected" : ""} ${isHovered() ? "panel-hovered" : ""}`}
      style={{
        left: `${props.panel.x}px`,
        top: `${props.panel.y}px`,
        width: `${props.panel.width}px`,
        height: `${props.panel.height}px`,
        "z-index": props.panel.zIndex,
      }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <div class="panel-header" onMouseDown={handleMouseDown}>
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
            title={t("app.openToolWindow")}
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

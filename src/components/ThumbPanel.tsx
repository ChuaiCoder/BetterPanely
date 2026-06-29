import { createSignal, Show } from "solid-js";
import { useI18n } from "../lib/i18n";
import type { PanelState } from "../lib/types";

interface ThumbPanelProps {
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
 * 缩略图面板组件
 * 展示外部窗口的实时缩略图，支持拖拽、聚焦源窗口、置顶等操作
 */
export function ThumbPanel(props: ThumbPanelProps) {
  const { t } = useI18n();
  const [isHovered, setIsHovered] = createSignal(false);

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

  const handleFocus = () => {
    props.onSelect(props.panel.id);
    props.onFocus(props.panel.id);
  };

  const handleTop = () => {
    props.onTop(props.panel.id);
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
      <div
        class="panel-content panel-content-transparent panel-content-focusable"
        onClick={handleFocus}
      >
        <Show when={!props.panel.sourceHwnd}>
          <div class="panel-placeholder">
            <p>{t("app.panelPlaceholder")}</p>
          </div>
        </Show>
      </div>
    </div>
  );
}

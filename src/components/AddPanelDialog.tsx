import { createSignal, createEffect, For, Show } from "solid-js";
import { useI18n } from "../lib/i18n";
import { enumerateWindows } from "../lib/workbench-api";
import type { WindowInfo, ToolDefinition } from "../lib/types";

interface AddPanelDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onAddThumbnails: (windows: WindowInfo[]) => void;
  onAddTool: (toolId: string) => void;
}

const TOOLS: ToolDefinition[] = [
  { id: "calculator", name: "Calculator", description: "Calculator tool", icon: "🔢", defaultWidth: 280, defaultHeight: 420, url: "src/tools/calculator/index.html" },
  { id: "notes", name: "Notes", description: "Notes tool", icon: "📝", defaultWidth: 350, defaultHeight: 400, url: "src/tools/notes/index.html" },
  { id: "timer", name: "Timer", description: "Timer tool", icon: "⏱️", defaultWidth: 300, defaultHeight: 200, url: "src/tools/timer/index.html" },
  { id: "weather", name: "Weather", description: "Weather tool", icon: "🌤️", defaultWidth: 300, defaultHeight: 350, url: "src/tools/weather/index.html" },
];

/**
 * 添加面板对话框组件
 * 提供桌面窗口列表选择和内置工具快捷添加功能
 */
export function AddPanelDialog(props: AddPanelDialogProps) {
  const { t } = useI18n();
  const [windows, setWindows] = createSignal<WindowInfo[]>([]);
  const [selectedHwnds, setSelectedHwnds] = createSignal<Set<number>>(new Set());
  const [searchQuery, setSearchQuery] = createSignal("");

  createEffect(() => {
    if (props.isOpen) {
      enumerateWindows().then(setWindows).catch(console.error);
      setSelectedHwnds(new Set<number>());
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

  const getIncompatibilityReason = (windowInfo: WindowInfo) =>
    windowInfo.incompatibilityReason || t("app.windowNotCapturable");

  const toggleWindow = (windowInfo: WindowInfo) => {
    if (!windowInfo.isCompatible) return;

    const newSet = new Set(selectedHwnds());
    const hwnd = windowInfo.hwnd;
    if (newSet.has(hwnd)) {
      newSet.delete(hwnd);
    } else {
      newSet.add(hwnd);
    }
    setSelectedHwnds(newSet);
  };

  const handleAddWindows = () => {
    const selected = windows().filter((w) => selectedHwnds().has(w.hwnd) && w.isCompatible);
    if (selected.length === 0) return;

    props.onAddThumbnails(selected);
    props.onClose();
  };

  const handleAddTool = (toolId: string) => {
    props.onAddTool(toolId);
    props.onClose();
  };

  return (
    <Show when={props.isOpen}>
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
                  <label
                    class={`window-item${w.isCompatible ? "" : " window-item-disabled"}`}
                    title={w.isCompatible ? undefined : getIncompatibilityReason(w)}
                  >
                    <input
                      type="checkbox"
                      checked={w.isCompatible && selectedHwnds().has(w.hwnd)}
                      disabled={!w.isCompatible}
                      onChange={() => toggleWindow(w)}
                    />
                    <span class="window-main">
                      <span class="window-title">{w.title}</span>
                      <Show when={!w.isCompatible}>
                        <span class="window-incompatible-reason">
                          {getIncompatibilityReason(w)}
                        </span>
                      </Show>
                    </span>
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
    </Show>
  );
}

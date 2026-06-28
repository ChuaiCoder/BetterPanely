import { createSignal, onMount, For, Show } from "solid-js";
import { enumerateWindows } from "../lib/panel-api";
import { useI18n } from "../lib/i18n";
import type { WindowInfo } from "../lib/types";

interface WindowPickerProps {
  onSelect: (hwnd: number) => void;
  onClose: () => void;
}

export function WindowPicker(props: WindowPickerProps) {
  const { t } = useI18n();
  const [windows, setWindows] = createSignal<WindowInfo[]>([]);
  const [search, setSearch] = createSignal("");
  const [loading, setLoading] = createSignal(true);

  onMount(async () => {
    try {
      const list = await enumerateWindows();
      setWindows(list);
    } catch (e) {
      console.error("Failed to enumerate windows:", e);
    } finally {
      setLoading(false);
    }
  });

  const filtered = () => {
    const q = search().toLowerCase();
    if (!q) return windows();
    return windows().filter(
      (w) =>
        w.title.toLowerCase().includes(q) ||
        w.exePath.toLowerCase().includes(q) ||
        w.className.toLowerCase().includes(q)
    );
  };

  function handleSelect(win: WindowInfo) {
    if (win.isCompatible) {
      props.onSelect(win.hwnd);
    }
  }

  function getIncompatReason(win: WindowInfo): string {
    const r = win.incompatibilityReason;
    if (!r) return t("windowPicker.incompatible");
    // Map known reasons to locale keys
    const keyMap: Record<string, string> = {
      "UWP apps cannot be embedded": "incompatibility.uwp",
      "System shell window": "incompatibility.shell",
      "BetterPanely window": "incompatibility.self",
      "Already a child window": "incompatibility.child",
      "Administrator process (UIPI blocked)": "incompatibility.elevated",
      "Fullscreen / DirectX application": "incompatibility.fullscreen",
    };
    const key = keyMap[r] ?? r;
    return key.startsWith("incompatibility.") ? t(key) : r;
  }

  return (
    <div class="modal-overlay" onClick={props.onClose}>
      <div class="modal-content" onClick={(e) => e.stopPropagation()}>
        <div class="modal-header">
          <h3>{t("windowPicker.title")}</h3>
          <button class="modal-close" onClick={props.onClose}>
            ✕
          </button>
        </div>
        <div class="modal-search">
          <input
            type="text"
            placeholder={t("windowPicker.searchPlaceholder")}
            value={search()}
            onInput={(e) => setSearch(e.currentTarget.value)}
          />
        </div>
        <div class="modal-list">
          <Show when={!loading()} fallback={<p style="padding:20px;text-align:center;color:var(--text-secondary)">{t("windowPicker.loading")}</p>}>
            <Show
              when={filtered().length > 0}
              fallback={
                <p style="padding:20px;text-align:center;color:var(--text-secondary)">
                  {t("windowPicker.noWindows")}
                </p>
              }
            >
              <For each={filtered()}>
                {(win) => (
                  <div
                    class={`window-item ${!win.isCompatible ? "incompatible" : ""}`}
                    onClick={() => handleSelect(win)}
                  >
                    <span class="window-item-icon">
                      {win.isCompatible ? "🪟" : "🚫"}
                    </span>
                    <div class="window-item-info">
                      <span class="window-item-title">
                        {win.title || t("windowPicker.noTitle")}
                      </span>
                      <span class="window-item-exe">{win.exePath}</span>
                    </div>
                    <span
                      class={`window-item-badge ${win.isCompatible ? "badge-compatible" : "badge-incompatible"}`}
                    >
                      {win.isCompatible
                        ? t("windowPicker.ok")
                        : getIncompatReason(win)}
                    </span>
                  </div>
                )}
              </For>
            </Show>
          </Show>
        </div>
      </div>
    </div>
  );
}

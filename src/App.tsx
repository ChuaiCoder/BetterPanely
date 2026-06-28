import { createSignal, onMount, onCleanup, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import {
  listPanels,
  createPanel,
  destroyPanel,
  launchTool,
  embedWindow,
  releaseWindow,
  startDragCapture,
  stopDragCapture,
  onPanelCreated,
  onPanelDestroyed,
} from "./lib/panel-api";
import type { Panel } from "./lib/types";
import { useI18n, type Lang } from "./lib/i18n";
import { WindowPicker } from "./components/WindowPicker";
import "./App.css";

function App() {
  const { t, lang, setLang } = useI18n();
  const [panels, setPanels] = createSignal<Panel[]>([]);
  const [showWindowPicker, setShowWindowPicker] = createSignal(false);
  const [captureActive, setCaptureActive] = createSignal(false);
  const [targetPanelId, setTargetPanelId] = createSignal<string | null>(null);

  let unlistenCreated: (() => void) | null = null;
  let unlistenDestroyed: (() => void) | null = null;

  onMount(async () => {
    try {
      const existing = await listPanels();
      setPanels(existing);
    } catch (e) {
      console.error("Failed to list panels:", e);
    }

    unlistenCreated = await onPanelCreated((panel) => {
      setPanels((prev) => [...prev, panel]);
    });

    unlistenDestroyed = await onPanelDestroyed((panelId) => {
      setPanels((prev) => prev.filter((p) => p.id !== panelId));
    });
  });

  onCleanup(() => {
    unlistenCreated?.();
    unlistenDestroyed?.();
  });

  async function handleNewPanel() {
    try {
      await createPanel(t("app.newPanel"), { type: "embedded", embedInfo: null as any });
    } catch (e) {
      console.error("Failed to create panel:", e);
    }
  }

  async function handleLaunchTool(toolId: string) {
    try {
      await launchTool(toolId);
    } catch (e) {
      console.error("Failed to launch tool:", e);
    }
  }

  async function handleDestroyPanel(panelId: string) {
    try {
      await destroyPanel(panelId);
    } catch (e) {
      console.error("Failed to destroy panel:", e);
    }
  }

  function handleOpenWindowPicker(panelId: string) {
    setTargetPanelId(panelId);
    setShowWindowPicker(true);
  }

  async function handleEmbedWindow(sourceHwnd: number) {
    const pid = targetPanelId();
    if (!pid) return;
    try {
      await embedWindow(pid, sourceHwnd);
      setShowWindowPicker(false);
    } catch (e) {
      console.error("Failed to embed window:", e);
    }
  }

  async function handleRelease(panelId: string) {
    try {
      await releaseWindow(panelId);
    } catch (e) {
      console.error("Failed to release window:", e);
    }
  }

  async function handleToggleCapture() {
    if (captureActive()) {
      await stopDragCapture();
      setCaptureActive(false);
    } else {
      await startDragCapture();
      setCaptureActive(true);
    }
  }

  function toggleLang() {
    const next: Lang = lang() === "en" ? "zh" : "en";
    setLang(next);
  }

  return (
    <div class="app-container">
      <header class="app-header">
        <h1>{t("app.title")}</h1>
        <div class="header-actions">
          <button class="btn btn-secondary btn-small" onClick={() => invoke("open_settings")}>
            ⚙
          </button>
          <button class="btn btn-lang" onClick={toggleLang} title={t("app.language")}>
            {t("app.langToggle")}
          </button>
          <button class="btn btn-primary" onClick={handleNewPanel}>
            {t("app.newPanel")}
          </button>
          <button
            class={`btn ${captureActive() ? "btn-danger" : "btn-secondary"}`}
            onClick={handleToggleCapture}
          >
            {captureActive() ? t("app.stopDragCapture") : t("app.startDragCapture")}
          </button>
        </div>
      </header>

      <section class="quick-tools">
        <h2>{t("app.quickTools")}</h2>
        <div class="tool-grid">
          <button class="tool-btn" onClick={() => handleLaunchTool("calculator")}>
            <span class="tool-icon">🔢</span>
            <span class="tool-label">{t("tools.calculator")}</span>
          </button>
          <button class="tool-btn" onClick={() => handleLaunchTool("notes")}>
            <span class="tool-icon">📝</span>
            <span class="tool-label">{t("tools.notes")}</span>
          </button>
          <button class="tool-btn" onClick={() => handleLaunchTool("timer")}>
            <span class="tool-icon">⏱️</span>
            <span class="tool-label">{t("tools.timer")}</span>
          </button>
          <button class="tool-btn" onClick={() => handleLaunchTool("weather")}>
            <span class="tool-icon">🌤️</span>
            <span class="tool-label">{t("tools.weather")}</span>
          </button>
        </div>
      </section>

      <section class="panels-list">
        <h2>{t("app.activePanels", { count: panels().length })}</h2>
        <Show
          when={panels().length > 0}
          fallback={<p class="empty-state">{t("app.noPanels")}</p>}
        >
          <For each={panels()}>
            {(panel) => (
              <div class="panel-card">
                <div class="panel-card-info">
                  <span class="panel-card-title">{panel.title}</span>
                  <span class="panel-card-type">
                    {panel.panelType.type === "tool"
                      ? t("app.panelType.tool", { toolId: panel.panelType.toolId })
                      : panel.panelType.embedInfo
                        ? t("app.panelType.embedded", { title: panel.panelType.embedInfo.sourceTitle })
                        : t("app.panelType.empty")}
                  </span>
                  <span class="panel-card-size">
                    {panel.width}×{panel.height} @ ({panel.x}, {panel.y})
                  </span>
                </div>
                <div class="panel-card-actions">
                  <Show
                    when={
                      panel.panelType.type === "embedded" &&
                      panel.panelType.embedInfo
                    }
                  >
                    <button
                      class="btn btn-small btn-warning"
                      onClick={() => handleRelease(panel.id)}
                    >
                      {t("app.release")}
                    </button>
                  </Show>
                  <Show when={panel.panelType.type === "embedded" && !panel.panelType.embedInfo}>
                    <button
                      class="btn btn-small btn-secondary"
                      onClick={() => handleOpenWindowPicker(panel.id)}
                    >
                      {t("app.embedWindow")}
                    </button>
                  </Show>
                  <button
                    class="btn btn-small btn-danger"
                    onClick={() => handleDestroyPanel(panel.id)}
                  >
                    {t("app.close")}
                  </button>
                </div>
              </div>
            )}
          </For>
        </Show>
      </section>

      <Show when={showWindowPicker()}>
        <WindowPicker
          onSelect={handleEmbedWindow}
          onClose={() => setShowWindowPicker(false)}
        />
      </Show>
    </div>
  );
}

export default App;

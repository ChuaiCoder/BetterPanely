/* @refresh reload */
import { render } from "solid-js/web";
import App from "./App";
import { createI18n } from "./lib/i18n.tsx";
import { getSettings, onSettingsChanged } from "./lib/settings-api";
import { applyAppTheme, watchSystemTheme } from "./lib/theme";
import type { Lang } from "./lib/i18n.tsx";

const SETTINGS_LISTENER_RETRY_MS = 1000;
const SETTINGS_LISTENER_MAX_ATTEMPTS = 3;

let settingsThemeFallbackInstalled = false;

async function loadSettingsForBootstrap(): Promise<Lang> {
  try {
    const saved = await getSettings();
    applyAppTheme(saved.theme);
    return saved.language === "zh" ? "zh" : "en";
  } catch (error) {
    console.error("Failed to load settings during bootstrap:", error);
    applyAppTheme("dark");
    return "en";
  }
}

async function refreshThemeFromSettings(context: string) {
  try {
    const saved = await getSettings();
    applyAppTheme(saved.theme);
  } catch (error) {
    console.error(`Failed to refresh theme from settings (${context}):`, error);
  }
}

function installSettingsThemeFallback() {
  if (settingsThemeFallbackInstalled) return;
  settingsThemeFallbackInstalled = true;

  window.addEventListener("focus", () => {
    void refreshThemeFromSettings("focus-fallback");
  });
}

function registerSettingsThemeListener(attempt = 1) {
  onSettingsChanged((settings) => applyAppTheme(settings.theme)).catch((error) => {
    console.error(`Failed to listen for settings changes (attempt ${attempt}):`, error);
    if (attempt < SETTINGS_LISTENER_MAX_ATTEMPTS) {
      window.setTimeout(
        () => registerSettingsThemeListener(attempt + 1),
        SETTINGS_LISTENER_RETRY_MS
      );
      return;
    }
    installSettingsThemeFallback();
  });
}

async function bootstrap() {
  const initialLang = await loadSettingsForBootstrap();

  watchSystemTheme();
  registerSettingsThemeListener();

  const { I18nProvider } = createI18n(initialLang);

  render(
    () => (
      <I18nProvider>
        <App />
      </I18nProvider>
    ),
    document.getElementById("root")!
  );
}

bootstrap();

/* @refresh reload */
import { render } from "solid-js/web";
import App from "./App";
import { createI18n } from "./lib/i18n.tsx";
import { getSettings, onSettingsChanged } from "./lib/settings-api";
import { applyAppTheme, watchSystemTheme } from "./lib/theme";
import type { Lang } from "./lib/i18n.tsx";

async function bootstrap() {
  let initialLang: Lang = "en";

  try {
    const saved = await getSettings();
    if (saved.language === "zh") initialLang = "zh";
    applyAppTheme(saved.theme);
  } catch {
    applyAppTheme("dark");
  }

  watchSystemTheme();
  onSettingsChanged((settings) => applyAppTheme(settings.theme)).catch(console.error);

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

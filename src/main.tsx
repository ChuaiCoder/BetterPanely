/* @refresh reload */
import { render } from "solid-js/web";
import App from "./App";
import { createI18n } from "./lib/i18n.tsx";
import { getLanguage } from "./lib/settings-api";
import type { Lang } from "./lib/i18n.tsx";

async function bootstrap() {
  // Get persisted language
  let initialLang: Lang = "en";
  try {
    const saved = await getLanguage();
    if (saved === "zh") initialLang = "zh";
  } catch {
    // Default to English if backend unavailable
  }

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

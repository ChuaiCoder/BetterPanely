import { createSignal, createContext, useContext } from "solid-js";
import type { JSX } from "solid-js";
import { setLanguage as setLangBackend, onLanguageChanged } from "./settings-api";
import enLocale from "./locales/en.json";
import zhLocale from "./locales/zh.json";

type LocaleData = Record<string, string>;

const locales: Record<string, LocaleData> = {
  en: enLocale as LocaleData,
  zh: zhLocale as LocaleData,
};

export type Lang = "en" | "zh";

export interface I18nContextValue {
  t: (key: string, params?: Record<string, string | number>) => string;
  lang: () => Lang;
  setLang: (lang: Lang) => Promise<void>;
}

const I18nContext = createContext<I18nContextValue>();

/**
 * Create the i18n instance. Must be called once at app startup.
 */
export function createI18n(initialLang: Lang) {
  const [lang, setLangSignal] = createSignal<Lang>(initialLang);

  function t(key: string, params?: Record<string, string | number>): string {
    const locale = locales[lang()];
    let value = locale?.[key] ?? locales.en[key] ?? key;
    if (params) {
      for (const [k, v] of Object.entries(params)) {
        value = value.replace(`{${k}}`, String(v));
      }
    }
    return value;
  }

  async function setLang(newLang: Lang) {
    await setLangBackend(newLang);
    setLangSignal(newLang);
  }

  onLanguageChanged((newLang: string) => {
    if (newLang === "en" || newLang === "zh") {
      setLangSignal(newLang as Lang);
    }
  });

  import("@tauri-apps/api/event").then(({ listen }) => {
    listen<string>("tray:set-language", (event) => {
      const newLang = event.payload;
      if (newLang === "en" || newLang === "zh") {
        setLang(newLang as Lang);
      }
    });
  });

  function I18nProvider(props: { children: JSX.Element }) {
    const value: I18nContextValue = { t, lang, setLang };
    return (
      <I18nContext.Provider value={value}>
        {props.children}
      </I18nContext.Provider>
    );
  }

  return { t, lang, setLang, I18nProvider };
}

/** Hook to access i18n functions. Must be used within an I18nProvider. */
export function useI18n(): I18nContextValue {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used within an I18nProvider");
  return ctx;
}

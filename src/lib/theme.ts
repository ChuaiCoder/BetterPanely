import type { AppSettings } from "./types";

type ThemePreference = AppSettings["theme"];
type ResolvedTheme = "dark" | "light";

let currentTheme: ThemePreference = "dark";

function resolveTheme(theme: ThemePreference): ResolvedTheme {
  if (theme === "system") {
    return window.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark";
  }

  return theme;
}

export function applyAppTheme(theme: ThemePreference) {
  currentTheme = theme;
  document.documentElement.dataset.themePreference = theme;
  document.documentElement.dataset.theme = resolveTheme(theme);
}

export function watchSystemTheme() {
  const media = window.matchMedia?.("(prefers-color-scheme: light)");
  if (!media) return;

  const handleChange = () => {
    if (currentTheme === "system") {
      applyAppTheme(currentTheme);
    }
  };

  media.addEventListener("change", handleChange);
}

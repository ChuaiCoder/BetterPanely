import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppSettings } from "./types";

export async function getSettings(): Promise<AppSettings> {
  return invoke("get_settings");
}

export async function getLanguage(): Promise<string> {
  return invoke("get_language");
}

export async function setLanguage(lang: string): Promise<string> {
  return invoke("set_language", { lang });
}

export function onLanguageChanged(
  callback: (lang: string) => void
): Promise<UnlistenFn> {
  return listen<string>("language-changed", (event) => callback(event.payload));
}

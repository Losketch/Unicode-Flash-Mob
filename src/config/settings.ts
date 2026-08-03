import { isHexColor } from "../utils/config";
import { loadStoredJson } from "../utils/storage";

export const SUPPORTED_LOCALES = ["en", "zh-CN"] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

export const THEME_MODES = ["light", "dark", "system"] as const;
export type ThemeMode = (typeof THEME_MODES)[number];

export interface AppSettings {
  locale: SupportedLocale;
  themeMode: ThemeMode;
  accentColor: string;
  outputDir: string;
  encoderAvc: string;
  encoderHevc: string;
}

export const SETTINGS_STORAGE_KEY = "ufm-settings";

export const DEFAULT_SETTINGS: AppSettings = {
  locale: "zh-CN",
  themeMode: "system",
  accentColor: "#6750A4",
  outputDir: "",
  encoderAvc: "",
  encoderHevc: "",
};

export const isSupportedLocale = (value: unknown): value is SupportedLocale =>
  typeof value === "string" &&
  SUPPORTED_LOCALES.some((locale) => locale === value);

export const isThemeMode = (value: unknown): value is ThemeMode =>
  typeof value === "string" && THEME_MODES.some((mode) => mode === value);

const storedString = (value: unknown, fallback: string): string =>
  typeof value === "string" ? value : fallback;

export function loadAppSettings(): AppSettings {
  const stored = loadStoredJson<Record<string, unknown>>(SETTINGS_STORAGE_KEY, {});
  const accentColor = storedString(
    stored.accentColor,
    DEFAULT_SETTINGS.accentColor
  );
  return {
    locale: isSupportedLocale(stored.locale)
      ? stored.locale
      : DEFAULT_SETTINGS.locale,
    themeMode: isThemeMode(stored.themeMode)
      ? stored.themeMode
      : DEFAULT_SETTINGS.themeMode,
    accentColor: isHexColor(accentColor)
      ? accentColor
      : DEFAULT_SETTINGS.accentColor,
    outputDir: storedString(stored.outputDir, DEFAULT_SETTINGS.outputDir),
    encoderAvc: storedString(stored.encoderAvc, DEFAULT_SETTINGS.encoderAvc),
    encoderHevc: storedString(stored.encoderHevc, DEFAULT_SETTINGS.encoderHevc),
  };
}

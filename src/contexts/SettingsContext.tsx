import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import {
  DEFAULT_SETTINGS,
  SETTINGS_STORAGE_KEY,
  loadAppSettings,
  type AppSettings,
  type SupportedLocale,
  type ThemeMode,
} from "../config/settings";
import { saveStoredJson } from "../utils/storage";

export type { AppSettings, ThemeMode } from "../config/settings";

interface SettingsContextValue {
  settings: AppSettings;
  setLocale: (locale: SupportedLocale) => void;
  setThemeMode: (mode: ThemeMode) => void;
  setAccentColor: (color: string) => void;
  setOutputDir: (path: string) => void;
  setEncoderAvc: (encoder: string) => void;
  setEncoderHevc: (encoder: string) => void;
  resetSettings: () => void;
  followSystem: boolean;
  setFollowSystem: (follow: boolean) => void;
  effectiveMode: "light" | "dark";
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

function getSystemMode(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export const SettingsProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const { i18n } = useTranslation();
  const [settings, setSettings] = useState<AppSettings>(loadAppSettings);
  const [systemMode, setSystemMode] = useState<"light" | "dark">(
    getSystemMode
  );

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) =>
      setSystemMode(e.matches ? "dark" : "light");
    media.addEventListener("change", handler);
    return () => media.removeEventListener("change", handler);
  }, []);

  useEffect(() => {
    saveStoredJson(SETTINGS_STORAGE_KEY, settings);
    if (i18n.language !== settings.locale) {
      i18n.changeLanguage(settings.locale);
    }
  }, [settings, i18n]);

  const effectiveMode = useMemo<"light" | "dark">(() => {
    if (settings.themeMode === "system") {
      return systemMode;
    }
    return settings.themeMode;
  }, [settings.themeMode, systemMode]);

  const followSystem = useMemo(
    () => settings.themeMode === "system",
    [settings.themeMode]
  );

  const update = useCallback((patch: Partial<AppSettings>) => {
    setSettings((prev) => ({ ...prev, ...patch }));
  }, []);

  const setLocale = useCallback(
    (locale: SupportedLocale) => update({ locale }),
    [update]
  );
  const setThemeMode = useCallback(
    (themeMode: ThemeMode) => update({ themeMode }),
    [update]
  );
  const setAccentColor = useCallback(
    (accentColor: string) => update({ accentColor }),
    [update]
  );
  const setOutputDir = useCallback(
    (outputDir: string) => update({ outputDir }),
    [update]
  );
  const setEncoderAvc = useCallback(
    (encoderAvc: string) => update({ encoderAvc }),
    [update]
  );
  const setEncoderHevc = useCallback(
    (encoderHevc: string) => update({ encoderHevc }),
    [update]
  );
  const resetSettings = useCallback(() => {
    setSettings(DEFAULT_SETTINGS);
  }, []);

  const setFollowSystem = useCallback(
    (follow: boolean) => {
      if (follow) {
        update({ themeMode: "system" });
      } else {
        update({ themeMode: effectiveMode });
      }
    },
    [update, effectiveMode]
  );

  const value = useMemo(
    () => ({
      settings,
      setLocale,
      setThemeMode,
      setAccentColor,
      setOutputDir,
      setEncoderAvc,
      setEncoderHevc,
      resetSettings,
      followSystem,
      setFollowSystem,
      effectiveMode,
    }),
    [
      settings,
      setLocale,
      setThemeMode,
      setAccentColor,
      setOutputDir,
      setEncoderAvc,
      setEncoderHevc,
      resetSettings,
      followSystem,
      setFollowSystem,
      effectiveMode,
    ]
  );

  return (
    <SettingsContext.Provider value={value}>
      {children}
    </SettingsContext.Provider>
  );
};

export const useSettings = (): SettingsContextValue => {
  const ctx = useContext(SettingsContext);
  if (!ctx) {
    throw new Error("useSettings must be used within SettingsProvider");
  }
  return ctx;
};

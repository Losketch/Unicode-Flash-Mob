import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import {
  loadAppSettings,
  SUPPORTED_LOCALES,
  type SupportedLocale,
} from "../config/settings";
import { en, type TranslationResource } from "../locales/en";
import { zhCN } from "../locales/zh-CN";

export { SUPPORTED_LOCALES, type SupportedLocale } from "../config/settings";

export const LOCALE_LABELS: Record<SupportedLocale, string> = {
  en: "English",
  "zh-CN": "简体中文",
};

const resources: Record<SupportedLocale, TranslationResource> = {
  "zh-CN": zhCN,
  en,
};

const initialLocale = loadAppSettings().locale;

i18n.use(initReactI18next).init({
  resources,
  lng: initialLocale,
  supportedLngs: [...SUPPORTED_LOCALES],
  fallbackLng: "en",
  interpolation: {
    escapeValue: false,
  },
});

export default i18n;

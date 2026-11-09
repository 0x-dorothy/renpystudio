import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { ru } from "./locales/ru";
import { en } from "./locales/en";

export const SUPPORTED_LOCALES = ["ru", "en"] as const;
export type Locale = (typeof SUPPORTED_LOCALES)[number];

void i18n.use(initReactI18next).init({
  resources: {
    ru: { translation: ru },
    en: { translation: en },
  },
  lng: "ru",
  fallbackLng: "ru",
  interpolation: { escapeValue: false },
  returnNull: false,
});

export function setLanguage(locale: string) {
  if (i18n.language !== locale) {
    void i18n.changeLanguage(locale);
  }
}

export default i18n;

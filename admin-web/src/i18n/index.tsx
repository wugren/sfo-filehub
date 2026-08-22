// 轻量 i18n：React Context + 类型化字典；语言偏好仅存 localStorage（不存任何凭据）。

import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { MESSAGES, type Lang, type MessageKey } from "./messages";

const STORAGE_KEY = "fh_web_lang";
const HTML_TITLE: Record<Lang, string> = { zh: "filehub 管理后台", en: "filehub Admin Console" };
const HTML_LANG: Record<Lang, string> = { zh: "zh-CN", en: "en" };
const DATE_LOCALE: Record<Lang, string> = { zh: "zh-CN", en: "en-US" };

function initialLanguage(): Lang {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "zh" || stored === "en") {
      return stored;
    }
  } catch {
    // 存储不可用时回退浏览器语言。
  }
  if (typeof navigator === "undefined") {
    return "zh";
  }
  return navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
}

export type Translator = (key: MessageKey, params?: Record<string, string | number>) => string;

export interface LanguageContextValue {
  lang: Lang;
  setLang: (lang: Lang) => void;
  t: Translator;
}

export const LanguageContext = createContext<LanguageContextValue>({
  lang: "zh",
  setLang: () => undefined,
  t: (key) => MESSAGES[key].zh,
});

function render(template: string, params?: Record<string, string | number>): string {
  if (!params) {
    return template;
  }
  return template.replace(/\{(\w+)\}/g, (whole, name: string) => {
    const value = params[name];
    return value === undefined ? whole : String(value);
  });
}

export function LanguageProvider({ children }: { children: ReactNode }): ReactNode {
  const [lang, setLangState] = useState<Lang>(initialLanguage);

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, lang);
    } catch {
      // 存储不可用时仅保留当前会话的语言选择。
    }
    document.documentElement.lang = HTML_LANG[lang];
    document.title = HTML_TITLE[lang];
  }, [lang]);

  const value = useMemo<LanguageContextValue>(() => {
    return {
      lang,
      setLang: setLangState,
      t: (key, params) => render(MESSAGES[key][lang], params),
    };
  }, [lang]);

  return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>;
}

export function useLanguage(): LanguageContextValue {
  return useContext(LanguageContext);
}

export function useT(): Translator {
  return useLanguage().t;
}

export function formatDate(iso: string, lang: Lang): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  try {
    return new Intl.DateTimeFormat(DATE_LOCALE[lang], {
      year: "numeric",
      month: "short",
      day: "numeric",
    }).format(date);
  } catch {
    return date.toDateString();
  }
}

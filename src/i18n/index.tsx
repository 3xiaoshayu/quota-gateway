import { createContext, useCallback, useContext, useEffect, useMemo, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { zhCN, type MessageKey, type MessageVariables } from './resources';

// The console ships in Simplified Chinese only. The provider keeps the same
// shape the pages already use, so `t()` call sites did not have to change.
export type AppLocale = 'zh-CN';

const LOCALE: AppLocale = 'zh-CN';

export function getCurrentLocale(): AppLocale {
  return LOCALE;
}

export function normalizeLocale(_value: string | null | undefined): AppLocale {
  return LOCALE;
}

function interpolate(template: string, variables?: MessageVariables): string {
  if (!variables) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    Object.prototype.hasOwnProperty.call(variables, name) ? String(variables[name]) : match,
  );
}

export function translate(_locale: AppLocale, key: MessageKey, variables?: MessageVariables): string {
  return interpolate(zhCN[key], variables);
}

type I18nContextValue = {
  locale: AppLocale;
  setLocale: (locale: AppLocale) => void;
  t: (key: MessageKey, variables?: MessageVariables) => string;
  formatNumber: (value: number, options?: Intl.NumberFormatOptions) => string;
  formatDate: (value: Date | number | string, options?: Intl.DateTimeFormatOptions) => string;
};

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  useEffect(() => {
    document.documentElement.lang = LOCALE;
    document.documentElement.dir = 'ltr';
    void invoke('set_app_locale', { locale: LOCALE }).catch((error) => {
      console.warn('Failed to synchronize the app locale with the native shell', error);
    });
  }, []);

  const setLocale = useCallback((_nextLocale: AppLocale) => {}, []);
  const t = useCallback(
    (key: MessageKey, variables?: MessageVariables) => translate(LOCALE, key, variables),
    [],
  );
  const formatNumber = useCallback(
    (value: number, options?: Intl.NumberFormatOptions) =>
      new Intl.NumberFormat(LOCALE, options).format(value),
    [],
  );
  const formatDate = useCallback(
    (value: Date | number | string, options?: Intl.DateTimeFormatOptions) => {
      const date = value instanceof Date ? value : new Date(value);
      return new Intl.DateTimeFormat(LOCALE, options).format(date);
    },
    [],
  );

  const context = useMemo<I18nContextValue>(
    () => ({ locale: LOCALE, setLocale, t, formatNumber, formatDate }),
    [formatDate, formatNumber, setLocale, t],
  );

  return <I18nContext.Provider value={context}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nContextValue {
  const context = useContext(I18nContext);
  if (!context) throw new Error('useI18n must be used inside I18nProvider');
  return context;
}

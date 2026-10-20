import { createNasI18n } from '@valeriogc/nas-base-app';
import it from './locales/it';
import en from './locales/en';

export const messages = { it, en };
export const { locale, setLocale } = createNasI18n(messages);

export function t(key: string): string {
  const catalog: Record<string, string> = messages[locale.value];
  return catalog[key] ?? key;
}

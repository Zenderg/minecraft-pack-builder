export const languages = ["en", "ru"] as const;

export type Language = (typeof languages)[number];
export type LocalizedText = Record<Language, string>;

export function isLanguage(value: string): value is Language {
  return languages.includes(value as Language);
}

export function getInitialLanguage(
  preferredLanguages: readonly string[] = navigator.languages ?? [navigator.language],
): Language {
  return preferredLanguages[0]?.toLowerCase().startsWith("ru") ? "ru" : "en";
}

export function textForLanguage(language: Language, text: LocalizedText): string {
  return text[language];
}

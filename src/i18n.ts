export const languages = ["en", "ru"] as const;

export type Language = (typeof languages)[number];

type MessageKey =
  | "app.title"
  | "workspace.library"
  | "workspace.viewer"
  | "workspace.review"
  | "workspace.materials"
  | "workspace.settings"
  | "workspace.addModpack"
  | "status.aiDisconnected"
  | "status.localOnly"
  | "library.empty"
  | "library.modpack"
  | "library.scheme"
  | "viewer.emptyTitle"
  | "viewer.emptyBody"
  | "review.changeRequests"
  | "review.pending"
  | "review.selection"
  | "materials.total"
  | "settings.language"
  | "settings.diagnostics"
  | "settings.openDataFolder"
  | "settings.appData"
  | "settings.desktopOnly";

const messages: Record<Language, Record<MessageKey, string>> = {
  en: {
    "app.title": "Minecraft Pack Builder",
    "workspace.library": "Library",
    "workspace.viewer": "Viewer",
    "workspace.review": "Review",
    "workspace.materials": "Materials",
    "workspace.settings": "Settings",
    "workspace.addModpack": "Add modpack",
    "status.aiDisconnected": "AI disconnected",
    "status.localOnly": "Local workspace",
    "library.empty": "No imported modpacks yet",
    "library.modpack": "All the Mods 10",
    "library.scheme": "Starter Factory",
    "viewer.emptyTitle": "Scheme viewer ready",
    "viewer.emptyBody": "Import a modpack and open a scheme to review the structure here.",
    "review.changeRequests": "Change requests",
    "review.pending": "Pending",
    "review.selection": "Selection",
    "materials.total": "Total blocks",
    "settings.language": "Language",
    "settings.diagnostics": "Diagnostics",
    "settings.openDataFolder": "Open data folder",
    "settings.appData": "App data",
    "settings.desktopOnly": "Available in the desktop app",
  },
  ru: {
    "app.title": "Minecraft Pack Builder",
    "workspace.library": "Библиотека",
    "workspace.viewer": "Просмотр",
    "workspace.review": "Проверка",
    "workspace.materials": "Материалы",
    "workspace.settings": "Настройки",
    "workspace.addModpack": "Добавить модпак",
    "status.aiDisconnected": "AI не подключен",
    "status.localOnly": "Локальная рабочая среда",
    "library.empty": "Импортированных модпаков пока нет",
    "library.modpack": "All the Mods 10",
    "library.scheme": "Стартовая фабрика",
    "viewer.emptyTitle": "Просмотр схемы готов",
    "viewer.emptyBody": "Импортируйте модпак и откройте схему, чтобы проверить структуру здесь.",
    "review.changeRequests": "Запросы на изменения",
    "review.pending": "Ожидают",
    "review.selection": "Выделение",
    "materials.total": "Всего блоков",
    "settings.language": "Язык",
    "settings.diagnostics": "Диагностика",
    "settings.openDataFolder": "Открыть папку данных",
    "settings.appData": "Данные приложения",
    "settings.desktopOnly": "Доступно в desktop-приложении",
  },
};

export function isLanguage(value: string): value is Language {
  return languages.includes(value as Language);
}

export function getInitialLanguage(systemLanguages: readonly string[] = navigator.languages): Language {
  const first = systemLanguages[0]?.toLowerCase() ?? "";
  return first.startsWith("ru") ? "ru" : "en";
}

export function translate(language: Language, key: MessageKey): string {
  return messages[language][key];
}

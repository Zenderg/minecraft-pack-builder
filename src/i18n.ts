export const languages = ["en", "ru"] as const;

export type Language = (typeof languages)[number];

const messages = {
  en: {
    "app.title": "Minecraft Pack Builder",
    "workspace.library": "Library",
    "workspace.viewer": "Viewer",
    "workspace.review": "Review",
    "workspace.materials": "Materials",
    "workspace.settings": "Settings",
    "workspace.addModpack": "Add modpack",
    "status.aiDisconnected": "AI disconnected",
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
    "onboarding.title": "Set up your local builder",
    "onboarding.languageTitle": "Choose the interface language",
    "onboarding.languageBody": "You can change this later in Settings.",
    "onboarding.aiTitle": "Connect an external AI tool",
    "onboarding.aiBody":
      "The app will expose local tools for Codex, Claude Code, opencode, or a similar client. You can continue now and connect the agent later.",
    "onboarding.aiPromptTitle": "Prompt for your agent",
    "onboarding.aiPrompt":
      "You are helping me use Minecraft Pack Builder. When the app exposes its local MCP-compatible tools, connect to them and use only those tools for modpack imports, scheme edits, validation, and export. Do not read or write project files directly unless I explicitly ask. If the tool server is not available yet, tell me what is missing and wait.",
    "onboarding.keyTitle": "Add your CurseForge API key",
    "onboarding.keyBody":
      "The key is used only by the desktop backend to discover and download modpack releases. It is stored in the operating system secure credential store.",
    "onboarding.skip": "Skip",
    "onboarding.next": "Next",
    "onboarding.back": "Back",
    "onboarding.finish": "Finish",
    "onboarding.saveAndFinish": "Save and finish",
    "settings.aiIntegration": "AI integration",
    "settings.curseforgeKey": "CurseForge API key",
    "settings.dataFolders": "Data folders",
    "settings.status": "Status",
    "settings.activeClient": "Active client",
    "settings.connection": "Connection",
    "settings.aiInstructions":
      "Use an external AI client and connect it to the local tool server once the MCP surface lands in a later phase.",
    "settings.noActiveClient": "No active client",
    "settings.keyMissing": "No key saved",
    "settings.keySaved": "Key saved securely",
    "settings.keyReplaced": "Key replaced securely",
    "settings.keyUnavailable": "Secure storage unavailable",
    "settings.keyStoredNotice": "Saved keys are never displayed again.",
    "settings.keyPlaceholder": "Paste CurseForge API key",
    "settings.backend": "Secure backend",
    "settings.diagnosticsFolder": "Diagnostics folder",
    "settings.openSettings": "Open settings",
    "settings.close": "Close settings",
    "settings.checkKey": "Check key",
    "settings.keyCheckEmpty": "Paste a key before checking it.",
    "settings.keyCheckReady":
      "Local check passed and the key was saved. Online validation will run during modpack import.",
    "settings.keyCheckNote": "Phase 2 does not call CurseForge yet.",
    "settings.showOnboarding": "Show onboarding again",
    "settings.existingKey": "A CurseForge API key is already saved. You can finish onboarding or paste a new key and check it to replace the saved key.",
    "import.readyTitle": "Modpack import is ready for the next phase",
    "import.readyBody":
      "The secure key is saved. Release discovery and downloads are implemented in phase 5.",
    "import.needsKey": "Add a CurseForge key before importing a modpack.",
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
    "onboarding.title": "Настройте локальный builder",
    "onboarding.languageTitle": "Выберите язык интерфейса",
    "onboarding.languageBody": "Позже язык можно изменить в настройках.",
    "onboarding.aiTitle": "Подключите внешний AI-инструмент",
    "onboarding.aiBody":
      "Приложение будет отдавать локальные инструменты Codex, Claude Code, opencode или похожему клиенту. Можно продолжить сейчас и подключить агента позже.",
    "onboarding.aiPromptTitle": "Промпт для вашего агента",
    "onboarding.aiPrompt":
      "You are helping me use Minecraft Pack Builder. When the app exposes its local MCP-compatible tools, connect to them and use only those tools for modpack imports, scheme edits, validation, and export. Do not read or write project files directly unless I explicitly ask. If the tool server is not available yet, tell me what is missing and wait.",
    "onboarding.keyTitle": "Добавьте CurseForge API key",
    "onboarding.keyBody":
      "Ключ нужен только desktop-backend для поиска и скачивания релизов модпаков. Он хранится в системном защищенном хранилище.",
    "onboarding.skip": "Пропустить",
    "onboarding.next": "Далее",
    "onboarding.back": "Назад",
    "onboarding.finish": "Завершить",
    "onboarding.saveAndFinish": "Сохранить и завершить",
    "settings.aiIntegration": "AI integration",
    "settings.curseforgeKey": "CurseForge API key",
    "settings.dataFolders": "Папки данных",
    "settings.status": "Статус",
    "settings.activeClient": "Активный клиент",
    "settings.connection": "Подключение",
    "settings.aiInstructions":
      "Используйте внешний AI-клиент и подключите его к локальному серверу инструментов, когда MCP-поверхность появится в следующей фазе.",
    "settings.noActiveClient": "Активного клиента нет",
    "settings.keyMissing": "Ключ не сохранен",
    "settings.keySaved": "Ключ сохранен безопасно",
    "settings.keyReplaced": "Ключ заменен безопасно",
    "settings.keyUnavailable": "Защищенное хранилище недоступно",
    "settings.keyStoredNotice": "Сохраненные ключи больше не показываются.",
    "settings.keyPlaceholder": "Вставьте CurseForge API key",
    "settings.backend": "Secure backend",
    "settings.diagnosticsFolder": "Папка диагностики",
    "settings.openSettings": "Открыть настройки",
    "settings.close": "Закрыть настройки",
    "settings.checkKey": "Проверить ключ",
    "settings.keyCheckEmpty": "Вставьте ключ перед проверкой.",
    "settings.keyCheckReady":
      "Локальная проверка пройдена, ключ сохранен. Онлайн-валидация будет при импорте модпака.",
    "settings.keyCheckNote": "Фаза 2 пока не вызывает CurseForge.",
    "settings.showOnboarding": "Открыть onboarding заново",
    "settings.existingKey": "CurseForge API key уже сохранен. Можно завершить onboarding или вставить новый ключ и проверить его, чтобы заменить сохраненный.",
    "import.readyTitle": "Импорт модпака готов к следующей фазе",
    "import.readyBody":
      "Защищенный ключ сохранен. Поиск релизов и скачивание реализуются в фазе 5.",
    "import.needsKey": "Добавьте CurseForge key перед импортом модпака.",
  },
} satisfies Record<Language, Record<string, string>>;

export type MessageKey = keyof (typeof messages)["en"];

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

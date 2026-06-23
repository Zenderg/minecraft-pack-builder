import { describe, expect, it } from "vitest";
import { getInitialLanguage, isLanguage, translate } from "./i18n";

describe("i18n", () => {
  it("translates the workspace chrome into English and Russian", () => {
    expect(translate("en", "workspace.library")).toBe("Library");
    expect(translate("ru", "workspace.library")).toBe("Библиотека");
    expect(translate("en", "status.aiDisconnected")).toBe("AI disconnected");
    expect(translate("ru", "status.aiDisconnected")).toBe("AI не подключен");
  });

  it("accepts only supported languages", () => {
    expect(isLanguage("en")).toBe(true);
    expect(isLanguage("ru")).toBe(true);
    expect(isLanguage("de")).toBe(false);
  });

  it("uses Russian when the system language starts with ru", () => {
    expect(getInitialLanguage(["ru-RU", "en-US"])).toBe("ru");
    expect(getInitialLanguage(["en-US", "ru-RU"])).toBe("en");
  });
});

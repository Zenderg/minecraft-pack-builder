// @vitest-environment jsdom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const tauriMocks = vi.hoisted(() => ({
  checkResolve: undefined as (() => void) | undefined,
  cancelCurseForgeImport: vi.fn(),
  checkCurseForgeApiKey: vi.fn(),
  createScheme: vi.fn(),
  deleteImportedModpack: vi.fn(),
  deleteScheme: vi.fn(),
  discoverAppPaths: vi.fn(),
  exportScheme: vi.fn(),
  getAiIntegrationStatus: vi.fn(),
  getCurseForgeKeyStatus: vi.fn(),
  listLibrary: vi.fn(),
  listenToAgentEvents: vi.fn(),
  listenToModpackImportStatus: vi.fn(),
  listenToModpackImportProgress: vi.fn(),
  openAppDataFolder: vi.fn(),
  renameImportedModpack: vi.fn(),
  renameScheme: vi.fn(),
  retryModpackImport: vi.fn(),
  saveCurseForgeApiKey: vi.fn(),
  searchCurseForgeModpacks: vi.fn(),
}));

vi.mock("./tauri", () => ({
  cancelCurseForgeImport: tauriMocks.cancelCurseForgeImport,
  checkCurseForgeApiKey: tauriMocks.checkCurseForgeApiKey,
  createScheme: tauriMocks.createScheme,
  deleteImportedModpack: tauriMocks.deleteImportedModpack,
  deleteScheme: tauriMocks.deleteScheme,
  discoverAppPaths: tauriMocks.discoverAppPaths,
  exportScheme: tauriMocks.exportScheme,
  getAiIntegrationStatus: tauriMocks.getAiIntegrationStatus,
  getCurseForgeKeyStatus: tauriMocks.getCurseForgeKeyStatus,
  listLibrary: tauriMocks.listLibrary,
  listenToAgentEvents: tauriMocks.listenToAgentEvents,
  listenToModpackImportStatus: tauriMocks.listenToModpackImportStatus,
  listenToModpackImportProgress: tauriMocks.listenToModpackImportProgress,
  openAppDataFolder: tauriMocks.openAppDataFolder,
  renameImportedModpack: tauriMocks.renameImportedModpack,
  renameScheme: tauriMocks.renameScheme,
  retryModpackImport: tauriMocks.retryModpackImport,
  saveCurseForgeApiKey: tauriMocks.saveCurseForgeApiKey,
  searchCurseForgeModpacks: tauriMocks.searchCurseForgeModpacks,
}));

import { App } from "./App";

function buttonByText(container: HTMLElement, text: string): HTMLButtonElement {
  const button = [...container.querySelectorAll("button")].find((item) =>
    item.textContent?.includes(text),
  );
  if (!(button instanceof HTMLButtonElement)) {
    const buttonTexts = [...container.querySelectorAll("button")]
      .map((item) => item.textContent?.trim())
      .join(", ");
    throw new Error(`Button with text "${text}" was not found. Existing buttons: ${buttonTexts}`);
  }
  return button;
}

function installLocalStorageMock() {
  const values = new Map<string, string>();
  const storage: Storage = {
    get length() {
      return values.size;
    },
    clear: () => values.clear(),
    getItem: (key) => values.get(key) ?? null,
    key: (index) => [...values.keys()][index] ?? null,
    removeItem: (key) => values.delete(key),
    setItem: (key, value) => values.set(key, value),
  };
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: storage,
  });
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: storage,
  });
}

describe("CurseForge key check UI", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(async () => {
    Object.defineProperty(globalThis, "IS_REACT_ACT_ENVIRONMENT", {
      configurable: true,
      value: true,
      writable: true,
    });
    installLocalStorageMock();
    localStorage.setItem("mpb.onboardingComplete", "true");
    tauriMocks.checkResolve = undefined;
    tauriMocks.discoverAppPaths.mockResolvedValue(null);
    tauriMocks.getCurseForgeKeyStatus.mockResolvedValue({
      state: "missing",
      backend: "Test secure storage",
      message: null,
      apiKey: null,
    });
    tauriMocks.getAiIntegrationStatus.mockResolvedValue({
      serverRunning: true,
      transport: "streamable-http",
      endpoint: "http://127.0.0.1:7777/mcp",
      protocolVersion: "2025-06-18",
      activeClient: null,
      toolCount: 19,
    });
    tauriMocks.listLibrary.mockResolvedValue([]);
    tauriMocks.listenToAgentEvents.mockResolvedValue(() => {});
    tauriMocks.listenToModpackImportStatus.mockResolvedValue(() => {});
    tauriMocks.listenToModpackImportProgress.mockResolvedValue(() => {});
    tauriMocks.checkCurseForgeApiKey.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          tauriMocks.checkResolve = resolve;
        }),
    );
    tauriMocks.saveCurseForgeApiKey.mockResolvedValue({
      state: "saved",
      backend: "Test secure storage",
      message: null,
      apiKey: null,
    });

    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => {
      root.render(<App />);
    });
  });

  afterEach(() => {
    if (root) {
      act(() => {
        root.unmount();
      });
    }
    container?.remove();
    localStorage.clear();
    vi.clearAllMocks();
  });

  it("shows an immediate visible loading state while the key check is pending", async () => {
    await act(async () => {
      buttonByText(container, "Settings").click();
    });
    await act(async () => {
      buttonByText(container, "CurseForge API key").click();
    });

    const input = container.querySelector('input[type="password"]');
    if (!(input instanceof HTMLInputElement)) {
      throw new Error("CurseForge key input was not found");
    }

    await act(async () => {
      const valueSetter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
      valueSetter?.call(input, "valid-key");
      input.dispatchEvent(new Event("input", { bubbles: true }));
      input.dispatchEvent(new Event("change", { bubbles: true }));
    });
    act(() => {
      buttonByText(container, "Check key").click();
    });

    const loadingButton = buttonByText(container, "Checking...");
    expect(loadingButton.className).toContain("loading");
    expect(loadingButton.getAttribute("aria-busy")).toBe("true");
    expect(container.querySelector(".secret-input-row.checking")).not.toBeNull();
    expect(container.querySelector('[role="status"]')?.textContent).toContain(
      "Checking the key with CurseForge...",
    );

    await act(async () => {
      tauriMocks.checkResolve?.();
    });
  });
});

// @vitest-environment jsdom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const tauriMocks = vi.hoisted(() => ({
  cancelCurseForgeImport: vi.fn(),
  checkCurseForgeApiKey: vi.fn(),
  checkForUpdates: vi.fn(),
  createScheme: vi.fn(),
  deleteImportedModpack: vi.fn(),
  deleteScheme: vi.fn(),
  discoverAppPaths: vi.fn(),
  exportScheme: vi.fn(),
  getAiIntegrationStatus: vi.fn(),
  getCurseForgeKeyStatus: vi.fn(),
  getSchemeRenderScene: vi.fn(),
  listLibrary: vi.fn(),
  listenToAgentEvents: vi.fn(),
  listenToModpackImportProgress: vi.fn(),
  listenToModpackImportStatus: vi.fn(),
  openAppDataFolder: vi.fn(),
  renameImportedModpack: vi.fn(),
  renameScheme: vi.fn(),
  retryModpackImport: vi.fn(),
  saveCurseForgeApiKey: vi.fn(),
}));

vi.mock("./tauri", () => ({
  cancelCurseForgeImport: tauriMocks.cancelCurseForgeImport,
  checkCurseForgeApiKey: tauriMocks.checkCurseForgeApiKey,
  checkForUpdates: tauriMocks.checkForUpdates,
  createScheme: tauriMocks.createScheme,
  deleteImportedModpack: tauriMocks.deleteImportedModpack,
  deleteScheme: tauriMocks.deleteScheme,
  discoverAppPaths: tauriMocks.discoverAppPaths,
  exportScheme: tauriMocks.exportScheme,
  getAiIntegrationStatus: tauriMocks.getAiIntegrationStatus,
  getCurseForgeKeyStatus: tauriMocks.getCurseForgeKeyStatus,
  getSchemeRenderScene: tauriMocks.getSchemeRenderScene,
  listLibrary: tauriMocks.listLibrary,
  listenToAgentEvents: tauriMocks.listenToAgentEvents,
  listenToModpackImportProgress: tauriMocks.listenToModpackImportProgress,
  listenToModpackImportStatus: tauriMocks.listenToModpackImportStatus,
  openAppDataFolder: tauriMocks.openAppDataFolder,
  renameImportedModpack: tauriMocks.renameImportedModpack,
  renameScheme: tauriMocks.renameScheme,
  retryModpackImport: tauriMocks.retryModpackImport,
  saveCurseForgeApiKey: tauriMocks.saveCurseForgeApiKey,
}));

import { App } from "./App";
import type { RenderScene } from "./renderViewer";

const createdRenderScene: RenderScene = {
  schemeId: 10,
  schemeName: "Compact Base",
  dimensions: [12, 2500, 24],
  stages: [],
  blocks: [],
  chunks: [],
  largeSchemeThreshold: 4096,
};

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

function buttonByText(container: HTMLElement, text: string): HTMLButtonElement {
  const button = [...container.querySelectorAll("button")].find((item) =>
    item.textContent?.includes(text),
  );
  if (!(button instanceof HTMLButtonElement)) {
    throw new Error(`Button with text "${text}" was not found`);
  }
  return button;
}

function changeInput(input: HTMLInputElement, value: string) {
  const valueSetter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
  valueSetter?.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

describe("create scheme dialog", () => {
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
    tauriMocks.discoverAppPaths.mockResolvedValue(null);
    tauriMocks.getCurseForgeKeyStatus.mockResolvedValue({
      state: "saved",
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
    tauriMocks.checkForUpdates.mockResolvedValue({
      status: "current",
      currentVersion: "0.1.0",
      latestVersion: null,
      notes: null,
      date: null,
      errorMessage: null,
    });
    tauriMocks.listenToAgentEvents.mockResolvedValue(() => {});
    tauriMocks.listenToModpackImportStatus.mockResolvedValue(() => {});
    tauriMocks.listenToModpackImportProgress.mockResolvedValue(() => {});
    tauriMocks.listLibrary.mockResolvedValue([
      {
        id: 1,
        localName: "AOC - 1.0.0",
        sourceUrl: "https://www.curseforge.com/minecraft/modpacks/aoc",
        versionName: "1.0.0",
        minecraftVersion: "1.20.1",
        loader: "Forge",
        importStatus: "imported",
        importMessage: null,
        schemes: [],
      },
    ]);
    tauriMocks.createScheme.mockResolvedValue([
      {
        id: 1,
        localName: "AOC - 1.0.0",
        sourceUrl: "https://www.curseforge.com/minecraft/modpacks/aoc",
        versionName: "1.0.0",
        minecraftVersion: "1.20.1",
        loader: "Forge",
        importStatus: "imported",
        importMessage: null,
        schemes: [
          {
            id: 10,
            modpackId: 1,
            name: "Compact Base",
            dimensions: [12, 2500, 24],
          },
        ],
      },
    ]);
    tauriMocks.getSchemeRenderScene.mockResolvedValue(createdRenderScene);

    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    await act(async () => {
      root.render(<App />);
    });
  });

  afterEach(() => {
    act(() => {
      root.unmount();
    });
    container.remove();
    localStorage.clear();
    vi.clearAllMocks();
  });

  it("lets users choose dimensions with sliders or manual values beyond the slider cap", async () => {
    await act(async () => {
      container.querySelector<HTMLButtonElement>('button[aria-label="Create scheme"]')?.click();
    });

    const dialog = container.querySelector('section[role="dialog"][aria-label="Create scheme"]');
    expect(dialog).not.toBeNull();
    const nameInput = dialog!.querySelector<HTMLInputElement>('input[type="text"]');
    const sliders = [...dialog!.querySelectorAll<HTMLInputElement>('input[type="range"]')];
    const dimensionInputs = [...dialog!.querySelectorAll<HTMLInputElement>('input[type="number"]')];
    expect(nameInput?.value).toBe("New scheme");
    expect(sliders).toHaveLength(3);
    expect(dimensionInputs).toHaveLength(3);
    expect(sliders.map((input) => input.max)).toEqual(["2000", "2000", "2000"]);
    expect(dimensionInputs.map((input) => input.value)).toEqual(["64", "64", "64"]);

    await act(async () => {
      changeInput(nameInput!, "Compact Base");
      changeInput(sliders[0], "12");
      changeInput(dimensionInputs[1], "2500");
      changeInput(sliders[2], "24");
    });
    await act(async () => {
      buttonByText(container, "Confirm").click();
    });

    expect(tauriMocks.createScheme).toHaveBeenCalledWith(1, "Compact Base", [12, 2500, 24]);
  });
});

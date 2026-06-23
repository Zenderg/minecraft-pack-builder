// @vitest-environment jsdom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const tauriMocks = vi.hoisted(() => ({
  cancelCurseForgeImport: vi.fn(),
  checkCurseForgeApiKey: vi.fn(),
  createScheme: vi.fn(),
  deleteImportedModpack: vi.fn(),
  deleteScheme: vi.fn(),
  discoverAppPaths: vi.fn(),
  getCurseForgeKeyStatus: vi.fn(),
  getSchemeRenderScene: vi.fn(),
  listLibrary: vi.fn(),
  listenToModpackImportProgress: vi.fn(),
  listenToModpackImportStatus: vi.fn(),
  openAppDataFolder: vi.fn(),
  renameImportedModpack: vi.fn(),
  renameScheme: vi.fn(),
  retryModpackImport: vi.fn(),
  saveCurseForgeApiKey: vi.fn(),
  seedLocalLibraryFixture: vi.fn(),
}));

vi.mock("./tauri", () => ({
  cancelCurseForgeImport: tauriMocks.cancelCurseForgeImport,
  checkCurseForgeApiKey: tauriMocks.checkCurseForgeApiKey,
  createScheme: tauriMocks.createScheme,
  deleteImportedModpack: tauriMocks.deleteImportedModpack,
  deleteScheme: tauriMocks.deleteScheme,
  discoverAppPaths: tauriMocks.discoverAppPaths,
  getCurseForgeKeyStatus: tauriMocks.getCurseForgeKeyStatus,
  getSchemeRenderScene: tauriMocks.getSchemeRenderScene,
  listLibrary: tauriMocks.listLibrary,
  listenToModpackImportProgress: tauriMocks.listenToModpackImportProgress,
  listenToModpackImportStatus: tauriMocks.listenToModpackImportStatus,
  openAppDataFolder: tauriMocks.openAppDataFolder,
  renameImportedModpack: tauriMocks.renameImportedModpack,
  renameScheme: tauriMocks.renameScheme,
  retryModpackImport: tauriMocks.retryModpackImport,
  saveCurseForgeApiKey: tauriMocks.saveCurseForgeApiKey,
  seedLocalLibraryFixture: tauriMocks.seedLocalLibraryFixture,
}));

import { App } from "./App";
import { browserRenderSceneFixture } from "./renderViewer";

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

describe("phase 7 viewer workspace", () => {
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
        schemes: [
          {
            id: 10,
            modpackId: 1,
            name: "Starter Factory",
            dimensions: [8, 5, 8],
          },
        ],
      },
    ]);
    tauriMocks.getSchemeRenderScene.mockResolvedValue(browserRenderSceneFixture);

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

  it("renders stage controls and selected-stage metrics for an opened scheme", async () => {
    await act(async () => {
      container.querySelector<HTMLButtonElement>(".scheme-label")?.click();
    });

    expect(await screenText(container, "Stage 1")).toBe(true);
    expect(await screenText(container, "Stage 2")).toBe(true);
    expect(await screenText(container, "Unassigned")).toBe(true);
    expect(container.querySelector(".viewer-three-canvas")).not.toBeNull();
    expect(container.textContent).toContain("7 / 9");
    expect(container.querySelector(".viewer-toolbar")).toBeNull();
    expect(container.querySelector(".tool-summary")).not.toBeNull();
    expect(container.querySelector(".tool-summary")?.textContent).toContain("7 / 9");
    expect(container.querySelector(".tool-summary")?.textContent).toContain("2 chunks");
    expect(container.querySelector(".tool-summary")?.textContent).toContain("faces");
    expect(container.querySelector(".tool-rail")).toBeNull();
    expect(container.querySelector(".tool-tree")).not.toBeNull();
    expect(container.querySelector(".tool-node.expanded")).not.toBeNull();
    expect(container.querySelector(".tool-label")?.textContent).toContain("Stages");
    expect(container.querySelector(".tool-children")?.textContent).toContain("Stage 2");
    expect(container.querySelector(".viewer-toolbar .stage-tabs")).toBeNull();
    expect(container.querySelector(".tool-panel-stage-list")?.textContent).toContain("Stage 2");
    expect(container.querySelector(".viewer-heading")).toBeNull();
    expect(container.querySelector(".viewer-footer-primary")?.textContent).toContain("Starter Factory");
    expect(container.querySelector(".viewer-footer-primary")?.textContent).toContain("8 x 5 x 8");
    expect(container.querySelector(".viewer-region")?.textContent).not.toContain("Viewer");
    expect(container.querySelector(".status-strip")).toBeNull();
    expect(container.querySelector(".language-switch")).toBeNull();
    expect(container.querySelector(".brand-status")?.textContent).toContain("AI disconnected");

    await act(async () => {
      buttonByText(container, "Stage 1").click();
    });

    expect(container.textContent).toContain("3 / 9");
    expect(container.querySelector(".tool-summary")?.textContent).toContain("3 / 9");

    await act(async () => {
      buttonByText(container, "Materials").click();
    });

    expect(container.querySelector(".tool-tree")?.textContent).toContain("minecraft:stone_bricks");
  });
});

function buttonByText(container: HTMLElement, text: string): HTMLButtonElement {
  const button = [...container.querySelectorAll("button")].find((item) =>
    item.textContent?.includes(text),
  );
  if (!(button instanceof HTMLButtonElement)) {
    throw new Error(`Button with text "${text}" was not found`);
  }
  return button;
}

async function screenText(container: HTMLElement, text: string): Promise<boolean> {
  await act(async () => {
    await Promise.resolve();
  });
  return container.textContent?.includes(text) ?? false;
}

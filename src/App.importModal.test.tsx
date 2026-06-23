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
  listLibrary: vi.fn(),
  listenToModpackImportStatus: vi.fn(),
  listenToModpackImportProgress: vi.fn(),
  openAppDataFolder: vi.fn(),
  renameImportedModpack: vi.fn(),
  renameScheme: vi.fn(),
  retryModpackImport: vi.fn(),
  saveCurseForgeApiKey: vi.fn(),
  searchCurseForgeModpacks: vi.fn(),
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
  listLibrary: tauriMocks.listLibrary,
  listenToModpackImportStatus: tauriMocks.listenToModpackImportStatus,
  listenToModpackImportProgress: tauriMocks.listenToModpackImportProgress,
  openAppDataFolder: tauriMocks.openAppDataFolder,
  renameImportedModpack: tauriMocks.renameImportedModpack,
  renameScheme: tauriMocks.renameScheme,
  retryModpackImport: tauriMocks.retryModpackImport,
  saveCurseForgeApiKey: tauriMocks.saveCurseForgeApiKey,
  searchCurseForgeModpacks: tauriMocks.searchCurseForgeModpacks,
  seedLocalLibraryFixture: tauriMocks.seedLocalLibraryFixture,
}));

import { App } from "./App";

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

describe("modpack import modal", () => {
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
    tauriMocks.listLibrary.mockResolvedValue([]);
    tauriMocks.listenToModpackImportStatus.mockResolvedValue(() => {});
    tauriMocks.listenToModpackImportProgress.mockResolvedValue(() => {});
    tauriMocks.searchCurseForgeModpacks.mockResolvedValue([
      { id: 42, name: "AOC", slug: "aoc", logoUrl: null },
    ]);

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

  it("opens add-modpack as a dialog without replacing the scheme viewer", async () => {
    await act(async () => {
      buttonByText(container, "Add modpack").click();
    });

    const dialog = container.querySelector('section[role="dialog"][aria-label="Add modpack"]');
    const viewer = container.querySelector('.content-grid > .viewer-region[aria-label="Viewer"]');
    const importInWorkspace = container.querySelector(".content-grid > .import-workspace");

    expect(dialog).not.toBeNull();
    expect(viewer).not.toBeNull();
    expect(importInWorkspace).toBeNull();
  });

  it("does not render asset diagnostics in the main workspace", () => {
    expect(container.querySelector(".asset-preview-panel")).toBeNull();
  });
});

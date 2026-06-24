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

const exportDialogMocks = vi.hoisted(() => ({
  chooseExportDestination: vi.fn(),
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

vi.mock("./exportDialog", () => ({
  chooseExportDestination: exportDialogMocks.chooseExportDestination,
}));

import { App } from "./App";
import type { RenderScene } from "./renderViewer";

const renderSceneFixture: RenderScene = {
  schemeId: 10,
  schemeName: "Starter Factory",
  dimensions: [8, 5, 8],
  stages: [
    { id: 1, name: "Stage 1", order: 1 },
    { id: 2, name: "Stage 2", order: 2 },
  ],
  blocks: [
    { coordinate: [0, 0, 0], blockId: "minecraft:stone_bricks", stageId: 1, color: "#9aa39e" },
    { coordinate: [1, 1, 0], blockId: "thermal:machine_frame", stageId: 2, color: "#d3a44e" },
    { coordinate: [4, 0, 1], blockId: "create:andesite_casing", stageId: null, color: "#6bb48f" },
  ],
  chunks: [{ coordinate: [0, 0, 0], blockCount: 3, faceCount: 18 }],
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
    tauriMocks.listenToModpackImportStatus.mockResolvedValue(() => {});
    tauriMocks.listenToModpackImportProgress.mockResolvedValue(() => {});
    tauriMocks.listenToAgentEvents.mockResolvedValue(() => {});
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
    tauriMocks.getSchemeRenderScene.mockResolvedValue(renderSceneFixture);
    exportDialogMocks.chooseExportDestination.mockResolvedValue("/tmp/starter-factory.litematic");
    tauriMocks.exportScheme.mockResolvedValue({
      schemeId: 10,
      format: "litematic",
      path: "/tmp/starter-factory.litematic",
      byteLen: 128,
      blockCount: 9,
    });

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
    expect(container.textContent).toContain("2 / 3");
    expect(container.querySelector(".viewer-toolbar")).toBeNull();
    expect(container.querySelector(".tool-summary")).not.toBeNull();
    expect(container.querySelector(".tool-summary")?.textContent).toContain("2 / 3");
    expect(container.querySelector(".tool-summary")?.textContent).toContain("1 chunks");
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
    expect(container.querySelector(".brand-status")?.textContent).toContain("AI server running");

    await act(async () => {
      buttonByText(container, "Stage 1").click();
    });

    expect(container.textContent).toContain("1 / 3");
    expect(container.querySelector(".tool-summary")?.textContent).toContain("1 / 3");

    await act(async () => {
      buttonByText(container, "Materials").click();
    });

    expect(container.querySelector(".tool-tree")?.textContent).toContain("minecraft:stone_bricks");
  });

  it("does not expose the removed selected-area notes workflow", async () => {
    await act(async () => {
      container.querySelector<HTMLButtonElement>(".scheme-label")?.click();
    });

    expect(await screenText(container, "Materials")).toBe(true);
    expect(container.textContent).not.toContain("Review");
    expect(container.textContent).not.toContain("Selected-area notes");
    expect(container.textContent).not.toContain("Move the machine wall two blocks east");
  });

  it("shows the MCP endpoint once in AI settings", async () => {
    await act(async () => {
      container.querySelector<HTMLButtonElement>(".settings-link")?.click();
    });

    expect(await screenText(container, "AI integration")).toBe(true);
    expect(countText(container, "http://127.0.0.1:7777/mcp")).toBe(1);
  });

  it("exports the opened scheme through its scheme actions menu and modal", async () => {
    await act(async () => {
      container.querySelector<HTMLButtonElement>(".scheme-label")?.click();
    });

    expect(container.querySelector(".viewer-footer")?.textContent).not.toContain("Export");

    await act(async () => {
      buttonByLabel(container, "Scheme actions").click();
    });

    expect(await screenText(container, "Rename")).toBe(true);
    expect(await screenText(container, "Export")).toBe(true);
    expect(await screenText(container, "Delete")).toBe(true);
    expect(container.querySelector(".modpack-menu")?.textContent).not.toContain("Scheme");

    await act(async () => {
      buttonByText(container, "Export").click();
    });

    expect(await screenText(container, "Export scheme")).toBe(true);

    await act(async () => {
      buttonByText(container, "Litematica .litematic").click();
    });

    await act(async () => {
      buttonByText(container, "Choose path").click();
    });

    expect(exportDialogMocks.chooseExportDestination).toHaveBeenCalledWith({
      defaultFileName: "Starter Factory.litematic",
      format: "litematic",
    });

    await act(async () => {
      buttonByText(container, "Export").click();
    });

    expect(tauriMocks.exportScheme).toHaveBeenCalledWith(
      10,
      "litematic",
      "/tmp/starter-factory.litematic",
    );
    expect(container.querySelector(".library-message")?.textContent).toContain(
      "/tmp/starter-factory.litematic",
    );
  });

  it("renders a handoff prompt with endpoint and interface language guidance", async () => {
    await act(async () => {
      container.querySelector<HTMLButtonElement>(".settings-link")?.click();
    });

    expect(await screenText(container, "AI integration")).toBe(true);
    const prompt = container.querySelector(".prompt-block code")?.textContent ?? "";
    expect(prompt).toContain("http://127.0.0.1:7777/mcp");
    expect(prompt).toContain("minecraft-pack-builder");
    expect(prompt).toContain("Respond to me in the same language as the Minecraft Pack Builder interface: English.");
    expect(prompt).toContain("If the client must be restarted or MCP config must be reloaded");
  });

  it("shows update settings, persists automatic checks, and reports manual update results", async () => {
    tauriMocks.checkForUpdates.mockResolvedValueOnce({
      status: "available",
      currentVersion: "0.1.0",
      latestVersion: "0.2.0",
      notes: "Packaging smoke build",
      date: "2026-06-24T00:00:00Z",
      errorMessage: null,
    });

    await act(async () => {
      container.querySelector<HTMLButtonElement>(".settings-link")?.click();
    });

    await act(async () => {
      buttonByText(container, "Updates").click();
    });

    expect(await screenText(container, "Automatic update checks")).toBe(true);
    expect(container.querySelector<HTMLInputElement>('input[type="checkbox"]')?.checked).toBe(true);
    expect(await screenText(container, "Current version")).toBe(true);

    await act(async () => {
      buttonByText(container, "Check for updates").click();
    });

    expect(tauriMocks.checkForUpdates).toHaveBeenCalledTimes(2);
    expect(await screenText(container, "Update available: 0.2.0")).toBe(true);
    expect(await screenText(container, "Packaging smoke build")).toBe(true);

    await act(async () => {
      container.querySelector<HTMLInputElement>('input[type="checkbox"]')?.click();
    });

    expect(localStorage.getItem("mpb.autoUpdateChecks")).toBe("false");
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

function buttonByLabel(container: HTMLElement, label: string): HTMLButtonElement {
  const button = [...container.querySelectorAll("button")].find(
    (item) => item.getAttribute("aria-label") === label,
  );
  if (!(button instanceof HTMLButtonElement)) {
    throw new Error(`Button with label "${label}" was not found`);
  }
  return button;
}

async function screenText(container: HTMLElement, text: string): Promise<boolean> {
  await act(async () => {
    await Promise.resolve();
  });
  return container.textContent?.includes(text) ?? false;
}

function countText(container: HTMLElement, text: string): number {
  return (container.textContent?.split(text).length ?? 1) - 1;
}

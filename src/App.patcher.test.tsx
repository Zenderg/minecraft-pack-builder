// @vitest-environment jsdom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const tauriMocks = vi.hoisted(() => ({
  discoverPrismLauncherRoots: vi.fn(),
  listPatcherInstances: vi.fn(),
  patchPrismInstance: vi.fn(),
  removePrismInstancePatch: vi.fn(),
}));

vi.mock("./tauri", () => ({
  discoverPrismLauncherRoots: tauriMocks.discoverPrismLauncherRoots,
  listPatcherInstances: tauriMocks.listPatcherInstances,
  patchPrismInstance: tauriMocks.patchPrismInstance,
  removePrismInstancePatch: tauriMocks.removePrismInstancePatch,
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

import { App } from "./App";

describe("MPB patcher app", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    (globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT =
      true;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    tauriMocks.discoverPrismLauncherRoots.mockResolvedValue([
      {
        rootPath: "/Prism",
        valid: true,
        message: "ok",
        instanceCount: 1,
        instances: [],
      },
    ]);
    tauriMocks.listPatcherInstances.mockResolvedValue([fabricInstance("notPatched")]);
    tauriMocks.patchPrismInstance.mockResolvedValue({ status: "patched", steps: [] });
    tauriMocks.removePrismInstancePatch.mockResolvedValue({ status: "notPatched", steps: [] });
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.clearAllMocks();
  });

  it("renders Prism instances with patch status after launcher root detection", async () => {
    await act(async () => {
      root.render(<App />);
    });
    await flush();

    expect(container.textContent).toContain("MPB Patcher");
    expect(container.textContent).toContain("/Prism");
    expect(container.textContent).toContain("Factory Pack");
    expect(container.textContent).toContain("Not patched");
    expect(container.textContent).toContain("Minecraft");
    expect(container.textContent).toContain("Fabric");
  });

  it("applies the patch and shows the Minecraft next step", async () => {
    tauriMocks.listPatcherInstances
      .mockResolvedValueOnce([fabricInstance("notPatched")])
      .mockResolvedValueOnce([fabricInstance("patched")]);

    await act(async () => {
      root.render(<App />);
    });
    await flush();

    await act(async () => {
      buttonByText(container, "Apply patch").click();
    });
    await flush();

    expect(tauriMocks.patchPrismInstance).toHaveBeenCalledWith(
      "/Prism/instances/Factory Pack",
      "apply",
    );
    expect(container.textContent).toContain("Patched");
    expect(container.textContent).toContain("start the instance in PrismLauncher");
    expect(container.textContent).toContain("/mpb");
  });
});

function fabricInstance(patchStatus: "notPatched" | "patched") {
  return {
    instanceId: "factory-pack",
    displayName: "Factory Pack",
    instancePath: "/Prism/instances/Factory Pack",
    minecraftDir: "/Prism/instances/Factory Pack/.minecraft",
    minecraftVersion: "1.20.1",
    loader: "Fabric",
    loaderVersion: "0.16.9",
    patchStatus,
    patchReason: null,
  };
}

async function flush() {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
}

function buttonByText(container: HTMLElement, text: string): HTMLButtonElement {
  const button = [...container.querySelectorAll("button")].find((candidate) =>
    candidate.textContent?.includes(text),
  );
  if (!(button instanceof HTMLButtonElement)) {
    throw new Error(`Button with text "${text}" was not found`);
  }
  return button;
}

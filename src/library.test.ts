import { describe, expect, it } from "vitest";

import {
  clampSidebarWidth,
  createEmptyLibraryDraft,
  getCompactLibraryNodeGap,
  getActiveLibrarySelection,
  getInitialExpandedModpackIds,
  getLoaderIconKind,
  getLibraryDialogContent,
  getModpackMenuPlacement,
  getNextSelectionAfterSchemeDelete,
  getNextOpenModpackMenuId,
  toggleExpandedModpack,
  type LibraryModpack,
} from "./library";

const library: LibraryModpack[] = [
  {
    id: 1,
    instanceId: "all-the-mods-10",
    displayName: "All the Mods 10",
    instancePath: "/PrismLauncher/instances/all-the-mods-10",
    minecraftDir: "/PrismLauncher/instances/all-the-mods-10/.minecraft",
    minecraftVersion: "1.20.1",
    loader: "Forge",
    loaderVersion: "47.4.0",
    status: "ready",
    statusMessage: null,
    schemes: [
      { id: 10, modpackId: 1, name: "Starter Factory", dimensions: [64, 64, 64] },
      { id: 11, modpackId: 1, name: "Tower", dimensions: [32, 96, 32] },
    ],
  },
  {
    id: 2,
    instanceId: "aoc",
    displayName: "AOC",
    instancePath: "/PrismLauncher/instances/aoc",
    minecraftDir: "/PrismLauncher/instances/aoc/.minecraft",
    minecraftVersion: "1.19.2",
    loader: "Forge",
    loaderVersion: "43.5.0",
    status: "ready",
    statusMessage: null,
    schemes: [],
  },
];

describe("phase 3 library ui state", () => {
  it("selects the first scheme in the first imported modpack by default", () => {
    expect(getActiveLibrarySelection(library, null)).toEqual({
      modpackId: 1,
      schemeId: 10,
    });
  });

  it("keeps a requested scheme selected when it still exists", () => {
    expect(getActiveLibrarySelection(library, { modpackId: 1, schemeId: 11 })).toEqual({
      modpackId: 1,
      schemeId: 11,
    });
  });

  it("clears a deleted scheme selection to the next scheme in the same modpack", () => {
    expect(getNextSelectionAfterSchemeDelete(library, { modpackId: 1, schemeId: 10 })).toEqual({
      modpackId: 1,
      schemeId: 11,
    });
  });

  it("starts create-scheme drafts with phase 3 default dimensions", () => {
    expect(createEmptyLibraryDraft(1)).toEqual({
      modpackId: 1,
      name: "New scheme",
      dimensions: [64, 64, 64],
    });
  });

  it("opens imported modpack nodes by default", () => {
    expect(getInitialExpandedModpackIds(library)).toEqual(new Set([1, 2]));
  });

  it("toggles one imported modpack node without changing the others", () => {
    const collapsed = toggleExpandedModpack(new Set([1, 2]), 1);
    expect(collapsed).toEqual(new Set([2]));
    expect(toggleExpandedModpack(collapsed, 1)).toEqual(new Set([1, 2]));
  });

  it("clamps the resizable sidebar to fixed min and max widths", () => {
    expect(clampSidebarWidth(180)).toBe(248);
    expect(clampSidebarWidth(320)).toBe(320);
    expect(clampSidebarWidth(620)).toBe(420);
  });

  it("maps known Minecraft loaders to sidebar icon variants", () => {
    expect(getLoaderIconKind("Forge")).toBe("forge");
    expect(getLoaderIconKind("NeoForge")).toBe("neoforge");
    expect(getLoaderIconKind("Fabric")).toBe("fabric");
    expect(getLoaderIconKind("Quilt")).toBe("quilt");
    expect(getLoaderIconKind(null)).toBe("generic");
    expect(getLoaderIconKind("Unknown Loader")).toBe("generic");
  });

  it("closes the open modpack menu when clicking outside it", () => {
    expect(getNextOpenModpackMenuId(1, 1, "menuButton")).toBe(null);
    expect(getNextOpenModpackMenuId(1, 2, "menuButton")).toBe(2);
    expect(getNextOpenModpackMenuId(1, null, "outside")).toBe(null);
    expect(getNextOpenModpackMenuId(1, null, "menuSurface")).toBe(1);
  });

  it("uses a compact vertical gap between imported modpacks", () => {
    expect(getCompactLibraryNodeGap()).toBe(2);
  });

  it("places the modpack menu outside clipped library containers", () => {
    expect(
      getModpackMenuPlacement(
        { left: 430, right: 488, top: 95, bottom: 145 },
        { width: 520, height: 600 },
      ),
    ).toEqual({ left: 320, top: 153 });
  });

  it("flips the modpack menu above the trigger near the viewport bottom", () => {
    expect(
      getModpackMenuPlacement(
        { left: 430, right: 488, top: 480, bottom: 516 },
        { width: 520, height: 600 },
      ),
    ).toEqual({ left: 320, top: 362 });
  });

  it("keeps form dialog body separate from the name field label", () => {
    expect(getLibraryDialogContent("createScheme")).not.toHaveProperty("bodyKey");
  });

  it("renders delete dialogs as danger confirmations with body copy", () => {
    expect(getLibraryDialogContent("deleteScheme")).toEqual({
      titleKey: "library.deleteSchemeTitle",
      bodyKey: "library.confirmDeleteScheme",
      tone: "danger",
    });
  });
});

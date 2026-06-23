import { describe, expect, it } from "vitest";

import {
  createInitialImportWizardState,
  createInitialProjectSearchQuery,
  getDebouncedSearchQuery,
  getDefaultReleaseId,
  getFilteredReleases,
  getProjectLogoUrl,
  getNextSelectedReleaseId,
  getProjectSourceUrl,
  getReleaseImportState,
  getReleasePrimaryActionLabel,
  importWizardReducer,
  isImportWizardBusy,
  shouldSearchModpacks,
  statusText,
  type CurseForgeProject,
  type CurseForgeReleaseSummary,
} from "./importWizard";
import type { LibraryModpack } from "./library";

const releases: CurseForgeReleaseSummary[] = [
  {
    fileId: 100,
    versionName: "AOC 1.0.0",
    fileName: "aoc-1.0.0.zip",
    minecraftVersions: ["1.19.2"],
    loaders: ["Forge"],
    fileDate: "2026-01-01T00:00:00Z",
    fileLength: 12,
  },
  {
    fileId: 200,
    versionName: "AOC 1.1.0",
    fileName: "aoc-1.1.0.zip",
    minecraftVersions: ["1.20.1"],
    loaders: ["Forge"],
    fileDate: "2026-02-01T00:00:00Z",
    fileLength: 12,
  },
  {
    fileId: 300,
    versionName: "AOC 1.2.0",
    fileName: "aoc-1.2.0.zip",
    minecraftVersions: ["1.20.1"],
    loaders: ["NeoForge"],
    fileDate: "2026-03-01T00:00:00Z",
    fileLength: 12,
  },
];

const projects: CurseForgeProject[] = [
  { id: 42, name: "AOC", slug: "aoc", logoUrl: "https://example.test/aoc.png" },
  { id: 84, name: "Better MC", slug: "better-mc", logoUrl: null },
];

describe("phase 5 import wizard state", () => {
  it("selects the latest release by default", () => {
    expect(getDefaultReleaseId(releases)).toBe(300);
  });

  it("filters releases by Minecraft version and loader", () => {
    expect(
      getFilteredReleases(releases, {
        minecraftVersion: "1.20.1",
        loader: "Forge",
      }).map((release) => release.fileId),
    ).toEqual([200]);
  });

  it("moves selection to the latest visible release when filters hide the previous selection", () => {
    const filtered = getFilteredReleases(releases, {
      minecraftVersion: "1.20.1",
      loader: "Forge",
    });

    expect(getNextSelectedReleaseId(300, filtered)).toBe(200);
  });

  it("keeps cancellation as a user-visible failed state without an imported entry", () => {
    const state = importWizardReducer(createInitialImportWizardState(), {
      type: "downloadCancelled",
    });

    expect(state.status).toBe("failed");
    expect(state.message).toMatch(/cancel/i);
    expect(state.importedModpackId).toBeNull();
  });

  it("marks release discovery and download as busy states", () => {
    expect(isImportWizardBusy("discovering")).toBe(true);
    expect(isImportWizardBusy("downloading")).toBe(true);
    expect(isImportWizardBusy("idle")).toBe(false);
    expect(isImportWizardBusy("ready")).toBe(false);
    expect(isImportWizardBusy("success")).toBe(false);
    expect(isImportWizardBusy("failed")).toBe(false);
  });

  it("searches modpacks only after a meaningful debounced query", () => {
    expect(createInitialProjectSearchQuery()).toBe("");
    expect(shouldSearchModpacks("a")).toBe(false);
    expect(shouldSearchModpacks("  aoc  ")).toBe(true);
    expect(getDebouncedSearchQuery("  all of create  ")).toBe("all of create");
  });

  it("derives a CurseForge source URL from a selected project", () => {
    expect(getProjectSourceUrl(projects[0])).toBe(
      "https://www.curseforge.com/minecraft/modpacks/aoc",
    );
  });

  it("uses project thumbnails when CurseForge provides them", () => {
    expect(getProjectLogoUrl(projects[0])).toBe("https://example.test/aoc.png");
    expect(getProjectLogoUrl(projects[1])).toBeNull();
  });

  it("shows a release-ready status after a release is selected", () => {
    const t = (key: string) => key;

    expect(statusText(createInitialImportWizardState(), false, t)).toBe("import.readyBody");
    expect(statusText(importWizardReducer(createInitialImportWizardState(), { type: "releaseReady" }), true, t)).toBe(
      "import.releaseReady",
    );
  });

  it("treats already tracked releases as busy and labels the action as adding", () => {
    const trackedLibrary: LibraryModpack[] = [
      {
        id: 1,
        localName: "AOC - AOC 1.2.0",
        sourceUrl: "https://www.curseforge.com/minecraft/modpacks/aoc",
        versionName: "AOC 1.2.0",
        minecraftVersion: "1.20.1",
        loader: "NeoForge",
        importStatus: "importing",
        importMessage: "Downloading selected release...",
        schemes: [],
      },
    ];

    expect(getReleaseImportState(releases[2], trackedLibrary)).toBe("importing");
    expect(getReleaseImportState(releases[1], trackedLibrary)).toBe("none");
    expect(getReleasePrimaryActionLabel((key: string) => key)).toBe("import.addSelected");
  });
});

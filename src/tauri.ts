import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { LibraryModpack } from "./library";
import { browserDomainDemoArtifact, type DomainDemoArtifact } from "./phase4Demo";

export type AppDataPaths = {
  appDataDir: string;
  diagnosticsDir: string;
};

export type CurseForgeCredentialStatus = {
  state: "missing" | "saved" | "unavailable";
  backend: string;
  message?: string | null;
  apiKey?: null;
};

export type CurseForgeProject = {
  id: number;
  name: string;
  slug: string;
  logoUrl: string | null;
};

export type CurseForgeReleaseSummary = {
  fileId: number;
  versionName: string;
  fileName: string;
  minecraftVersions: string[];
  loaders: string[];
  fileDate: string;
  fileLength: number;
};

export type CurseForgeReleaseDiscovery = {
  modpack: CurseForgeProject;
  sourceUrl: string;
  releases: CurseForgeReleaseSummary[];
  minecraftVersions: string[];
  loaders: string[];
  defaultFileId: number;
};

export type ImportProgress = {
  modpackId: number;
  stage: string;
  bytesDownloaded: number;
  totalBytes: number | null;
  progressPercent: number | null;
};

export type ImportedModpackResult = {
  library: LibraryModpack[];
  modpackId: number;
  archivePath: string;
  assetReportPath: string;
};

export type ModpackImportStatusChanged = {
  modpackId: number;
  status: LibraryModpack["importStatus"];
  message: string | null;
  stage: string;
  library: LibraryModpack[];
};

function getProjectSourceUrl(project: CurseForgeProject): string {
  return `https://www.curseforge.com/minecraft/modpacks/${project.slug}`;
}

function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

export async function discoverAppPaths(): Promise<AppDataPaths | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<AppDataPaths>("discover_app_paths");
}

export async function openAppDataFolder(): Promise<AppDataPaths | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  return invoke<AppDataPaths>("open_app_data_folder");
}

export async function getCurseForgeKeyStatus(): Promise<CurseForgeCredentialStatus> {
  if (!isTauriRuntime()) {
    return {
      state: "saved",
      backend: "Browser demo fixture",
      message: "Desktop builds use OS secure storage",
      apiKey: null,
    };
  }

  return invoke<CurseForgeCredentialStatus>("get_curseforge_key_status");
}

export async function saveCurseForgeApiKey(apiKey: string): Promise<CurseForgeCredentialStatus> {
  if (!isTauriRuntime()) {
    if (apiKey.trim().length === 0 || apiKey.toLowerCase().includes("invalid")) {
      return {
        state: "unavailable",
        backend: "Browser demo fixture",
        message: "CurseForge did not accept this API key",
        apiKey: null,
      };
    }
    return {
      state: "saved",
      backend: "Browser demo fixture",
      message: "Desktop builds save this in OS secure storage",
      apiKey: null,
    };
  }

  return invoke<CurseForgeCredentialStatus>("save_curseforge_api_key", { apiKey });
}

export async function checkCurseForgeApiKey(apiKey: string): Promise<void> {
  if (!isTauriRuntime()) {
    if (apiKey.trim().length === 0 || apiKey.toLowerCase().includes("invalid")) {
      throw new Error("CurseForge did not accept this API key");
    }
    return;
  }

  return invoke<void>("check_curseforge_api_key", { apiKey });
}

const browserSeedLibrary: LibraryModpack[] = [
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
        dimensions: [64, 64, 64],
      },
    ],
  },
  {
    id: 2,
    localName: "AOC - 1.0.0 (2)",
    sourceUrl: "https://www.curseforge.com/minecraft/modpacks/aoc",
    versionName: "1.0.0",
    minecraftVersion: "1.20.1",
    loader: "Forge",
    importStatus: "imported",
    importMessage: null,
    schemes: [],
  },
];

let browserLibrary = structuredClone(browserSeedLibrary);
let browserNextSchemeId = 100;
let browserNextModpackId = 1000;

export async function listLibrary(): Promise<LibraryModpack[]> {
  if (!isTauriRuntime()) {
    return structuredClone(browserLibrary);
  }

  return invoke<LibraryModpack[]>("list_library");
}

export async function seedLocalLibraryFixture(): Promise<LibraryModpack[]> {
  if (!isTauriRuntime()) {
    browserLibrary = structuredClone(browserSeedLibrary);
    browserNextSchemeId = 100;
    return structuredClone(browserLibrary);
  }

  return invoke<LibraryModpack[]>("seed_local_library_fixture");
}

export async function createScheme(
  modpackId: number,
  name: string,
  dimensions: [number, number, number],
): Promise<LibraryModpack[]> {
  if (!isTauriRuntime()) {
    browserLibrary = browserLibrary.map((modpack) =>
      modpack.id === modpackId
        ? {
            ...modpack,
            schemes: [
              ...modpack.schemes,
              {
                id: browserNextSchemeId++,
                modpackId,
                name,
                dimensions,
              },
            ],
          }
        : modpack,
    );
    return structuredClone(browserLibrary);
  }

  return invoke<LibraryModpack[]>("create_scheme", {
    modpackId,
    name,
    sizeX: dimensions[0],
    sizeY: dimensions[1],
    sizeZ: dimensions[2],
  });
}

export async function renameScheme(schemeId: number, name: string): Promise<LibraryModpack[]> {
  if (!isTauriRuntime()) {
    browserLibrary = browserLibrary.map((modpack) => ({
      ...modpack,
      schemes: modpack.schemes.map((scheme) =>
        scheme.id === schemeId ? { ...scheme, name } : scheme,
      ),
    }));
    return structuredClone(browserLibrary);
  }

  return invoke<LibraryModpack[]>("rename_scheme", { schemeId, name });
}

export async function deleteScheme(schemeId: number): Promise<LibraryModpack[]> {
  if (!isTauriRuntime()) {
    browserLibrary = browserLibrary.map((modpack) => ({
      ...modpack,
      schemes: modpack.schemes.filter((scheme) => scheme.id !== schemeId),
    }));
    return structuredClone(browserLibrary);
  }

  return invoke<LibraryModpack[]>("delete_scheme", { schemeId });
}

export async function renameImportedModpack(
  modpackId: number,
  name: string,
): Promise<LibraryModpack[]> {
  if (!isTauriRuntime()) {
    browserLibrary = browserLibrary.map((modpack) =>
      modpack.id === modpackId ? { ...modpack, localName: browserUniqueModpackName(name, modpackId) } : modpack,
    );
    return structuredClone(browserLibrary);
  }

  return invoke<LibraryModpack[]>("rename_imported_modpack", { modpackId, name });
}

export async function deleteImportedModpack(modpackId: number): Promise<LibraryModpack[]> {
  if (!isTauriRuntime()) {
    browserLibrary = browserLibrary.filter((modpack) => modpack.id !== modpackId);
    return structuredClone(browserLibrary);
  }

  return invoke<LibraryModpack[]>("delete_imported_modpack", { modpackId });
}

export async function generateDomainDemoReport(): Promise<DomainDemoArtifact> {
  if (!isTauriRuntime()) {
    return structuredClone(browserDomainDemoArtifact);
  }

  return invoke<DomainDemoArtifact>("generate_domain_demo_report");
}

export async function searchCurseForgeModpacks(query: string): Promise<CurseForgeProject[]> {
  if (!isTauriRuntime()) {
    const normalized = query.trim().toLowerCase();
    if (normalized.length < 2) {
      return [];
    }
    return browserReleaseDiscoveries
      .map((discovery) => discovery.modpack)
      .filter((project) => {
        return (
          project.name.toLowerCase().includes(normalized) ||
          project.slug.toLowerCase().includes(normalized)
        );
      })
      .map((project) => structuredClone(project));
  }

  return invoke<CurseForgeProject[]>("search_curseforge_modpacks", { query });
}

export async function discoverCurseForgeReleases(
  pageUrl: string,
): Promise<CurseForgeReleaseDiscovery> {
  if (!isTauriRuntime()) {
    const discovery = getBrowserReleaseDiscovery(pageUrl);
    if (!discovery) {
      throw new Error("only CurseForge modpack page URLs are supported");
    }
    return structuredClone(discovery);
  }

  return invoke<CurseForgeReleaseDiscovery>("discover_curseforge_releases", { pageUrl });
}

export async function importCurseForgeModpack(
  pageUrl: string,
  fileId: number,
  onProgress: (progress: ImportProgress) => void,
): Promise<ImportedModpackResult> {
  if (!isTauriRuntime()) {
    const discovery = getBrowserReleaseDiscovery(pageUrl);
    if (!discovery) {
      throw new Error("only CurseForge modpack page URLs are supported");
    }
    const release = discovery.releases.find((item) => item.fileId === fileId);
    if (!release) {
      throw new Error(`release file ${fileId} was not found`);
    }
    onProgress({
      modpackId: browserNextModpackId,
      stage: "download",
      bytesDownloaded: Math.floor(release.fileLength / 2),
      totalBytes: release.fileLength,
      progressPercent: 20,
    });
    onProgress({
      modpackId: browserNextModpackId,
      stage: "download",
      bytesDownloaded: release.fileLength,
      totalBytes: release.fileLength,
      progressPercent: 30,
    });
    const imported: LibraryModpack = {
      id: browserNextModpackId++,
      localName: browserUniqueNewModpackName(`${discovery.modpack.name} - ${release.versionName}`),
      sourceUrl: pageUrl,
      versionName: release.versionName,
      minecraftVersion: release.minecraftVersions[0] ?? null,
      loader: release.loaders[0] ?? null,
      importStatus: "importing",
      importMessage: "Adding selected release...",
      schemes: [],
    };
    browserLibrary = [...browserLibrary, imported];
    window.setTimeout(() => {
      browserLibrary = browserLibrary.map((modpack) =>
        modpack.id === imported.id
          ? {
              ...modpack,
              importStatus: "imported",
              importMessage: "Ready",
            }
          : modpack,
      );
    }, 1200);
    return {
      library: structuredClone(browserLibrary),
      modpackId: imported.id,
      archivePath: `/browser-demo/modpacks/${discovery.modpack.slug}-${release.fileId}/${release.fileName}`,
      assetReportPath: `/browser-demo/diagnostics/${discovery.modpack.slug}-${release.fileId}-assets.json`,
    };
  }

  const unlisten = await listen<ImportProgress>("modpack_import_progress", (event) => {
    onProgress(event.payload);
  });
  try {
    return await invoke<ImportedModpackResult>("import_curseforge_modpack", { pageUrl, fileId });
  } finally {
    unlisten();
  }
}

export async function retryModpackImport(modpackId: number): Promise<LibraryModpack[]> {
  if (!isTauriRuntime()) {
    browserLibrary = browserLibrary.map((modpack) =>
      modpack.id === modpackId
        ? { ...modpack, importStatus: "importing", importMessage: "Retry queued..." }
        : modpack,
    );
    return structuredClone(browserLibrary);
  }

  return invoke<LibraryModpack[]>("retry_modpack_import", { modpackId });
}

export async function listenToModpackImportStatus(
  onChanged: (event: ModpackImportStatusChanged) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }

  return listen<ModpackImportStatusChanged>("modpack_import_status_changed", (event) => {
    onChanged(event.payload);
  });
}

export async function listenToModpackImportProgress(
  onProgress: (event: ImportProgress) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }

  return listen<ImportProgress>("modpack_import_progress", (event) => {
    onProgress(event.payload);
  });
}

export async function cancelCurseForgeImport(): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }

  return invoke<void>("cancel_curseforge_import");
}

function browserUniqueModpackName(requestedName: string, excludingId: number): string {
  const baseName = requestedName.trim() || "Imported modpack";
  let candidate = baseName;
  let suffix = 2;

  while (
    browserLibrary.some((modpack) => modpack.id !== excludingId && modpack.localName === candidate)
  ) {
    candidate = `${baseName} (${suffix})`;
    suffix += 1;
  }

  return candidate;
}

function browserUniqueNewModpackName(requestedName: string): string {
  const baseName = requestedName.trim() || "Imported modpack";
  let candidate = baseName;
  let suffix = 2;

  while (browserLibrary.some((modpack) => modpack.localName === candidate)) {
    candidate = `${baseName} (${suffix})`;
    suffix += 1;
  }

  return candidate;
}

function getBrowserReleaseDiscovery(pageUrl: string): CurseForgeReleaseDiscovery | null {
  return (
    browserReleaseDiscoveries.find((discovery) => discovery.sourceUrl === pageUrl) ??
    browserReleaseDiscoveries.find((discovery) => pageUrl.endsWith(`/minecraft/modpacks/${discovery.modpack.slug}`)) ??
    null
  );
}

const browserReleaseDiscoveries: CurseForgeReleaseDiscovery[] = [
{
  modpack: {
    id: 42,
    name: "AOC",
    slug: "aoc",
    logoUrl: "https://media.forgecdn.net/avatars/468/937/637751618537532468.png",
  },
  sourceUrl: getProjectSourceUrl({ id: 42, name: "AOC", slug: "aoc", logoUrl: null }),
  defaultFileId: 300,
  minecraftVersions: ["1.20.1", "1.19.2"],
  loaders: ["Forge", "NeoForge"],
  releases: [
    {
      fileId: 300,
      versionName: "AOC 1.2.0",
      fileName: "aoc-1.2.0.zip",
      minecraftVersions: ["1.20.1"],
      loaders: ["NeoForge"],
      fileDate: "2026-03-01T00:00:00Z",
      fileLength: 12_000_000,
    },
    {
      fileId: 200,
      versionName: "AOC 1.1.0",
      fileName: "aoc-1.1.0.zip",
      minecraftVersions: ["1.20.1"],
      loaders: ["Forge"],
      fileDate: "2026-02-01T00:00:00Z",
      fileLength: 10_000_000,
    },
    {
      fileId: 100,
      versionName: "AOC 1.0.0",
      fileName: "aoc-1.0.0.zip",
      minecraftVersions: ["1.19.2"],
      loaders: ["Forge"],
      fileDate: "2026-01-01T00:00:00Z",
      fileLength: 8_000_000,
    },
  ],
},
{
  modpack: {
    id: 84,
    name: "Better MC",
    slug: "better-mc",
    logoUrl: "https://media.forgecdn.net/avatars/359/694/637526918651898716.png",
  },
  sourceUrl: getProjectSourceUrl({ id: 84, name: "Better MC", slug: "better-mc", logoUrl: null }),
  defaultFileId: 420,
  minecraftVersions: ["1.21.1", "1.20.1"],
  loaders: ["Fabric", "Forge"],
  releases: [
    {
      fileId: 420,
      versionName: "Better MC 4.0.0",
      fileName: "better-mc-4.0.0.zip",
      minecraftVersions: ["1.21.1"],
      loaders: ["Fabric"],
      fileDate: "2026-04-01T00:00:00Z",
      fileLength: 20_000_000,
    },
    {
      fileId: 410,
      versionName: "Better MC 3.5.0",
      fileName: "better-mc-3.5.0.zip",
      minecraftVersions: ["1.20.1"],
      loaders: ["Forge"],
      fileDate: "2026-03-11T00:00:00Z",
      fileLength: 18_000_000,
    },
  ],
},
];

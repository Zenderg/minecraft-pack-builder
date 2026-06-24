import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { LibraryModpack } from "./library";
import type { RenderScene } from "./renderViewer";

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

export type AgentStatus = {
  serverRunning: boolean;
  transport: string;
  endpoint: string | null;
  protocolVersion: string;
  activeClient: string | null;
  toolCount: number;
};

export type UpdateCheckResult = {
  status: "current" | "available" | "failed";
  currentVersion: string;
  latestVersion: string | null;
  notes: string | null;
  date: string | null;
  errorMessage: string | null;
};

export type AgentEvent =
  | { libraryChanged: Record<string, never> }
  | { schemeChanged: { schemeId: number } };

export type ExportFormat = "schem" | "litematic";

export type ExportArtifact = {
  path: string;
  format: ExportFormat;
  byteLen: number;
  blockCount: number;
  schemeId?: number;
};

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
      backend: "Browser fallback",
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
        backend: "Browser fallback",
        message: "CurseForge did not accept this API key",
        apiKey: null,
      };
    }
    return {
      state: "saved",
      backend: "Browser fallback",
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

let browserLibrary: LibraryModpack[] = [];
let browserNextSchemeId = 100;

export async function listLibrary(): Promise<LibraryModpack[]> {
  if (!isTauriRuntime()) {
    return structuredClone(browserLibrary);
  }

  return invoke<LibraryModpack[]>("list_library");
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

export async function getSchemeRenderScene(schemeId: number): Promise<RenderScene> {
  if (!isTauriRuntime()) {
    return {
      schemeId,
      schemeName: "Unsaved scheme",
      dimensions: [1, 1, 1],
      stages: [],
      blocks: [],
      chunks: [],
      largeSchemeThreshold: 4096,
    };
  }

  return invoke<RenderScene>("get_scheme_render_scene", { schemeId });
}

export async function exportScheme(
  schemeId: number,
  format: ExportFormat,
  destinationPath: string,
): Promise<ExportArtifact> {
  if (!isTauriRuntime()) {
    return {
      schemeId,
      format,
      path: destinationPath,
      byteLen: 128,
      blockCount: 0,
    };
  }

  return invoke<ExportArtifact>("export_scheme", {
    schemeId,
    format,
    destinationPath,
  });
}

export async function getAiIntegrationStatus(): Promise<AgentStatus> {
  if (!isTauriRuntime()) {
    return {
      serverRunning: true,
      transport: "streamable-http",
      endpoint: "http://127.0.0.1:47392/mcp",
      protocolVersion: "2025-06-18",
      activeClient: null,
      toolCount: 19,
    };
  }

  return invoke<AgentStatus>("get_ai_integration_status");
}

export async function checkForUpdates(): Promise<UpdateCheckResult> {
  if (!isTauriRuntime()) {
    return {
      status: "current",
      currentVersion: "0.1.0",
      latestVersion: null,
      notes: null,
      date: null,
      errorMessage: null,
    };
  }

  return invoke<UpdateCheckResult>("check_for_updates");
}

export async function listenToAgentEvents(
  onEvent: (event: AgentEvent) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }

  return listen<AgentEvent>("ai_agent_event", (event) => {
    onEvent(event.payload);
  });
}

export async function searchCurseForgeModpacks(query: string): Promise<CurseForgeProject[]> {
  if (!isTauriRuntime()) {
    return [];
  }

  return invoke<CurseForgeProject[]>("search_curseforge_modpacks", { query });
}

export async function discoverCurseForgeReleases(
  pageUrl: string,
): Promise<CurseForgeReleaseDiscovery> {
  if (!isTauriRuntime()) {
    throw new Error("CurseForge release discovery requires the Tauri desktop app");
  }

  return invoke<CurseForgeReleaseDiscovery>("discover_curseforge_releases", { pageUrl });
}

export async function importCurseForgeModpack(
  pageUrl: string,
  fileId: number,
  onProgress: (progress: ImportProgress) => void,
): Promise<ImportedModpackResult> {
  if (!isTauriRuntime()) {
    void onProgress;
    void pageUrl;
    void fileId;
    throw new Error("CurseForge import requires the Tauri desktop app");
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

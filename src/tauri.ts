import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { LibraryModpack, LibraryScheme, PrismInstanceStatus } from "./library";
import type { RenderScene } from "./renderViewer";

export type AppDataPaths = {
  appDataDir: string;
  diagnosticsDir: string;
};

export type PrismInstanceDescriptor = {
  instanceId: string;
  displayName: string;
  instancePath: string;
  minecraftDir: string;
  minecraftVersion: string | null;
  loader: string | null;
  loaderVersion: string | null;
  identityFingerprint: string;
  contentFingerprint: string;
  status: PrismInstanceStatus;
  statusMessage: string | null;
};

export type PrismRootValidation = {
  rootPath: string;
  valid: boolean;
  message: string;
  instanceCount: number;
  instances: PrismInstanceDescriptor[];
};

export type PrismRootSelection = {
  validation: PrismRootValidation;
  library: LibraryModpack[];
  relinkCandidates: PrismRelinkCandidate[];
};

export type PrismRelinkCandidate = {
  existingId: number;
  existingDisplayName: string;
  existingInstancePath: string;
  discoveredIdentityFingerprint: string;
  discoveredDisplayName: string;
  discoveredInstancePath: string;
  minecraftVersion: string | null;
  loader: string | null;
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

export type LibraryChangedEvent = {
  library: LibraryModpack[];
};

function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

function normalizeLibrary(library: LibraryModpack[]): LibraryModpack[] {
  return library.map((instance) => ({
    ...instance,
    schemes: instance.schemes.map((scheme: LibraryScheme) => ({
      ...scheme,
      modpackId: scheme.modpackId ?? scheme.prismInstanceId ?? instance.id,
    })),
  }));
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

export async function discoverPrismLauncherRoots(): Promise<PrismRootValidation[]> {
  if (!isTauriRuntime()) {
    return [];
  }

  return invoke<PrismRootValidation[]>("discover_prism_launcher_roots");
}

export async function validatePrismLauncherRoot(rootPath: string): Promise<PrismRootValidation> {
  if (!isTauriRuntime()) {
    return {
      rootPath,
      valid: false,
      message: "Desktop builds validate PrismLauncher Launcher Root folders.",
      instanceCount: 0,
      instances: [],
    };
  }

  return invoke<PrismRootValidation>("validate_prism_launcher_root", { rootPath });
}

export async function selectPrismLauncherRoot(rootPath: string): Promise<PrismRootSelection> {
  if (!isTauriRuntime()) {
    return {
      validation: await validatePrismLauncherRoot(rootPath),
      library: [],
      relinkCandidates: [],
    };
  }

  const selection = await invoke<PrismRootSelection>("select_prism_launcher_root", { rootPath });
  return {
    ...selection,
    library: normalizeLibrary(selection.library),
    relinkCandidates: selection.relinkCandidates ?? [],
  };
}

let browserLibrary: LibraryModpack[] = [];
let browserNextSchemeId = 100;

export async function listLibrary(): Promise<LibraryModpack[]> {
  if (!isTauriRuntime()) {
    return structuredClone(browserLibrary);
  }

  return normalizeLibrary(await invoke<LibraryModpack[]>("list_library"));
}

export async function listPrismRelinkCandidates(): Promise<PrismRelinkCandidate[]> {
  if (!isTauriRuntime()) {
    return [];
  }

  return invoke<PrismRelinkCandidate[]>("list_prism_relink_candidates");
}

export async function confirmPrismInstanceRelink(
  existingId: number,
  discoveredIdentityFingerprint: string,
): Promise<LibraryModpack[]> {
  if (!isTauriRuntime()) {
    return structuredClone(browserLibrary);
  }

  return normalizeLibrary(
    await invoke<LibraryModpack[]>("confirm_prism_instance_relink", {
      existingId,
      discoveredIdentityFingerprint,
    }),
  );
}

export async function createScheme(
  modpackId: number,
  name: string,
  dimensions: [number, number, number],
): Promise<LibraryModpack[]> {
  if (!isTauriRuntime()) {
    browserLibrary = browserLibrary.map((instance) =>
      instance.id === modpackId && instance.status === "ready"
        ? {
            ...instance,
            schemes: [
              ...instance.schemes,
              {
                id: browserNextSchemeId++,
                modpackId,
                name,
                dimensions,
              },
            ],
          }
        : instance,
    );
    return structuredClone(browserLibrary);
  }

  return normalizeLibrary(
    await invoke<LibraryModpack[]>("create_scheme", {
      prismInstanceId: modpackId,
      name,
      sizeX: dimensions[0],
      sizeY: dimensions[1],
      sizeZ: dimensions[2],
    }),
  );
}

export async function renameScheme(schemeId: number, name: string): Promise<LibraryModpack[]> {
  if (!isTauriRuntime()) {
    browserLibrary = browserLibrary.map((instance) => ({
      ...instance,
      schemes: instance.schemes.map((scheme) =>
        scheme.id === schemeId ? { ...scheme, name } : scheme,
      ),
    }));
    return structuredClone(browserLibrary);
  }

  return normalizeLibrary(await invoke<LibraryModpack[]>("rename_scheme", { schemeId, name }));
}

export async function deleteScheme(schemeId: number): Promise<LibraryModpack[]> {
  if (!isTauriRuntime()) {
    browserLibrary = browserLibrary.map((instance) => ({
      ...instance,
      schemes: instance.schemes.filter((scheme) => scheme.id !== schemeId),
    }));
    return structuredClone(browserLibrary);
  }

  return normalizeLibrary(await invoke<LibraryModpack[]>("delete_scheme", { schemeId }));
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
      toolCount: 18,
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

export async function listenToLibraryChanged(
  onChanged: (event: LibraryChangedEvent) => void,
): Promise<() => void> {
  if (!isTauriRuntime()) {
    return () => {};
  }

  return listen<LibraryChangedEvent>("library_changed", (event) => {
    onChanged({ library: normalizeLibrary(event.payload.library) });
  });
}

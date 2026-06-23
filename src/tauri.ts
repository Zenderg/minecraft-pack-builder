import { invoke } from "@tauri-apps/api/core";
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
      state: "unavailable",
      backend: "Desktop secure storage",
      message: "Available in the desktop app",
      apiKey: null,
    };
  }

  return invoke<CurseForgeCredentialStatus>("get_curseforge_key_status");
}

export async function saveCurseForgeApiKey(apiKey: string): Promise<CurseForgeCredentialStatus> {
  if (!isTauriRuntime()) {
    return {
      state: "unavailable",
      backend: "Desktop secure storage",
      message: "Available in the desktop app",
      apiKey: null,
    };
  }

  return invoke<CurseForgeCredentialStatus>("save_curseforge_api_key", { apiKey });
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
    schemes: [],
  },
];

let browserLibrary = structuredClone(browserSeedLibrary);
let browserNextSchemeId = 100;

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

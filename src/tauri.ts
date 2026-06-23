import { invoke } from "@tauri-apps/api/core";

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

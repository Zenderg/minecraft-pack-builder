import { invoke } from "@tauri-apps/api/core";

export type AppDataPaths = {
  appDataDir: string;
  diagnosticsDir: string;
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

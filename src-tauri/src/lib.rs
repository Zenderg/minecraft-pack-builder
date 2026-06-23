use std::process::Command;

use mpb_storage::{ensure_app_data_dirs, AppDataPaths};
use tauri::Manager;

#[tauri::command]
fn discover_app_paths(app: tauri::AppHandle) -> Result<AppDataPaths, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not discover app data directory: {error}"))?;

    ensure_app_data_dirs(app_data_dir).map_err(|error| error.to_string())
}

#[tauri::command]
fn open_app_data_folder(app: tauri::AppHandle) -> Result<AppDataPaths, String> {
    let paths = discover_app_paths(app)?;

    Command::new(open_folder_command_for_platform())
        .arg(&paths.app_data_dir)
        .spawn()
        .map_err(|error| format!("Could not open app data folder: {error}"))?;

    Ok(paths)
}

pub fn open_folder_command_for_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    }
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            discover_app_paths,
            open_app_data_folder
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Minecraft Pack Builder desktop app");
}

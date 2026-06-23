use std::process::Command;

use credentials::{curseforge_key_status, save_curseforge_key, CurseForgeCredentialStatus};
#[cfg(debug_assertions)]
use mpb_storage::{ImportStatus, NewImportedModpack};
use mpb_storage::{ensure_app_data_dirs, AppDataPaths, LibraryModpack, LibraryRepository, NewScheme};
use tauri::Manager;

mod credentials;

#[tauri::command]
fn discover_app_paths(app: tauri::AppHandle) -> Result<AppDataPaths, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not discover app data directory: {error}"))?;

    ensure_app_data_dirs(app_data_dir).map_err(|error| error.to_string())
}

fn library_repository(app: &tauri::AppHandle) -> Result<(AppDataPaths, LibraryRepository), String> {
    let paths = discover_app_paths(app.clone())?;
    let database_path = paths.app_data_dir.join("library.sqlite3");
    let database =
        mpb_storage::LibraryDatabase::open(database_path).map_err(|error| error.to_string())?;
    Ok((paths, LibraryRepository::new(database)))
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

#[tauri::command]
fn get_curseforge_key_status() -> CurseForgeCredentialStatus {
    curseforge_key_status()
}

#[tauri::command]
fn save_curseforge_api_key(api_key: String) -> Result<CurseForgeCredentialStatus, String> {
    save_curseforge_key(&api_key).map_err(|error| error.to_string())
}

#[tauri::command]
fn list_library(app: tauri::AppHandle) -> Result<Vec<LibraryModpack>, String> {
    let (_, repository) = library_repository(&app)?;
    repository
        .list_library()
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[cfg(debug_assertions)]
fn seed_local_library_fixture(app: tauri::AppHandle) -> Result<Vec<LibraryModpack>, String> {
    let (paths, repository) = library_repository(&app)?;
    let existing = repository
        .list_library()
        .map_err(|error| error.to_string())?;
    if !existing.is_empty() {
        return Ok(existing);
    }

    let first_cache = paths.app_data_dir.join("modpacks").join("aoc");
    let second_cache = paths.app_data_dir.join("modpacks").join("aoc-duplicate");
    std::fs::create_dir_all(&first_cache).map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&second_cache).map_err(|error| error.to_string())?;

    let first = repository
        .create_imported_modpack(NewImportedModpack {
            local_name: "AOC - 1.0.0".to_string(),
            source_slug: Some("aoc".to_string()),
            source_url: Some("https://www.curseforge.com/minecraft/modpacks/aoc".to_string()),
            version_name: "1.0.0".to_string(),
            minecraft_version: Some("1.20.1".to_string()),
            loader: Some("Forge".to_string()),
            cache_dir: Some(first_cache),
            import_status: ImportStatus::Imported,
        })
        .map_err(|error| error.to_string())?;
    repository
        .create_scheme(NewScheme {
            modpack_id: first.id,
            name: "Starter Factory".to_string(),
            size_x: 64,
            size_y: 64,
            size_z: 64,
        })
        .map_err(|error| error.to_string())?;
    repository
        .create_imported_modpack(NewImportedModpack {
            local_name: "AOC - 1.0.0".to_string(),
            source_slug: Some("aoc".to_string()),
            source_url: Some("https://www.curseforge.com/minecraft/modpacks/aoc".to_string()),
            version_name: "1.0.0".to_string(),
            minecraft_version: Some("1.20.1".to_string()),
            loader: Some("Forge".to_string()),
            cache_dir: Some(second_cache),
            import_status: ImportStatus::Imported,
        })
        .map_err(|error| error.to_string())?;

    repository
        .list_library()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn create_scheme(
    app: tauri::AppHandle,
    modpack_id: i64,
    name: String,
    size_x: i64,
    size_y: i64,
    size_z: i64,
) -> Result<Vec<LibraryModpack>, String> {
    let (_, repository) = library_repository(&app)?;
    repository
        .create_scheme(NewScheme {
            modpack_id,
            name,
            size_x,
            size_y,
            size_z,
        })
        .map_err(|error| error.to_string())?;
    repository
        .list_library()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn rename_scheme(
    app: tauri::AppHandle,
    scheme_id: i64,
    name: String,
) -> Result<Vec<LibraryModpack>, String> {
    let (_, repository) = library_repository(&app)?;
    repository
        .rename_scheme(scheme_id, &name)
        .map_err(|error| error.to_string())?;
    repository
        .list_library()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_scheme(app: tauri::AppHandle, scheme_id: i64) -> Result<Vec<LibraryModpack>, String> {
    let (_, repository) = library_repository(&app)?;
    repository
        .delete_scheme(scheme_id)
        .map_err(|error| error.to_string())?;
    repository
        .list_library()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn rename_imported_modpack(
    app: tauri::AppHandle,
    modpack_id: i64,
    name: String,
) -> Result<Vec<LibraryModpack>, String> {
    let (_, repository) = library_repository(&app)?;
    repository
        .rename_imported_modpack(modpack_id, &name)
        .map_err(|error| error.to_string())?;
    repository
        .list_library()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_imported_modpack(
    app: tauri::AppHandle,
    modpack_id: i64,
) -> Result<Vec<LibraryModpack>, String> {
    let (_, repository) = library_repository(&app)?;
    let deleted = repository
        .delete_imported_modpack(modpack_id)
        .map_err(|error| error.to_string())?;
    if let Some(cache_dir) = deleted.cache_dir {
        if cache_dir.exists() {
            std::fs::remove_dir_all(cache_dir).map_err(|error| error.to_string())?;
        }
    }
    repository
        .list_library()
        .map_err(|error| error.to_string())
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
    let builder = tauri::Builder::default();

    #[cfg(debug_assertions)]
    let builder = builder.invoke_handler(tauri::generate_handler![
            discover_app_paths,
            open_app_data_folder,
            get_curseforge_key_status,
            save_curseforge_api_key,
            list_library,
            seed_local_library_fixture,
            create_scheme,
            rename_scheme,
            delete_scheme,
            rename_imported_modpack,
            delete_imported_modpack
        ]);

    #[cfg(not(debug_assertions))]
    let builder = builder.invoke_handler(tauri::generate_handler![
            discover_app_paths,
            open_app_data_folder,
            get_curseforge_key_status,
            save_curseforge_api_key,
            list_library,
            create_scheme,
            rename_scheme,
            delete_scheme,
            rename_imported_modpack,
            delete_imported_modpack
        ]);

    builder
        .run(tauri::generate_context!())
        .expect("failed to run Minecraft Pack Builder desktop app");
}

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use credentials::{
    curseforge_key_status, read_curseforge_key, save_curseforge_key, CurseForgeCredentialStatus,
};
use mpb_assets::{
    discover_modpack_releases, download_release_archive, parse_modpack_page_url, CancellationToken,
    search_modpack_projects, CurseForgeGateway, CurseForgeHttpGateway, CurseForgeProject,
    DiscoveredReleases, DownloadProgress,
};
use mpb_core::DomainDemoReport;
use mpb_storage::{
    ensure_app_data_dirs, AppDataPaths, LibraryModpack, LibraryRepository, NewScheme,
};
use mpb_storage::{ImportStatus, NewImportedModpack};
use serde::Serialize;
use tauri::{Emitter, Manager};

mod credentials;

#[derive(Default)]
struct ImportController {
    current: Mutex<Option<CancellationToken>>,
}

impl ImportController {
    fn start(&self) -> Result<CancellationToken, String> {
        let token = CancellationToken::new();
        let mut current = self
            .current
            .lock()
            .map_err(|_| "import controller lock is poisoned".to_string())?;
        *current = Some(token.clone());
        Ok(token)
    }

    fn cancel(&self) -> Result<(), String> {
        let current = self
            .current
            .lock()
            .map_err(|_| "import controller lock is poisoned".to_string())?;
        if let Some(token) = current.as_ref() {
            token.cancel();
        }
        Ok(())
    }

    fn clear(&self) -> Result<(), String> {
        let mut current = self
            .current
            .lock()
            .map_err(|_| "import controller lock is poisoned".to_string())?;
        *current = None;
        Ok(())
    }
}

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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainDemoReportArtifact {
    pub path: PathBuf,
    pub report: DomainDemoReport,
}

pub fn write_domain_demo_report(
    diagnostics_dir: impl AsRef<Path>,
) -> Result<DomainDemoReportArtifact, String> {
    std::fs::create_dir_all(diagnostics_dir.as_ref()).map_err(|error| error.to_string())?;
    let report = mpb_core::domain_demo_report();
    let path = diagnostics_dir
        .as_ref()
        .join("phase-4-domain-demo-report.json");
    let json = serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?;
    std::fs::write(&path, json).map_err(|error| error.to_string())?;
    Ok(DomainDemoReportArtifact { path, report })
}

#[tauri::command]
fn generate_domain_demo_report(app: tauri::AppHandle) -> Result<DomainDemoReportArtifact, String> {
    let paths = discover_app_paths(app)?;
    write_domain_demo_report(paths.diagnostics_dir)
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
fn check_curseforge_api_key(api_key: String) -> Result<(), String> {
    credentials::validate_curseforge_api_key(&api_key).map_err(|error| error.to_string())?;
    let gateway = CurseForgeHttpGateway::new().map_err(|error| error.to_string())?;
    gateway
        .find_modpack_project(&api_key, "aoc")
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
fn discover_curseforge_releases(page_url: String) -> Result<DiscoveredReleases, String> {
    let api_key = read_curseforge_key().map_err(|error| error.to_string())?;
    let gateway = CurseForgeHttpGateway::new().map_err(|error| error.to_string())?;
    discover_modpack_releases(&gateway, &api_key, &page_url).map_err(|error| error.to_string())
}

#[tauri::command]
fn search_curseforge_modpacks(query: String) -> Result<Vec<CurseForgeProject>, String> {
    let api_key = read_curseforge_key().map_err(|error| error.to_string())?;
    let gateway = CurseForgeHttpGateway::new().map_err(|error| error.to_string())?;
    search_modpack_projects(&gateway, &api_key, &query).map_err(|error| error.to_string())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModpackImportProgress {
    bytes_downloaded: u64,
    total_bytes: Option<u64>,
}

impl From<DownloadProgress> for ModpackImportProgress {
    fn from(value: DownloadProgress) -> Self {
        Self {
            bytes_downloaded: value.bytes_downloaded,
            total_bytes: value.total_bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportedModpackResult {
    library: Vec<LibraryModpack>,
    modpack_id: i64,
    archive_path: PathBuf,
}

#[tauri::command]
fn import_curseforge_modpack(
    app: tauri::AppHandle,
    controller: tauri::State<ImportController>,
    page_url: String,
    file_id: u64,
) -> Result<ImportedModpackResult, String> {
    let api_key = read_curseforge_key().map_err(|error| error.to_string())?;
    let gateway = CurseForgeHttpGateway::new().map_err(|error| error.to_string())?;
    let parsed = parse_modpack_page_url(&page_url).map_err(|error| error.to_string())?;
    let project = gateway
        .find_modpack_project(&api_key, &parsed.slug)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!(
                "CurseForge modpack was not found for slug '{}'",
                parsed.slug
            )
        })?;
    let release = gateway
        .list_project_files(&api_key, project.id)
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|release| release.file_id == file_id)
        .ok_or_else(|| format!("CurseForge release file {file_id} was not found"))?;
    let summary = mpb_assets::filter_releases(
        &[mpb_assets::ReleaseSummary {
            file_id: release.file_id,
            version_name: release.display_name.clone(),
            file_name: release.file_name.clone(),
            minecraft_versions: release
                .game_versions
                .iter()
                .filter(|value| value.chars().next().is_some_and(|ch| ch.is_ascii_digit()))
                .cloned()
                .collect(),
            loaders: release
                .game_versions
                .iter()
                .filter(|value| matches!(value.as_str(), "Forge" | "NeoForge" | "Fabric" | "Quilt"))
                .cloned()
                .collect(),
            file_date: release.file_date.clone(),
            file_length: release.file_length,
        }],
        &mpb_assets::ReleaseFilter {
            minecraft_version: None,
            loader: None,
        },
    )
    .into_iter()
    .next()
    .cloned()
    .ok_or_else(|| "could not summarize selected release".to_string())?;

    let (paths, repository) = library_repository(&app)?;
    let safe_file_name = safe_path_segment(&release.file_name);
    let cache_dir = paths
        .app_data_dir
        .join("modpacks")
        .join(format!("{}-{}", parsed.slug, release.file_id));
    let archive_path = cache_dir.join("archives").join(safe_file_name);
    let token = controller.start()?;
    let emit_app = app.clone();
    let download_result = download_release_archive(
        &gateway,
        &api_key,
        &release,
        &archive_path,
        &token,
        |progress| {
            let _ = emit_app.emit(
                "modpack_import_progress",
                ModpackImportProgress::from(progress),
            );
        },
    );
    controller.clear()?;
    download_result.map_err(|error| error.to_string())?;

    let imported = repository
        .create_imported_modpack(NewImportedModpack {
            local_name: format!("{} - {}", project.name, summary.version_name),
            source_slug: Some(project.slug),
            source_url: Some(parsed.normalized_url),
            version_name: summary.version_name,
            minecraft_version: summary.minecraft_versions.first().cloned(),
            loader: summary.loaders.first().cloned(),
            cache_dir: Some(cache_dir),
            import_status: ImportStatus::Imported,
        })
        .map_err(|error| error.to_string())?;
    let library = repository
        .list_library()
        .map_err(|error| error.to_string())?;

    Ok(ImportedModpackResult {
        library,
        modpack_id: imported.id,
        archive_path,
    })
}

#[tauri::command]
fn cancel_curseforge_import(controller: tauri::State<ImportController>) -> Result<(), String> {
    controller.cancel()
}

#[tauri::command]
fn list_library(app: tauri::AppHandle) -> Result<Vec<LibraryModpack>, String> {
    let (_, repository) = library_repository(&app)?;
    repository.list_library().map_err(|error| error.to_string())
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

    repository.list_library().map_err(|error| error.to_string())
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
    repository.list_library().map_err(|error| error.to_string())
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
    repository.list_library().map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_scheme(app: tauri::AppHandle, scheme_id: i64) -> Result<Vec<LibraryModpack>, String> {
    let (_, repository) = library_repository(&app)?;
    repository
        .delete_scheme(scheme_id)
        .map_err(|error| error.to_string())?;
    repository.list_library().map_err(|error| error.to_string())
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
    repository.list_library().map_err(|error| error.to_string())
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
    repository.list_library().map_err(|error| error.to_string())
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

fn safe_path_segment(value: &str) -> String {
    let cleaned = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    if cleaned.is_empty() {
        "modpack.zip".to_string()
    } else {
        cleaned
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
        check_curseforge_api_key,
        discover_curseforge_releases,
        search_curseforge_modpacks,
        import_curseforge_modpack,
        cancel_curseforge_import,
        generate_domain_demo_report,
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
        check_curseforge_api_key,
        discover_curseforge_releases,
        search_curseforge_modpacks,
        import_curseforge_modpack,
        cancel_curseforge_import,
        generate_domain_demo_report,
        list_library,
        create_scheme,
        rename_scheme,
        delete_scheme,
        rename_imported_modpack,
        delete_imported_modpack
    ]);

    builder
        .manage(ImportController::default())
        .run(tauri::generate_context!())
        .expect("failed to run Minecraft Pack Builder desktop app");
}

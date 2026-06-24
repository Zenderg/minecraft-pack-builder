use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use credentials::{
    curseforge_key_status, read_curseforge_key, save_curseforge_key, CurseForgeCredentialStatus,
};
use mpb_agent::{start_streamable_http_server, AgentServer, AgentStatus, McpHttpServerHandle};
use mpb_assets::{
    build_modpack_asset_index_with_events, discover_modpack_releases, download_release_archive,
    parse_modpack_page_url, search_modpack_projects, AssetImportReport, CancellationToken,
    CurseForgeGateway, CurseForgeHttpGateway, CurseForgeProject, DiscoveredReleases,
    DownloadProgress, ModpackAssetImportRequest,
};
use mpb_core::DomainDemoReport;
use mpb_export::{write_scheme_export, ExportArtifact, ExportFormat};
use mpb_storage::{
    ensure_app_data_dirs, AppDataPaths, LibraryModpack, LibraryRepository, NewScheme,
};
use mpb_storage::{ImportStatus, NewImportedModpack};
use serde::Serialize;
use tauri::{Emitter, Manager};

mod credentials;
mod render_demo;

pub use render_demo::{
    demo_export_scheme, demo_render_scene, RenderBlockDto, RenderChunkSummaryDto, RenderSceneDto,
    RenderStageDto,
};

#[derive(Default)]
struct ImportController {
    current: Mutex<Option<CancellationToken>>,
}

struct AgentController {
    server: AgentServer,
    _http: Mutex<Option<McpHttpServerHandle>>,
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
fn get_scheme_render_scene(scheme_id: i64) -> RenderSceneDto {
    demo_render_scene(scheme_id)
}

pub fn write_demo_scheme_export(
    scheme_id: i64,
    format: ExportFormat,
    destination_path: impl AsRef<Path>,
) -> Result<ExportArtifact, String> {
    let scheme = demo_export_scheme(scheme_id);
    write_scheme_export(&scheme, format, destination_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn export_scheme(
    scheme_id: i64,
    format: String,
    destination_path: PathBuf,
) -> Result<ExportArtifact, String> {
    let format = ExportFormat::from_extension(&format)
        .ok_or_else(|| "Export format must be schem or litematic".to_string())?;
    write_demo_scheme_export(scheme_id, format, destination_path)
}

#[tauri::command]
fn get_ai_integration_status(controller: tauri::State<AgentController>) -> AgentStatus {
    controller.server.status()
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
    modpack_id: i64,
    stage: String,
    bytes_downloaded: u64,
    total_bytes: Option<u64>,
    progress_percent: Option<u8>,
}

impl ModpackImportProgress {
    fn from_download(modpack_id: i64, value: DownloadProgress) -> Self {
        let progress_percent = value.total_bytes.and_then(|total| {
            if total == 0 {
                return None;
            }
            let ratio = value.bytes_downloaded as f64 / total as f64;
            Some((10.0 + ratio.clamp(0.0, 1.0) * 20.0).round() as u8)
        });
        Self {
            modpack_id,
            stage: "download".to_string(),
            bytes_downloaded: value.bytes_downloaded,
            total_bytes: value.total_bytes,
            progress_percent,
        }
    }

    fn from_parse(modpack_id: i64, completed: u64, total: u64) -> Self {
        let progress_percent = if total == 0 {
            None
        } else {
            let ratio = completed as f64 / total as f64;
            Some((30.0 + ratio.clamp(0.0, 1.0) * 65.0).round() as u8)
        };
        Self {
            modpack_id,
            stage: "parse".to_string(),
            bytes_downloaded: 0,
            total_bytes: None,
            progress_percent,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportedModpackResult {
    library: Vec<LibraryModpack>,
    modpack_id: i64,
    archive_path: PathBuf,
    asset_report_path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModpackImportStatusChanged {
    modpack_id: i64,
    status: ImportStatus,
    message: Option<String>,
    stage: String,
    library: Vec<LibraryModpack>,
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
    let asset_report_slug = format!("{}-{}", parsed.slug, release.file_id);
    let import_asset_report_slug = asset_report_slug.clone();

    let imported = repository
        .create_imported_modpack(NewImportedModpack {
            local_name: format!("{} - {}", project.name, summary.version_name),
            source_slug: Some(project.slug),
            source_url: Some(parsed.normalized_url),
            version_name: summary.version_name.clone(),
            minecraft_version: summary.minecraft_versions.first().cloned(),
            loader: summary.loaders.first().cloned(),
            cache_dir: Some(cache_dir.clone()),
            import_status: ImportStatus::Importing,
        })
        .map_err(|error| error.to_string())?;
    repository
        .update_import_status(
            imported.id,
            ImportStatus::Importing,
            Some("Queued for background processing...".to_string()),
        )
        .map_err(|error| error.to_string())?;
    let library = repository
        .list_library()
        .map_err(|error| error.to_string())?;
    let token = controller.start()?;
    let import_app = app.clone();
    let import_api_key = api_key.clone();
    let import_release = release.clone();
    let import_archive_path = archive_path.clone();
    let import_cache_dir = cache_dir.clone();
    let import_diagnostics_dir = paths.diagnostics_dir.clone();
    let import_summary = summary.clone();
    let import_modpack_id = imported.id;
    std::thread::spawn(move || {
        let result = finish_modpack_import(
            import_app.clone(),
            import_api_key,
            import_release,
            import_archive_path,
            import_cache_dir,
            import_diagnostics_dir,
            import_asset_report_slug,
            import_summary,
            import_modpack_id,
            token,
        );
        if let Err(message) = result {
            let _ = set_import_status_and_emit(
                &import_app,
                import_modpack_id,
                ImportStatus::Failed,
                Some(message),
                "failed",
            );
        }
        if let Some(controller) = import_app.try_state::<ImportController>() {
            let _ = controller.clear();
        }
    });

    Ok(ImportedModpackResult {
        library,
        modpack_id: imported.id,
        archive_path,
        asset_report_path: paths
            .diagnostics_dir
            .join(format!("{}-assets.json", asset_report_slug)),
    })
}

#[tauri::command]
fn retry_modpack_import(
    app: tauri::AppHandle,
    controller: tauri::State<ImportController>,
    modpack_id: i64,
) -> Result<Vec<LibraryModpack>, String> {
    let api_key = read_curseforge_key().map_err(|error| error.to_string())?;
    let (_, repository) = library_repository(&app)?;
    let modpack = repository
        .get_imported_modpack(modpack_id)
        .map_err(|error| error.to_string())?;
    let page_url = modpack
        .source_url
        .clone()
        .ok_or_else(|| "Imported modpack has no CurseForge source URL".to_string())?;
    let cache_dir = modpack
        .cache_dir
        .clone()
        .ok_or_else(|| "Imported modpack has no cache directory".to_string())?;
    let file_id = curseforge_file_id_from_cache_dir(&cache_dir)?;
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
    let summary = release_summary_from_release(&release)?;
    let paths = discover_app_paths(app.clone())?;
    let archive_path = cache_dir
        .join("archives")
        .join(safe_path_segment(&release.file_name));
    let asset_report_slug = format!("{}-{}", parsed.slug, release.file_id);

    repository
        .update_import_status(
            modpack_id,
            ImportStatus::Importing,
            Some("Queued for retry...".to_string()),
        )
        .map_err(|error| error.to_string())?;
    let library = repository
        .list_library()
        .map_err(|error| error.to_string())?;

    let token = controller.start()?;
    let import_app = app.clone();
    std::thread::spawn(move || {
        let result = finish_modpack_import(
            import_app.clone(),
            api_key,
            release,
            archive_path,
            cache_dir,
            paths.diagnostics_dir,
            asset_report_slug,
            summary,
            modpack_id,
            token,
        );
        if let Err(message) = result {
            let _ = set_import_status_and_emit(
                &import_app,
                modpack_id,
                ImportStatus::Failed,
                Some(message),
                "failed",
            );
        }
        if let Some(controller) = import_app.try_state::<ImportController>() {
            let _ = controller.clear();
        }
    });

    Ok(library)
}

#[allow(clippy::too_many_arguments)]
fn finish_modpack_import(
    app: tauri::AppHandle,
    api_key: String,
    release: mpb_assets::CurseForgeRelease,
    archive_path: PathBuf,
    cache_dir: PathBuf,
    diagnostics_dir: PathBuf,
    asset_report_slug: String,
    summary: mpb_assets::ReleaseSummary,
    modpack_id: i64,
    token: CancellationToken,
) -> Result<(), String> {
    let gateway = CurseForgeHttpGateway::new().map_err(|error| error.to_string())?;
    let emit_app = app.clone();
    set_import_status_and_emit(
        &app,
        modpack_id,
        ImportStatus::Importing,
        Some("Downloading selected release...".to_string()),
        "download",
    )?;
    download_release_archive(
        &gateway,
        &api_key,
        &release,
        &archive_path,
        &token,
        |progress| {
            let _ = emit_app.emit(
                "modpack_import_progress",
                ModpackImportProgress::from_download(modpack_id, progress),
            );
        },
    )
    .map_err(|error| error.to_string())?;

    set_import_status_and_emit(
        &app,
        modpack_id,
        ImportStatus::Importing,
        Some("Parsing modpack assets...".to_string()),
        "parse",
    )?;
    let parse_event_app = app.clone();
    build_modpack_asset_index_with_events(
        &gateway,
        &api_key,
        ModpackAssetImportRequest {
            archive_path,
            cache_dir,
            diagnostics_dir,
            source_slug: asset_report_slug,
            release_name: summary.version_name.clone(),
            minecraft_version: summary.minecraft_versions.first().cloned(),
            loader: summary.loaders.first().cloned(),
        },
        &token,
        |event| {
            if let Some(progress) = event.progress {
                let _ = parse_event_app.emit(
                    "modpack_import_progress",
                    ModpackImportProgress::from_parse(
                        modpack_id,
                        progress.completed,
                        progress.total,
                    ),
                );
            }
            let _ = set_import_status_and_emit(
                &parse_event_app,
                modpack_id,
                ImportStatus::Importing,
                Some(event.message),
                "parse",
            );
        },
    )
    .map_err(|error| format!("Could not parse modpack assets: {error}"))?;

    set_import_status_and_emit(
        &app,
        modpack_id,
        ImportStatus::Imported,
        Some("Ready".to_string()),
        "done",
    )?;
    Ok(())
}

fn set_import_status_and_emit(
    app: &tauri::AppHandle,
    modpack_id: i64,
    status: ImportStatus,
    message: Option<String>,
    stage: &str,
) -> Result<(), String> {
    let (_, repository) = library_repository(app)?;
    repository
        .update_import_status(modpack_id, status, message.clone())
        .map_err(|error| error.to_string())?;
    let library = repository
        .list_library()
        .map_err(|error| error.to_string())?;
    app.emit(
        "modpack_import_status_changed",
        ModpackImportStatusChanged {
            modpack_id,
            status,
            message,
            stage: stage.to_string(),
            library,
        },
    )
    .map_err(|error| error.to_string())
}

fn release_summary_from_release(
    release: &mpb_assets::CurseForgeRelease,
) -> Result<mpb_assets::ReleaseSummary, String> {
    mpb_assets::filter_releases(
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
    .ok_or_else(|| "could not summarize selected release".to_string())
}

fn curseforge_file_id_from_cache_dir(cache_dir: &Path) -> Result<u64, String> {
    let name = cache_dir
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Imported modpack cache directory is not readable".to_string())?;
    name.rsplit_once('-')
        .and_then(|(_, file_id)| file_id.parse::<u64>().ok())
        .ok_or_else(|| "Could not determine CurseForge file id for retry".to_string())
}

#[tauri::command]
fn load_modpack_asset_report(
    app: tauri::AppHandle,
    modpack_id: i64,
) -> Result<AssetImportReport, String> {
    let (paths, repository) = library_repository(&app)?;
    let modpack = repository
        .get_imported_modpack(modpack_id)
        .map_err(|error| error.to_string())?;
    let cache_dir = modpack
        .cache_dir
        .ok_or_else(|| "Imported modpack has no asset cache directory".to_string())?;
    let report_stem = cache_dir
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Imported modpack cache directory is not readable".to_string())?;
    let report_path = paths
        .diagnostics_dir
        .join(format!("{}-assets.json", report_stem));
    let json = std::fs::read_to_string(&report_path)
        .map_err(|error| format!("Could not read modpack asset diagnostics report: {error}"))?;
    serde_json::from_str(&json)
        .map_err(|error| format!("Could not parse modpack asset diagnostics report: {error}"))
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
        retry_modpack_import,
        cancel_curseforge_import,
        load_modpack_asset_report,
        generate_domain_demo_report,
        get_scheme_render_scene,
        export_scheme,
        get_ai_integration_status,
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
        retry_modpack_import,
        cancel_curseforge_import,
        load_modpack_asset_report,
        generate_domain_demo_report,
        get_scheme_render_scene,
        export_scheme,
        get_ai_integration_status,
        list_library,
        create_scheme,
        rename_scheme,
        delete_scheme,
        rename_imported_modpack,
        delete_imported_modpack
    ]);

    builder
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let server = AgentServer::new_demo();
            let app_handle = app.handle().clone();
            let http = start_streamable_http_server(server.clone(), move |events| {
                for event in events {
                    let _ = app_handle.emit("ai_agent_event", event);
                }
            })
            .map_err(|error| Box::<dyn std::error::Error>::from(error.to_string()))?;
            app.manage(AgentController {
                server,
                _http: Mutex::new(Some(http)),
            });
            Ok(())
        })
        .manage(ImportController::default())
        .run(tauri::generate_context!())
        .expect("failed to run Minecraft Pack Builder desktop app");
}

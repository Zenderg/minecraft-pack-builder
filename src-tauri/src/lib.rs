use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::Duration;

use mpb_agent::{start_streamable_http_server, AgentServer, AgentStatus, McpHttpServerHandle};
use mpb_assets::{
    build_prism_asset_index, prism_registry_metadata_path, prism_registry_report_path,
    validate_prism_root, PrismAssetIndexMetadata, PrismAssetIndexRequest, PrismInstanceDescriptor,
    PrismRootValidation, PRISM_REGISTRY_SCHEMA_VERSION,
};
use mpb_export::{write_scheme_export, ExportArtifact, ExportFormat};
use mpb_storage::{
    ensure_app_data_dirs, AppDataPaths, LibraryInstance, LibraryRepository, NewPrismInstance,
    NewScheme, PrismInstanceStatus,
};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue};
use tauri::{Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

mod render_scene;

pub use render_scene::{
    render_scene_from_scheme, render_scene_from_scheme_with_registry_report, RenderBlockDto,
    RenderChunkSummaryDto, RenderMaterialDto, RenderSceneDto, RenderStageDto,
};

struct AgentController {
    server: AgentServer,
    _http: Mutex<Option<McpHttpServerHandle>>,
}

struct PrismWatcherController {
    watcher: Mutex<Option<RecommendedWatcher>>,
    sync: Mutex<PrismSyncState>,
}

#[derive(Debug, Default)]
struct PrismSyncState {
    running: bool,
    pending: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDiagnosticReport {
    pub operation: String,
    pub status: String,
    pub scheme_id: i64,
    pub format: ExportFormat,
    pub destination_path: PathBuf,
    pub artifact_path: Option<PathBuf>,
    pub byte_len: Option<u64>,
    pub block_count: Option<usize>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub recovery_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDiagnosticArtifact {
    pub path: PathBuf,
    pub report: ExportDiagnosticReport,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportWithDiagnostics {
    pub artifact: ExportArtifact,
    pub diagnostic: ExportDiagnosticArtifact,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub status: String,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub notes: Option<String>,
    pub date: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExportDiagnosticError {
    pub message: String,
    pub diagnostic_path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismRootSelection {
    pub validation: PrismRootValidation,
    pub library: Vec<LibraryInstance>,
    pub relink_candidates: Vec<PrismRelinkCandidate>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismRelinkCandidate {
    pub existing_id: i64,
    pub existing_display_name: String,
    pub existing_instance_path: PathBuf,
    pub discovered_identity_fingerprint: String,
    pub discovered_display_name: String,
    pub discovered_instance_path: PathBuf,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LibraryChangedEvent {
    library: Vec<LibraryInstance>,
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

#[tauri::command]
fn validate_prism_launcher_root(root_path: PathBuf) -> Result<PrismRootValidation, String> {
    validate_prism_root(root_path).map_err(|error| error.to_string())
}

#[tauri::command]
fn discover_prism_launcher_roots() -> Result<Vec<PrismRootValidation>, String> {
    default_prism_root_candidates()
        .into_iter()
        .map(validate_prism_root)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn select_prism_launcher_root(
    app: tauri::AppHandle,
    root_path: PathBuf,
) -> Result<PrismRootSelection, String> {
    let validation = validate_prism_root(&root_path).map_err(|error| error.to_string())?;
    if !validation.valid {
        return Ok(PrismRootSelection {
            validation,
            library: list_library(app)?,
            relink_candidates: Vec::new(),
        });
    }

    let (_, repository) = library_repository(&app)?;
    repository
        .set_prism_root(Some(root_path))
        .map_err(|error| error.to_string())?;
    let relink_candidates = prism_relink_candidates(&repository, &validation.instances)?;
    let skipped = relink_candidates
        .iter()
        .map(|candidate| candidate.discovered_identity_fingerprint.clone())
        .collect::<Vec<_>>();
    record_prism_instances_for_background_sync(&repository, &validation.instances, &skipped)?;
    replace_prism_watcher(&app, &validation.root_path)?;
    trigger_prism_background_sync(&app)?;
    let library = repository
        .list_library()
        .map_err(|error| error.to_string())?;
    app.emit(
        "library_changed",
        LibraryChangedEvent {
            library: library.clone(),
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(PrismRootSelection {
        validation,
        library,
        relink_candidates,
    })
}

#[tauri::command]
fn sync_prism_library(app: tauri::AppHandle) -> Result<Vec<LibraryInstance>, String> {
    trigger_prism_background_sync(&app)?;
    list_library(app)
}

fn sync_prism_library_for_app(app: &tauri::AppHandle) -> Result<Vec<LibraryInstance>, String> {
    let (paths, repository) = library_repository(app)?;
    if let Some(root) = repository
        .get_prism_root()
        .map_err(|error| error.to_string())?
    {
        let validation = validate_prism_root(root).map_err(|error| error.to_string())?;
        if validation.valid {
            let relink_candidates = prism_relink_candidates(&repository, &validation.instances)?;
            let skipped = relink_candidates
                .iter()
                .map(|candidate| candidate.discovered_identity_fingerprint.clone())
                .collect::<Vec<_>>();
            sync_prism_instances(&repository, &paths, &validation.instances, &skipped)?;
        }
    }
    repository.list_library().map_err(|error| error.to_string())
}

#[tauri::command]
fn list_prism_relink_candidates(
    app: tauri::AppHandle,
) -> Result<Vec<PrismRelinkCandidate>, String> {
    let (_, repository) = library_repository(&app)?;
    let Some(root) = repository
        .get_prism_root()
        .map_err(|error| error.to_string())?
    else {
        return Ok(Vec::new());
    };
    let validation = validate_prism_root(root).map_err(|error| error.to_string())?;
    if !validation.valid {
        return Ok(Vec::new());
    }
    prism_relink_candidates(&repository, &validation.instances)
}

#[tauri::command]
fn confirm_prism_instance_relink(
    app: tauri::AppHandle,
    existing_id: i64,
    discovered_identity_fingerprint: String,
) -> Result<Vec<LibraryInstance>, String> {
    let (paths, repository) = library_repository(&app)?;
    let root = repository
        .get_prism_root()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No PrismLauncher Launcher Root is selected.".to_string())?;
    let validation = validate_prism_root(root).map_err(|error| error.to_string())?;
    let instance = validation
        .instances
        .iter()
        .find(|instance| instance.identity_fingerprint == discovered_identity_fingerprint)
        .ok_or_else(|| {
            "The selected PrismLauncher instance is no longer present in the active Launcher Root."
                .to_string()
        })?;
    let stored = repository
        .relink_prism_instance(
            existing_id,
            new_prism_instance(
                instance,
                PrismInstanceStatus::Indexing,
                Some("Indexing relinked PrismLauncher assets.".to_string()),
            ),
        )
        .map_err(|error| error.to_string())?;
    match build_prism_asset_index(PrismAssetIndexRequest {
        instance_id: instance.instance_id.clone(),
        identity_fingerprint: instance.identity_fingerprint.clone(),
        content_fingerprint: instance.content_fingerprint.clone(),
        instance_path: instance.instance_path.clone(),
        minecraft_dir: instance.minecraft_dir.clone(),
        diagnostics_dir: paths.diagnostics_dir.clone(),
        minecraft_version: instance.minecraft_version.clone(),
        loader: instance.loader.clone(),
    }) {
        Ok(report) => {
            let message = format!(
                "Ready. Indexed {} blocks from {} local archives.",
                report.block_count, report.archive_count
            );
            repository
                .update_prism_instance_status(stored.id, PrismInstanceStatus::Ready, Some(&message))
                .map_err(|error| error.to_string())?;
        }
        Err(error) => {
            let message = format!(
                "Could not build Prism block registry: {error}. Schemes stay available for export, but viewer and editing are blocked until the instance is ready."
            );
            repository
                .update_prism_instance_status(
                    stored.id,
                    PrismInstanceStatus::Failed,
                    Some(&message),
                )
                .map_err(|error| error.to_string())?;
        }
    }
    let library = sync_prism_library_for_app(&app)?;
    app.emit(
        "library_changed",
        LibraryChangedEvent {
            library: library.clone(),
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(library)
}

#[tauri::command]
fn list_library(app: tauri::AppHandle) -> Result<Vec<LibraryInstance>, String> {
    let (_, repository) = library_repository(&app)?;
    repository.list_library().map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_scheme_render_scene(
    app: tauri::AppHandle,
    scheme_id: i64,
) -> Result<RenderSceneDto, String> {
    tauri::async_runtime::spawn_blocking(move || get_scheme_render_scene_blocking(app, scheme_id))
        .await
        .map_err(|error| format!("Could not join render scene worker: {error}"))?
}

fn get_scheme_render_scene_blocking(
    app: tauri::AppHandle,
    scheme_id: i64,
) -> Result<RenderSceneDto, String> {
    let (paths, repository) = library_repository(&app)?;
    let stored = repository
        .load_scheme(scheme_id)
        .map_err(|error| error.to_string())?;
    let block_ids = stored
        .scheme
        .blocks()
        .map(|(_, block)| block.block_id.clone())
        .collect::<BTreeSet<_>>();
    let registry_report = repository
        .get_prism_instance(stored.record.prism_instance_id)
        .ok()
        .and_then(|instance| {
            read_registry_report_for_block_ids(
                &paths.diagnostics_dir,
                &instance.identity_fingerprint,
                &block_ids,
            )
        });
    Ok(render_scene_from_scheme_with_registry_report(
        scheme_id,
        &stored.scheme,
        registry_report.as_ref(),
    ))
}

pub fn write_stored_scheme_export(
    database_path: impl AsRef<Path>,
    scheme_id: i64,
    format: ExportFormat,
    destination_path: impl AsRef<Path>,
) -> Result<ExportArtifact, String> {
    let database =
        mpb_storage::LibraryDatabase::open(database_path).map_err(|error| error.to_string())?;
    let repository = LibraryRepository::new(database);
    let stored = repository
        .load_scheme(scheme_id)
        .map_err(|error| error.to_string())?;
    write_scheme_export(&stored.scheme, format, destination_path).map_err(|error| error.to_string())
}

pub fn write_stored_scheme_export_with_diagnostics(
    database_path: impl AsRef<Path>,
    scheme_id: i64,
    format: ExportFormat,
    destination_path: impl AsRef<Path>,
    diagnostics_dir: impl AsRef<Path>,
) -> Result<ExportWithDiagnostics, ExportDiagnosticError> {
    let destination_path = destination_path.as_ref().to_path_buf();
    let diagnostics_dir = diagnostics_dir.as_ref().to_path_buf();
    let diagnostic_path = diagnostics_dir.join(export_diagnostic_file_name(scheme_id, format));
    let result = (|| {
        let database =
            mpb_storage::LibraryDatabase::open(database_path).map_err(|error| error.to_string())?;
        let repository = LibraryRepository::new(database);
        let stored = repository
            .load_scheme(scheme_id)
            .map_err(|error| error.to_string())?;
        let scheme = stored.scheme;
        let artifact = write_scheme_export(&scheme, format, &destination_path)
            .map_err(|error| error.to_string())?;
        Ok::<_, String>((artifact, scheme.block_count()))
    })();

    match result {
        Ok((artifact, block_count)) => {
            let report = ExportDiagnosticReport {
                operation: "export".to_string(),
                status: "success".to_string(),
                scheme_id,
                format,
                destination_path,
                artifact_path: Some(artifact.path.clone()),
                byte_len: Some(artifact.byte_len),
                block_count: Some(block_count),
                error_code: None,
                error_message: None,
                recovery_message: None,
            };
            let diagnostic = write_export_diagnostic(&diagnostics_dir, &diagnostic_path, report)
                .map_err(|message| ExportDiagnosticError {
                    message,
                    diagnostic_path: diagnostic_path.clone(),
                })?;
            Ok(ExportWithDiagnostics {
                artifact,
                diagnostic,
            })
        }
        Err(error_message) => {
            let recovery_message = export_recovery_message(&error_message).to_string();
            let report = ExportDiagnosticReport {
                operation: "export".to_string(),
                status: "failed".to_string(),
                scheme_id,
                format,
                destination_path,
                artifact_path: None,
                byte_len: None,
                block_count: None,
                error_code: Some("export_failed".to_string()),
                error_message: Some(error_message.clone()),
                recovery_message: Some(recovery_message),
            };
            let diagnostic_path =
                write_export_diagnostic(&diagnostics_dir, &diagnostic_path, report)
                    .map(|diagnostic| diagnostic.path)
                    .unwrap_or(diagnostic_path);
            Err(ExportDiagnosticError {
                message: format!(
                    "Could not export scheme. {} Diagnostic report: {}",
                    error_message,
                    diagnostic_path.display()
                ),
                diagnostic_path,
            })
        }
    }
}

fn write_export_diagnostic(
    diagnostics_dir: &Path,
    diagnostic_path: &Path,
    report: ExportDiagnosticReport,
) -> Result<ExportDiagnosticArtifact, String> {
    std::fs::create_dir_all(diagnostics_dir)
        .map_err(|error| format!("Could not create diagnostics directory: {error}"))?;
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("Could not serialize export diagnostic report: {error}"))?;
    std::fs::write(diagnostic_path, json)
        .map_err(|error| format!("Could not write export diagnostic report: {error}"))?;
    Ok(ExportDiagnosticArtifact {
        path: diagnostic_path.to_path_buf(),
        report,
    })
}

fn export_diagnostic_file_name(scheme_id: i64, format: ExportFormat) -> String {
    format!("export-scheme-{scheme_id}-{}.json", format.extension())
}

fn export_recovery_message(error_message: &str) -> &'static str {
    if error_message.contains("No such file")
        || error_message.contains("cannot find the path")
        || error_message.contains("parent")
    {
        "Choose another destination folder and try export again."
    } else if error_message.contains("too large") || error_message.contains("exceeds") {
        "Resize the scheme into the supported export limits before trying again."
    } else {
        "Keep the scheme open, check the diagnostic report, and try export again."
    }
}

#[tauri::command]
fn export_scheme(
    app: tauri::AppHandle,
    scheme_id: i64,
    format: String,
    destination_path: PathBuf,
) -> Result<ExportArtifact, String> {
    let format = ExportFormat::from_extension(&format)
        .ok_or_else(|| format!("Unsupported export format: {format}"))?;
    let paths = discover_app_paths(app.clone())?;
    let database_path = paths.app_data_dir.join("library.sqlite3");
    write_stored_scheme_export_with_diagnostics(
        database_path,
        scheme_id,
        format,
        destination_path,
        paths.diagnostics_dir,
    )
    .map(|result| result.artifact)
    .map_err(|error| error.message)
}

#[tauri::command]
async fn check_for_updates(app: tauri::AppHandle) -> UpdateCheckResult {
    let current_version = app.package_info().version.to_string();
    match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => UpdateCheckResult {
                status: "available".to_string(),
                current_version,
                latest_version: Some(update.version),
                notes: update.body,
                date: update.date.map(|date| date.to_string()),
                error_message: None,
            },
            Ok(None) => UpdateCheckResult {
                status: "current".to_string(),
                current_version,
                latest_version: None,
                notes: None,
                date: None,
                error_message: None,
            },
            Err(error) => UpdateCheckResult {
                status: "failed".to_string(),
                current_version,
                latest_version: None,
                notes: None,
                date: None,
                error_message: Some(error.to_string()),
            },
        },
        Err(error) => UpdateCheckResult {
            status: "failed".to_string(),
            current_version,
            latest_version: None,
            notes: None,
            date: None,
            error_message: Some(error.to_string()),
        },
    }
}

#[tauri::command]
fn get_ai_integration_status(
    controller: tauri::State<AgentController>,
) -> Result<AgentStatus, String> {
    Ok(controller.server.status())
}

#[tauri::command]
fn create_scheme(
    app: tauri::AppHandle,
    prism_instance_id: i64,
    name: String,
    size_x: i64,
    size_y: i64,
    size_z: i64,
) -> Result<Vec<LibraryInstance>, String> {
    let (_, repository) = library_repository(&app)?;
    repository
        .create_scheme(NewScheme {
            prism_instance_id,
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
) -> Result<Vec<LibraryInstance>, String> {
    let (_, repository) = library_repository(&app)?;
    repository
        .rename_scheme(scheme_id, &name)
        .map_err(|error| error.to_string())?;
    repository.list_library().map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_scheme(app: tauri::AppHandle, scheme_id: i64) -> Result<Vec<LibraryInstance>, String> {
    let (_, repository) = library_repository(&app)?;
    repository
        .delete_scheme(scheme_id)
        .map_err(|error| error.to_string())?;
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

fn sync_prism_instances(
    repository: &LibraryRepository,
    paths: &AppDataPaths,
    instances: &[PrismInstanceDescriptor],
    skipped_identity_fingerprints: &[String],
) -> Result<(), String> {
    let mut active = Vec::with_capacity(instances.len());
    for instance in instances {
        active.push(instance.identity_fingerprint.clone());
        if skipped_identity_fingerprints
            .iter()
            .any(|fingerprint| fingerprint == &instance.identity_fingerprint)
        {
            continue;
        }
        let existing = repository
            .get_prism_instance_by_identity_fingerprint(&instance.identity_fingerprint)
            .map_err(|error| error.to_string())?;
        if let Some(existing) = &existing {
            let unchanged = existing.content_fingerprint == instance.content_fingerprint;
            let registry_current = registry_report_is_current(&paths.diagnostics_dir, instance);
            if unchanged && existing.status == PrismInstanceStatus::Ready && registry_current {
                repository
                    .upsert_prism_instance(new_prism_instance(
                        instance,
                        PrismInstanceStatus::Ready,
                        existing.status_message.clone(),
                    ))
                    .map_err(|error| error.to_string())?;
                continue;
            }
            if unchanged && existing.status == PrismInstanceStatus::Failed {
                repository
                    .upsert_prism_instance(new_prism_instance(
                        instance,
                        PrismInstanceStatus::Failed,
                        existing.status_message.clone(),
                    ))
                    .map_err(|error| error.to_string())?;
                continue;
            }
        }

        let stored = repository
            .upsert_prism_instance(new_prism_instance(
                instance,
                PrismInstanceStatus::Indexing,
                Some("Indexing local PrismLauncher assets.".to_string()),
            ))
            .map_err(|error| error.to_string())?;

        match build_prism_asset_index(PrismAssetIndexRequest {
            instance_id: instance.instance_id.clone(),
            identity_fingerprint: instance.identity_fingerprint.clone(),
            content_fingerprint: instance.content_fingerprint.clone(),
            instance_path: instance.instance_path.clone(),
            minecraft_dir: instance.minecraft_dir.clone(),
            diagnostics_dir: paths.diagnostics_dir.clone(),
            minecraft_version: instance.minecraft_version.clone(),
            loader: instance.loader.clone(),
        }) {
            Ok(report) => {
                let message = format!(
                    "Ready. Indexed {} blocks from {} local archives.",
                    report.block_count, report.archive_count
                );
                repository
                    .update_prism_instance_status(
                        stored.id,
                        PrismInstanceStatus::Ready,
                        Some(&message),
                    )
                    .map_err(|error| error.to_string())?;
            }
            Err(error) => {
                let message = format!(
                    "Could not build Prism block registry: {error}. Schemes stay available for export, but viewer and editing are blocked until the instance is ready."
                );
                repository
                    .update_prism_instance_status(
                        stored.id,
                        PrismInstanceStatus::Failed,
                        Some(&message),
                    )
                    .map_err(|error| error.to_string())?;
            }
        }
    }
    repository
        .mark_prism_instances_missing_except(&active)
        .map_err(|error| error.to_string())
}

pub fn record_prism_instances_for_background_sync(
    repository: &LibraryRepository,
    instances: &[PrismInstanceDescriptor],
    skipped_identity_fingerprints: &[String],
) -> Result<(), String> {
    let mut active = Vec::with_capacity(instances.len());
    for instance in instances {
        active.push(instance.identity_fingerprint.clone());
        if skipped_identity_fingerprints
            .iter()
            .any(|fingerprint| fingerprint == &instance.identity_fingerprint)
        {
            continue;
        }

        let existing = repository
            .get_prism_instance_by_identity_fingerprint(&instance.identity_fingerprint)
            .map_err(|error| error.to_string())?;
        let (status, message) = match existing {
            Some(existing) if existing.content_fingerprint == instance.content_fingerprint => {
                (existing.status, existing.status_message)
            }
            _ => (
                PrismInstanceStatus::Indexing,
                Some("Waiting for background PrismLauncher indexing.".to_string()),
            ),
        };
        repository
            .upsert_prism_instance(new_prism_instance(instance, status, message))
            .map_err(|error| error.to_string())?;
    }
    repository
        .mark_prism_instances_missing_except(&active)
        .map_err(|error| error.to_string())
}

fn trigger_prism_background_sync(app: &tauri::AppHandle) -> Result<(), String> {
    let controller = app.state::<PrismWatcherController>();
    {
        let mut sync = controller
            .sync
            .lock()
            .map_err(|_| "Could not lock PrismLauncher sync state.".to_string())?;
        if sync.running {
            sync.pending = true;
            return Ok(());
        }
        sync.running = true;
        sync.pending = false;
    }

    let app_handle = app.clone();
    thread::spawn(move || loop {
        let result = sync_prism_library_for_app(&app_handle);
        match result {
            Ok(library) => {
                let _ = app_handle.emit("library_changed", LibraryChangedEvent { library });
            }
            Err(error) => {
                eprintln!("PrismLauncher background sync failed: {error}");
            }
        }

        let controller = app_handle.state::<PrismWatcherController>();
        if let Ok(mut sync) = controller.sync.lock() {
            if sync.pending {
                sync.pending = false;
                continue;
            }
            sync.running = false;
        }
        break;
    });
    Ok(())
}

fn new_prism_instance(
    instance: &PrismInstanceDescriptor,
    status: PrismInstanceStatus,
    status_message: Option<String>,
) -> NewPrismInstance {
    NewPrismInstance {
        instance_id: instance.instance_id.clone(),
        display_name: instance.display_name.clone(),
        instance_path: instance.instance_path.clone(),
        minecraft_dir: instance.minecraft_dir.clone(),
        minecraft_version: instance.minecraft_version.clone(),
        loader: instance.loader.clone(),
        loader_version: instance.loader_version.clone(),
        identity_fingerprint: instance.identity_fingerprint.clone(),
        content_fingerprint: instance.content_fingerprint.clone(),
        status,
        status_message,
    }
}

fn registry_report_path(diagnostics_dir: &Path, identity_fingerprint: &str) -> PathBuf {
    prism_registry_report_path(diagnostics_dir, identity_fingerprint)
}

fn registry_metadata_path(diagnostics_dir: &Path, identity_fingerprint: &str) -> PathBuf {
    prism_registry_metadata_path(diagnostics_dir, identity_fingerprint)
}

fn prism_root_from_instance_path(instance_path: &Path) -> Option<PathBuf> {
    let instances_dir = instance_path.parent()?;
    if instances_dir.file_name()?.to_string_lossy() != "instances" {
        return None;
    }
    instances_dir.parent().map(Path::to_path_buf)
}

fn registry_report_is_current(diagnostics_dir: &Path, instance: &PrismInstanceDescriptor) -> bool {
    let path = registry_metadata_path(diagnostics_dir, &instance.identity_fingerprint);
    let Some(metadata) = std::fs::read_to_string(&path)
        .ok()
        .and_then(|json| serde_json::from_str::<PrismAssetIndexMetadata>(&json).ok())
    else {
        return legacy_registry_report_is_current(diagnostics_dir, instance);
    };
    if metadata.schema_version != PRISM_REGISTRY_SCHEMA_VERSION {
        return false;
    }
    if metadata.identity_fingerprint != instance.identity_fingerprint {
        return false;
    }
    if metadata.content_fingerprint != instance.content_fingerprint {
        return false;
    }
    if !metadata.report_path.is_file() {
        return false;
    }
    if metadata.runtime_status == "ready" {
        return true;
    }
    !runtime_prerequisites_present(instance)
}

fn legacy_registry_report_is_current(
    diagnostics_dir: &Path,
    instance: &PrismInstanceDescriptor,
) -> bool {
    let path = registry_report_path(diagnostics_dir, &instance.identity_fingerprint);
    if !path.is_file() {
        return false;
    }
    let Some(header) = read_registry_report_header(&path) else {
        return false;
    };
    if raw_json_u64_field(&header, "schemaVersion") != Some(PRISM_REGISTRY_SCHEMA_VERSION as u64) {
        return false;
    }
    if raw_json_string_field(&header, "runtimeStatus").as_deref() == Some("ready") {
        return true;
    }
    !runtime_prerequisites_present(instance)
}

fn read_registry_report_header(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut buffer = vec![0; 16 * 1024];
    let len = file.read(&mut buffer).ok()?;
    buffer.truncate(len);
    String::from_utf8(buffer).ok()
}

fn raw_json_u64_field(raw: &str, field: &str) -> Option<u64> {
    let key = format!("\"{field}\"");
    let after_key = raw.get(raw.find(&key)? + key.len()..)?;
    let after_colon = after_key.get(after_key.find(':')? + 1..)?.trim_start();
    let end = after_colon
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(after_colon.len());
    after_colon.get(..end)?.parse().ok()
}

fn raw_json_string_field(raw: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let after_key = raw.get(raw.find(&key)? + key.len()..)?;
    let after_colon = after_key.get(after_key.find(':')? + 1..)?;
    let value = after_colon.trim_start().strip_prefix('"')?;
    let end = value.find('"')?;
    let text = value.get(..end)?;
    (!text.contains('\\')).then(|| text.to_string())
}

pub fn runtime_prerequisites_present(instance: &PrismInstanceDescriptor) -> bool {
    let loader = instance.loader.as_deref().unwrap_or_default();
    let normalized_loader = loader.to_ascii_lowercase();
    let Some(minecraft_version) = instance.minecraft_version.as_deref() else {
        return false;
    };
    let Some(root) = prism_root_from_instance_path(&instance.instance_path) else {
        return false;
    };
    let libraries = root.join("libraries");

    if normalized_loader.contains("neoforge") {
        return neoforge_runtime_prerequisites_present(&libraries, minecraft_version);
    }
    if normalized_loader.contains("forge") {
        return forge_runtime_prerequisites_present(&root, &libraries, instance, minecraft_version);
    }
    if normalized_loader.contains("fabric") {
        return fabric_runtime_prerequisites_present(&libraries, minecraft_version);
    }
    false
}

fn neoforge_runtime_prerequisites_present(libraries: &Path, minecraft_version: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(libraries.join("net/neoforged/neoform")) else {
        return false;
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_str().map(ToString::to_string))
        .filter_map(|version| {
            version
                .strip_prefix(&format!("{minecraft_version}-"))
                .map(ToString::to_string)
        })
        .any(|neoform_version| {
            libraries
                .join(format!(
                    "net/minecraft/client/{minecraft_version}-{neoform_version}/client-{minecraft_version}-{neoform_version}-mappings.txt"
                ))
                .is_file()
        })
}

fn forge_runtime_prerequisites_present(
    root: &Path,
    libraries: &Path,
    instance: &PrismInstanceDescriptor,
    minecraft_version: &str,
) -> bool {
    let Some(loader_version) = instance.loader_version.as_deref() else {
        return false;
    };
    let Some(mcp_version) = forge_mcp_version(root, loader_version) else {
        return false;
    };
    libraries
        .join(format!(
            "net/minecraft/client/{minecraft_version}-{mcp_version}/client-{minecraft_version}-{mcp_version}-mappings.txt"
        ))
        .is_file()
}

fn forge_mcp_version(root: &Path, loader_version: &str) -> Option<String> {
    let meta_path = root
        .join("meta")
        .join("net.minecraftforge")
        .join(format!("{loader_version}.json"));
    let text = std::fs::read_to_string(meta_path).ok()?;
    let metadata = serde_json::from_str::<serde_json::Value>(&text).ok()?;
    let arguments = metadata.get("minecraftArguments")?.as_str()?;
    argument_value(arguments, "--fml.mcpVersion").map(ToString::to_string)
}

fn fabric_runtime_prerequisites_present(libraries: &Path, minecraft_version: &str) -> bool {
    [
        libraries.join(format!(
            "com/mojang/minecraft/{minecraft_version}/minecraft-{minecraft_version}-server.jar"
        )),
        libraries.join(format!(
            "net/minecraft/server/{minecraft_version}/server-{minecraft_version}.jar"
        )),
    ]
    .into_iter()
    .any(|path| path.is_file())
}

fn argument_value<'a>(arguments: &'a str, name: &str) -> Option<&'a str> {
    let mut parts = arguments.split_whitespace();
    while let Some(part) = parts.next() {
        if part == name {
            return parts.next();
        }
    }
    None
}

fn read_registry_report_for_block_ids(
    diagnostics_dir: &Path,
    identity_fingerprint: &str,
    block_ids: &BTreeSet<String>,
) -> Option<serde_json::Value> {
    let path = registry_report_path(diagnostics_dir, identity_fingerprint);
    let json_text = std::fs::read_to_string(path).ok()?;
    let report = serde_json::from_str::<RawRegistryReport>(&json_text).ok()?;
    let blocks = report
        .blocks
        .into_iter()
        .filter_map(|raw_block| {
            let id = raw_registry_block_identifier(raw_block.get())?;
            block_ids
                .contains(id)
                .then(|| serde_json::from_str::<serde_json::Value>(raw_block.get()).ok())
                .flatten()
        })
        .collect::<Vec<_>>();
    Some(json!({
        "runtimeStatus": report.runtime_status,
        "blocks": blocks
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRegistryReport<'a> {
    #[serde(default)]
    runtime_status: Option<&'a str>,
    #[serde(default, borrow)]
    blocks: Vec<&'a RawValue>,
}

fn raw_registry_block_identifier(raw_block: &str) -> Option<&str> {
    let key = "\"identifier\"";
    let after_key = raw_block.get(raw_block.find(key)? + key.len()..)?;
    let after_colon = after_key.get(after_key.find(':')? + 1..)?;
    let value = after_colon.trim_start().strip_prefix('"')?;
    let end = value.find('"')?;
    let identifier = value.get(..end)?;
    (!identifier.contains('\\')).then_some(identifier)
}

fn prism_relink_candidates(
    repository: &LibraryRepository,
    discovered: &[PrismInstanceDescriptor],
) -> Result<Vec<PrismRelinkCandidate>, String> {
    let stored = repository
        .list_prism_instances()
        .map_err(|error| error.to_string())?;
    let discovered_identities = discovered
        .iter()
        .map(|instance| instance.identity_fingerprint.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut candidates = Vec::new();

    for instance in discovered {
        if stored
            .iter()
            .any(|record| record.identity_fingerprint == instance.identity_fingerprint)
        {
            continue;
        }
        let Some(existing) = stored.iter().find(|record| {
            !discovered_identities.contains(record.identity_fingerprint.as_str())
                && possible_relink_match(record, instance)
        }) else {
            continue;
        };
        candidates.push(PrismRelinkCandidate {
            existing_id: existing.id,
            existing_display_name: existing.display_name.clone(),
            existing_instance_path: existing.instance_path.clone(),
            discovered_identity_fingerprint: instance.identity_fingerprint.clone(),
            discovered_display_name: instance.display_name.clone(),
            discovered_instance_path: instance.instance_path.clone(),
            minecraft_version: instance.minecraft_version.clone(),
            loader: instance.loader.clone(),
        });
    }

    Ok(candidates)
}

fn possible_relink_match(
    existing: &mpb_storage::PrismInstanceRecord,
    discovered: &PrismInstanceDescriptor,
) -> bool {
    normalized_match_key(&existing.display_name) == normalized_match_key(&discovered.display_name)
        && existing.minecraft_version == discovered.minecraft_version
        && existing.loader == discovered.loader
}

fn normalized_match_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn replace_prism_watcher(app: &tauri::AppHandle, root: &Path) -> Result<(), String> {
    let instances_dir = root.join("instances");
    if !instances_dir.is_dir() {
        return Ok(());
    }

    let app_handle = app.clone();
    let (event_tx, event_rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |event| {
            let _ = event_tx.send(event);
        },
        Config::default(),
    )
    .map_err(|error| format!("Could not start PrismLauncher watcher: {error}"))?;
    watcher
        .watch(&instances_dir, RecursiveMode::Recursive)
        .map_err(|error| {
            format!(
                "Could not watch PrismLauncher instances at {}: {error}",
                instances_dir.display()
            )
        })?;

    thread::spawn(move || {
        while event_rx.recv().is_ok() {
            thread::sleep(Duration::from_millis(700));
            while event_rx.try_recv().is_ok() {}
            if let Err(error) = trigger_prism_background_sync(&app_handle) {
                eprintln!("PrismLauncher watcher sync failed: {error}");
            }
        }
    });

    let controller = app.state::<PrismWatcherController>();
    let mut guard = controller
        .watcher
        .lock()
        .map_err(|_| "Could not lock PrismLauncher watcher state".to_string())?;
    *guard = Some(watcher);
    Ok(())
}

fn default_prism_root_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if let Some(home) = home {
        if cfg!(target_os = "macos") {
            candidates.push(home.join("Library/Application Support/PrismLauncher"));
        } else if cfg!(target_os = "windows") {
            if let Some(app_data) = std::env::var_os("APPDATA") {
                candidates.push(PathBuf::from(app_data).join("PrismLauncher"));
            }
            candidates.push(home.join("scoop/persist/prismlauncher"));
        } else {
            candidates.push(home.join(".local/share/PrismLauncher"));
            candidates
                .push(home.join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher"));
        }
    }
    candidates
}

pub fn run() {
    let builder = tauri::Builder::default();

    let builder = builder.invoke_handler(tauri::generate_handler![
        discover_app_paths,
        open_app_data_folder,
        discover_prism_launcher_roots,
        validate_prism_launcher_root,
        select_prism_launcher_root,
        sync_prism_library,
        list_prism_relink_candidates,
        confirm_prism_instance_relink,
        get_scheme_render_scene,
        export_scheme,
        check_for_updates,
        get_ai_integration_status,
        list_library,
        create_scheme,
        rename_scheme,
        delete_scheme
    ]);

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let paths = discover_app_paths(app.handle().clone())
                .map_err(Box::<dyn std::error::Error>::from)?;
            let server = AgentServer::new_storage(
                paths.app_data_dir.join("library.sqlite3"),
                paths.diagnostics_dir,
            );
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
            app.manage(PrismWatcherController {
                watcher: Mutex::new(None),
                sync: Mutex::new(PrismSyncState::default()),
            });
            if let Ok((_, repository)) = library_repository(app.handle()) {
                if let Ok(Some(root)) = repository.get_prism_root() {
                    let _ = replace_prism_watcher(app.handle(), &root);
                    let _ = trigger_prism_background_sync(app.handle());
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run Minecraft Pack Builder desktop app");
}

#[cfg(test)]
mod tests {
    use super::*;
    use mpb_assets::PrismInstanceStatus as AssetPrismInstanceStatus;
    use serde_json::json;
    use tempfile::tempdir;

    fn prism_instance(root: &Path, identity_fingerprint: &str) -> PrismInstanceDescriptor {
        PrismInstanceDescriptor {
            instance_id: "prod-pack".to_string(),
            display_name: "Prod Pack".to_string(),
            instance_path: root.join("PrismLauncher/instances/prod-pack"),
            minecraft_dir: root.join("PrismLauncher/instances/prod-pack/.minecraft"),
            minecraft_version: Some("1.20.1".to_string()),
            loader: Some("Forge".to_string()),
            loader_version: Some("47.4.20".to_string()),
            identity_fingerprint: identity_fingerprint.to_string(),
            content_fingerprint: "prod-pack-content".to_string(),
            status: AssetPrismInstanceStatus::Pending,
            status_message: None,
        }
    }

    fn write_registry_report(
        diagnostics_dir: &Path,
        identity_fingerprint: &str,
        schema_version: u64,
    ) {
        let path = registry_report_path(diagnostics_dir, identity_fingerprint);
        std::fs::create_dir_all(diagnostics_dir).expect("diagnostics dir");
        std::fs::write(
            &path,
            json!({
                "schemaVersion": schema_version,
                "runtimeStatus": "ready"
            })
            .to_string(),
        )
        .expect("registry report");
        write_registry_metadata(
            diagnostics_dir,
            identity_fingerprint,
            schema_version as u32,
            "prod-pack-content",
            &path,
        );
    }

    fn write_registry_metadata(
        diagnostics_dir: &Path,
        identity_fingerprint: &str,
        schema_version: u32,
        content_fingerprint: &str,
        report_path: &Path,
    ) {
        let metadata = PrismAssetIndexMetadata {
            schema_version,
            status: "ready".to_string(),
            static_status: "ready".to_string(),
            runtime_status: "ready".to_string(),
            runtime_message: None,
            instance_id: "prod-pack".to_string(),
            identity_fingerprint: identity_fingerprint.to_string(),
            content_fingerprint: content_fingerprint.to_string(),
            minecraft_version: Some("1.20.1".to_string()),
            loader: Some("Forge".to_string()),
            archive_count: 1,
            block_count: 1,
            asset_count: 1,
            report_path: report_path.to_path_buf(),
        };
        std::fs::write(
            registry_metadata_path(diagnostics_dir, identity_fingerprint),
            serde_json::to_string(&metadata).expect("metadata json"),
        )
        .expect("registry metadata");
    }

    #[test]
    fn registry_report_freshness_uses_current_asset_schema_version() {
        let temp = tempdir().expect("temp dir");
        let diagnostics_dir = temp.path().join("diagnostics");
        let current = prism_instance(temp.path(), "current-identity");
        let stale = prism_instance(temp.path(), "stale-identity");

        write_registry_report(
            &diagnostics_dir,
            &current.identity_fingerprint,
            PRISM_REGISTRY_SCHEMA_VERSION as u64,
        );
        write_registry_report(&diagnostics_dir, &stale.identity_fingerprint, 4);

        assert!(registry_report_is_current(&diagnostics_dir, &current));
        assert!(!registry_report_is_current(&diagnostics_dir, &stale));
    }

    #[test]
    fn render_scene_registry_reader_keeps_only_scheme_block_metadata() {
        let temp = tempdir().expect("temp dir");
        let diagnostics_dir = temp.path().join("diagnostics");
        let identity = "filtered-identity";
        std::fs::create_dir_all(&diagnostics_dir).expect("diagnostics dir");
        let report_path = registry_report_path(&diagnostics_dir, identity);
        std::fs::write(
            &report_path,
            json!({
                "schemaVersion": PRISM_REGISTRY_SCHEMA_VERSION,
                "runtimeStatus": "ready",
                "blocks": [
                    {
                        "identifier": "minecraft:stone",
                        "displayName": "Stone",
                        "modelElements": []
                    },
                    {
                        "identifier": "mod:huge_unused_machine",
                        "displayName": "Huge Unused Machine",
                        "modelElements": (0..2048)
                            .map(|index| json!({
                                "from": [index, 0, 0],
                                "to": [index + 1, 1, 1],
                                "faceTexturePaths": { "north": format!("/tmp/{index}.png") },
                                "faceUvs": {}
                            }))
                            .collect::<Vec<_>>()
                    }
                ]
            })
            .to_string(),
        )
        .expect("registry report");
        write_registry_metadata(
            &diagnostics_dir,
            identity,
            PRISM_REGISTRY_SCHEMA_VERSION,
            "prod-pack-content",
            &report_path,
        );

        let instance = prism_instance(temp.path(), identity);
        assert!(registry_report_is_current(&diagnostics_dir, &instance));

        let report = read_registry_report_for_block_ids(
            &diagnostics_dir,
            identity,
            &["minecraft:stone".to_string()].into_iter().collect(),
        )
        .expect("filtered registry report");
        let blocks = report["blocks"].as_array().expect("blocks");

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["identifier"], "minecraft:stone");
        assert!(!report.to_string().contains("huge_unused_machine"));
    }

    #[test]
    fn registry_freshness_reads_metadata_without_parsing_registry_report() {
        let temp = tempdir().expect("temp dir");
        let diagnostics_dir = temp.path().join("diagnostics");
        let identity = "metadata-only-identity";
        std::fs::create_dir_all(&diagnostics_dir).expect("diagnostics dir");
        let report_path = registry_report_path(&diagnostics_dir, identity);
        std::fs::write(
            &report_path,
            r#"{"schemaVersion":6,"runtimeStatus":"ready","blocks":[{"identifier":"broken""#,
        )
        .expect("registry report");
        write_registry_metadata(
            &diagnostics_dir,
            identity,
            PRISM_REGISTRY_SCHEMA_VERSION,
            "prod-pack-content",
            &report_path,
        );

        let instance = prism_instance(temp.path(), identity);
        assert!(registry_report_is_current(&diagnostics_dir, &instance));
    }

    #[test]
    fn registry_freshness_accepts_legacy_current_report_without_metadata() {
        let temp = tempdir().expect("temp dir");
        let diagnostics_dir = temp.path().join("diagnostics");
        let current = prism_instance(temp.path(), "legacy-current");
        let stale = prism_instance(temp.path(), "legacy-stale");
        std::fs::create_dir_all(&diagnostics_dir).expect("diagnostics dir");
        std::fs::write(
            registry_report_path(&diagnostics_dir, &current.identity_fingerprint),
            format!(
                r#"{{"schemaVersion":{},"runtimeStatus":"ready","blocks":[{{"identifier":"minecraft:stone","modelElements":[{}]}}]}}"#,
                PRISM_REGISTRY_SCHEMA_VERSION,
                "0,".repeat(2048)
            ),
        )
        .expect("current report");
        std::fs::write(
            registry_report_path(&diagnostics_dir, &stale.identity_fingerprint),
            r#"{"schemaVersion":4,"runtimeStatus":"ready","blocks":[]}"#,
        )
        .expect("stale report");

        assert!(registry_report_is_current(&diagnostics_dir, &current));
        assert!(!registry_report_is_current(&diagnostics_dir, &stale));
    }

    #[test]
    fn raw_registry_block_identifier_does_not_parse_the_whole_block() {
        let raw_block = r#"{
            "identifier": "mod:heavy_machine",
            "modelElements": [ this tail intentionally is not valid json ]
        }"#;

        assert_eq!(
            raw_registry_block_identifier(raw_block),
            Some("mod:heavy_machine")
        );
    }
}

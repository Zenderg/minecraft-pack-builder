//! CurseForge release discovery, modpack downloads, and asset import helpers.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const CURSEFORGE_GAME_ID: u64 = 432;
const CURSEFORGE_MODPACK_CLASS_ID: u64 = 4471;
const CURSEFORGE_API_BASE: &str = "https://api.curseforge.com/v1";
const CURSEFORGE_SEARCH_INDEX: u64 = 0;
const CURSEFORGE_SEARCH_PAGE_SIZE: u64 = 25;
const CURSEFORGE_SEARCH_SORT_FIELD_FEATURED: u64 = 1;

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("only CurseForge modpack page URLs are supported")]
    UnsupportedUrl,
    #[error("CurseForge modpack was not found for slug '{slug}'")]
    ModpackNotFound { slug: String },
    #[error("CurseForge API request failed: {0}")]
    Http(String),
    #[error("CurseForge API response was not usable: {0}")]
    Api(String),
    #[error("release file {file_id} is missing a download URL")]
    MissingDownloadUrl { file_id: u64 },
    #[error("download was cancelled")]
    Cancelled,
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip archive could not be parsed: {0}")]
    Zip(String),
    #[error("modpack archive is missing manifest.json")]
    MissingManifest,
    #[error("modpack manifest could not be parsed: {0}")]
    Manifest(String),
    #[error("modpack did not contain any parseable block assets")]
    NoParseableBlocks,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedModpackUrl {
    pub slug: String,
    pub normalized_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeProject {
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub logo_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeRelease {
    pub file_id: u64,
    pub display_name: String,
    pub file_name: String,
    pub download_url: Option<String>,
    pub game_versions: Vec<String>,
    pub file_date: String,
    pub file_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseSummary {
    pub file_id: u64,
    pub version_name: String,
    pub file_name: String,
    pub minecraft_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub file_date: String,
    pub file_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredReleases {
    pub modpack: CurseForgeProject,
    pub source_url: String,
    pub releases: Vec<ReleaseSummary>,
    pub minecraft_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub default_file_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseFilter {
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadedArchive {
    pub path: PathBuf,
    pub bytes_downloaded: u64,
}

#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

pub trait CurseForgeGateway {
    fn search_modpack_projects(
        &self,
        api_key: &str,
        query: &str,
    ) -> Result<Vec<CurseForgeProject>, AssetError>;

    fn find_modpack_project(
        &self,
        api_key: &str,
        slug: &str,
    ) -> Result<Option<CurseForgeProject>, AssetError>;

    fn list_project_files(
        &self,
        api_key: &str,
        project_id: u64,
    ) -> Result<Vec<CurseForgeRelease>, AssetError>;

    fn open_download(
        &self,
        api_key: &str,
        release: &CurseForgeRelease,
    ) -> Result<Box<dyn Read>, AssetError>;

    fn open_mod_file_download(
        &self,
        _api_key: &str,
        project_id: u64,
        file_id: u64,
    ) -> Result<Box<dyn Read>, AssetError> {
        Err(AssetError::Api(format!(
            "mod file download is not available for project {project_id} file {file_id}"
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModpackAssetImportRequest {
    pub archive_path: PathBuf,
    pub cache_dir: PathBuf,
    pub diagnostics_dir: PathBuf,
    pub source_slug: String,
    pub release_name: String,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetImportReport {
    pub status: String,
    pub selected_release: String,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub mod_file_count: usize,
    pub block_count: usize,
    pub asset_count: usize,
    pub cache_location: PathBuf,
    pub report_path: PathBuf,
    pub blocks: Vec<BlockAssetSample>,
    pub texture_atlas: TextureAtlasMetadata,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetImportEvent {
    pub message: String,
    pub progress: Option<AssetImportProgress>,
}

impl AssetImportEvent {
    fn message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            progress: None,
        }
    }

    fn progress(message: impl Into<String>, completed: usize, total: usize) -> Self {
        Self {
            message: message.into(),
            progress: Some(AssetImportProgress {
                completed: completed as u64,
                total: total as u64,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetImportProgress {
    pub completed: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockAssetSample {
    pub identifier: String,
    pub display_name: String,
    pub namespace: String,
    pub model: Option<String>,
    pub texture_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureAtlasMetadata {
    pub textures: Vec<TextureAtlasEntry>,
    pub tile_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureAtlasEntry {
    pub identifier: String,
    pub source_path: PathBuf,
    pub tile_index: usize,
}

pub fn search_modpack_projects(
    gateway: &impl CurseForgeGateway,
    api_key: &str,
    query: &str,
) -> Result<Vec<CurseForgeProject>, AssetError> {
    let trimmed = query.trim();
    if trimmed.len() < 2 {
        return Ok(Vec::new());
    }

    gateway.search_modpack_projects(api_key, trimmed)
}

pub fn parse_modpack_page_url(value: &str) -> Result<ParsedModpackUrl, AssetError> {
    let trimmed = value.trim();
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .ok_or(AssetError::UnsupportedUrl)?;
    let (host, path_with_query) = without_scheme
        .split_once('/')
        .ok_or(AssetError::UnsupportedUrl)?;

    if host != "www.curseforge.com" && host != "curseforge.com" {
        return Err(AssetError::UnsupportedUrl);
    }

    let path = path_with_query
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .trim_end_matches('/');
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != "minecraft" || parts[1] != "modpacks" {
        return Err(AssetError::UnsupportedUrl);
    }

    let slug = parts[2].trim();
    if slug.is_empty() {
        return Err(AssetError::UnsupportedUrl);
    }

    Ok(ParsedModpackUrl {
        slug: slug.to_string(),
        normalized_url: format!("https://www.curseforge.com/minecraft/modpacks/{slug}"),
    })
}

pub fn discover_modpack_releases(
    gateway: &impl CurseForgeGateway,
    api_key: &str,
    page_url: &str,
) -> Result<DiscoveredReleases, AssetError> {
    let parsed = parse_modpack_page_url(page_url)?;
    let modpack = gateway
        .find_modpack_project(api_key, &parsed.slug)?
        .ok_or_else(|| AssetError::ModpackNotFound {
            slug: parsed.slug.clone(),
        })?;
    let mut files = gateway.list_project_files(api_key, modpack.id)?;
    files.sort_by(|left, right| right.file_date.cmp(&left.file_date));

    let releases = files
        .iter()
        .map(release_to_summary)
        .collect::<Vec<ReleaseSummary>>();
    let default_file_id = releases
        .first()
        .map(|release| release.file_id)
        .ok_or_else(|| AssetError::Api("modpack has no downloadable releases".to_string()))?;
    let minecraft_versions = unique_ordered_versions(
        releases
            .iter()
            .flat_map(|release| release.minecraft_versions.iter().cloned()),
    );
    let mut loaders = unique_ordered_versions(
        releases
            .iter()
            .flat_map(|release| release.loaders.iter().cloned()),
    );
    loaders.sort();

    Ok(DiscoveredReleases {
        modpack,
        source_url: parsed.normalized_url,
        releases,
        minecraft_versions,
        loaders,
        default_file_id,
    })
}

pub fn filter_releases<'a>(
    releases: &'a [ReleaseSummary],
    filter: &ReleaseFilter,
) -> Vec<&'a ReleaseSummary> {
    releases
        .iter()
        .filter(|release| {
            filter
                .minecraft_version
                .as_ref()
                .is_none_or(|version| release.minecraft_versions.contains(version))
        })
        .filter(|release| {
            filter
                .loader
                .as_ref()
                .is_none_or(|loader| release.loaders.contains(loader))
        })
        .collect()
}

pub fn download_release_archive(
    gateway: &impl CurseForgeGateway,
    api_key: &str,
    release: &CurseForgeRelease,
    destination: &Path,
    cancellation: &CancellationToken,
    mut on_progress: impl FnMut(DownloadProgress),
) -> Result<DownloadedArchive, AssetError> {
    if cancellation.is_cancelled() {
        return Err(AssetError::Cancelled);
    }

    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut reader = gateway.open_download(api_key, release)?;
    let temporary_path = destination.with_extension("download");
    let mut file = File::create(&temporary_path)?;
    let mut buffer = [0_u8; 16 * 1024];
    let mut bytes_downloaded = 0_u64;

    loop {
        if cancellation.is_cancelled() {
            drop(file);
            let _ = std::fs::remove_file(&temporary_path);
            let _ = std::fs::remove_file(destination);
            return Err(AssetError::Cancelled);
        }

        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        bytes_downloaded += read as u64;
        on_progress(DownloadProgress {
            bytes_downloaded,
            total_bytes: Some(release.file_length),
        });
    }

    drop(file);
    std::fs::rename(&temporary_path, destination)?;
    Ok(DownloadedArchive {
        path: destination.to_path_buf(),
        bytes_downloaded,
    })
}

pub fn build_modpack_asset_index(
    gateway: &impl CurseForgeGateway,
    api_key: &str,
    request: ModpackAssetImportRequest,
) -> Result<AssetImportReport, AssetError> {
    build_modpack_asset_index_with_events(
        gateway,
        api_key,
        request,
        &CancellationToken::new(),
        |_| {},
    )
}

pub fn build_modpack_asset_index_with_events(
    gateway: &impl CurseForgeGateway,
    api_key: &str,
    request: ModpackAssetImportRequest,
    cancellation: &CancellationToken,
    mut on_event: impl FnMut(AssetImportEvent),
) -> Result<AssetImportReport, AssetError> {
    ensure_not_cancelled(cancellation)?;
    on_event(AssetImportEvent::message("Preparing asset cache..."));
    fs::create_dir_all(&request.cache_dir)?;
    fs::create_dir_all(&request.diagnostics_dir)?;

    let extracted_modpack_dir = request.cache_dir.join("extracted").join("modpack");
    let extracted_mods_dir = request.cache_dir.join("extracted").join("mods");
    let downloaded_mods_dir = request.cache_dir.join("mods");
    recreate_dir(&extracted_modpack_dir)?;
    recreate_dir(&extracted_mods_dir)?;
    fs::create_dir_all(&downloaded_mods_dir)?;

    ensure_not_cancelled(cancellation)?;
    on_event(AssetImportEvent::message("Extracting modpack archive..."));
    let extracted_count = extract_zip_file_with_cancellation(
        &request.archive_path,
        &extracted_modpack_dir,
        cancellation,
    )?;
    on_event(AssetImportEvent::message(format!(
        "Extracted {extracted_count} files from modpack archive."
    )));
    let manifest_path = extracted_modpack_dir.join("manifest.json");
    if !manifest_path.exists() {
        return Err(AssetError::MissingManifest);
    }
    ensure_not_cancelled(cancellation)?;
    on_event(AssetImportEvent::message("Reading CurseForge manifest..."));
    let manifest = parse_manifest(&manifest_path)?;
    on_event(AssetImportEvent::message(format!(
        "Manifest references {} mod files.",
        manifest.files.len()
    )));
    let mut collector = AssetCollector::default();
    ensure_not_cancelled(cancellation)?;
    on_event(AssetImportEvent::message(
        "Scanning modpack overrides for assets...",
    ));
    collector.scan_root_with_cancellation(&extracted_modpack_dir, cancellation)?;

    for (index, manifest_file) in manifest.files.iter().enumerate() {
        ensure_not_cancelled(cancellation)?;
        let position = index + 1;
        let total = manifest.files.len();
        let mod_archive_path = downloaded_mods_dir.join(format!(
            "{}-{}.jar",
            manifest_file.project_id, manifest_file.file_id
        ));
        on_event(AssetImportEvent::message(format!(
            "Resolving mod file {position}/{total}: {}/{}",
            manifest_file.project_id, manifest_file.file_id
        )));
        let mut reader = match gateway.open_mod_file_download(
            api_key,
            manifest_file.project_id,
            manifest_file.file_id,
        ) {
            Ok(reader) => reader,
            Err(error) => {
                let warning = format!(
                    "Skipped mod file {}/{}: {}",
                    manifest_file.project_id, manifest_file.file_id, error
                );
                on_event(AssetImportEvent::progress(warning.clone(), position, total));
                collector.warnings.push(warning);
                continue;
            }
        };
        ensure_not_cancelled(cancellation)?;
        on_event(AssetImportEvent::message(format!(
            "Downloading mod file {position}/{total}..."
        )));
        let mut writer = File::create(&mod_archive_path)?;
        copy_reader_with_cancellation(&mut reader, &mut writer, cancellation)?;
        drop(writer);

        let mod_extract_dir = extracted_mods_dir.join(format!(
            "{}-{}",
            manifest_file.project_id, manifest_file.file_id
        ));
        ensure_not_cancelled(cancellation)?;
        on_event(AssetImportEvent::message(format!(
            "Extracting mod file {position}/{total}..."
        )));
        let extracted_count = match extract_zip_file_with_cancellation(
            &mod_archive_path,
            &mod_extract_dir,
            cancellation,
        ) {
            Ok(extracted_count) => extracted_count,
            Err(error) => {
                let warning = format!(
                    "Skipped mod file {}/{}: {}",
                    manifest_file.project_id, manifest_file.file_id, error
                );
                on_event(AssetImportEvent::progress(warning.clone(), position, total));
                collector.warnings.push(warning);
                continue;
            }
        };
        ensure_not_cancelled(cancellation)?;
        on_event(AssetImportEvent::message(format!(
            "Scanning mod file {position}/{total} ({extracted_count} files)..."
        )));
        if let Err(error) = collector.scan_root_with_cancellation(&mod_extract_dir, cancellation) {
            let warning = format!(
                "Skipped mod file {}/{}: {}",
                manifest_file.project_id, manifest_file.file_id, error
            );
            on_event(AssetImportEvent::progress(warning.clone(), position, total));
            collector.warnings.push(warning);
        } else {
            on_event(AssetImportEvent::progress(
                format!("Indexed mod file {position}/{total}."),
                position,
                total,
            ));
        }
    }

    ensure_not_cancelled(cancellation)?;
    on_event(AssetImportEvent::message(
        "Building block registry from collected assets...",
    ));
    let blocks = collector.block_samples();
    if blocks.is_empty() {
        return Err(AssetError::NoParseableBlocks);
    }
    let texture_atlas = texture_atlas_metadata(&blocks);
    let report_path = request.diagnostics_dir.join(format!(
        "{}-assets.json",
        safe_path_segment(&request.source_slug)
    ));
    let report = AssetImportReport {
        status: "imported".to_string(),
        selected_release: request.release_name,
        minecraft_version: request.minecraft_version.or_else(|| {
            manifest
                .minecraft
                .as_ref()
                .map(|minecraft| minecraft.version.clone())
        }),
        loader: request.loader.or_else(|| {
            manifest
                .minecraft
                .as_ref()
                .and_then(|minecraft| minecraft.primary_loader())
        }),
        mod_file_count: manifest.files.len(),
        block_count: blocks.len(),
        asset_count: collector.asset_paths_seen.len(),
        cache_location: request.cache_dir,
        report_path: report_path.clone(),
        blocks,
        texture_atlas,
        warnings: collector.warnings,
    };
    ensure_not_cancelled(cancellation)?;
    on_event(AssetImportEvent::message(format!(
        "Writing asset diagnostics report with {} blocks and {} assets...",
        report.block_count, report.asset_count
    )));
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| AssetError::Api(error.to_string()))?;
    fs::write(&report_path, json)?;
    on_event(AssetImportEvent::message(format!(
        "Asset diagnostics report written: {}",
        report_path.display()
    )));
    Ok(report)
}

fn recreate_dir(path: &Path) -> Result<(), AssetError> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn extract_zip_file_with_cancellation(
    archive_path: &Path,
    destination: &Path,
    cancellation: &CancellationToken,
) -> Result<usize, AssetError> {
    ensure_not_cancelled(cancellation)?;
    let file = File::open(archive_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| AssetError::Zip(error.to_string()))?;
    let mut extracted_file_count = 0;

    for index in 0..archive.len() {
        ensure_not_cancelled(cancellation)?;
        let mut entry = archive
            .by_index(index)
            .map_err(|error| AssetError::Zip(error.to_string()))?;
        let Some(enclosed_name) = entry.enclosed_name() else {
            continue;
        };
        let output_path = destination.join(enclosed_name);
        if entry.is_dir() {
            fs::create_dir_all(&output_path)?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(&output_path)?;
        copy_reader_with_cancellation(&mut entry, &mut output, cancellation)?;
        extracted_file_count += 1;
    }

    Ok(extracted_file_count)
}

fn copy_reader_with_cancellation(
    reader: &mut impl Read,
    writer: &mut impl Write,
    cancellation: &CancellationToken,
) -> Result<u64, AssetError> {
    let mut buffer = [0_u8; 16 * 1024];
    let mut copied = 0_u64;
    loop {
        ensure_not_cancelled(cancellation)?;
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            return Ok(copied);
        }
        writer.write_all(&buffer[..read])?;
        copied += read as u64;
    }
}

fn ensure_not_cancelled(cancellation: &CancellationToken) -> Result<(), AssetError> {
    if cancellation.is_cancelled() {
        return Err(AssetError::Cancelled);
    }
    Ok(())
}

fn parse_manifest(path: &Path) -> Result<CurseForgeManifest, AssetError> {
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(|error| AssetError::Manifest(error.to_string()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeManifest {
    minecraft: Option<ManifestMinecraft>,
    #[serde(default)]
    files: Vec<ManifestFile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestMinecraft {
    version: String,
    #[serde(default)]
    mod_loaders: Vec<ManifestLoader>,
}

impl ManifestMinecraft {
    fn primary_loader(&self) -> Option<String> {
        self.mod_loaders
            .iter()
            .find(|loader| loader.primary)
            .or_else(|| self.mod_loaders.first())
            .map(|loader| {
                loader
                    .id
                    .split('-')
                    .next()
                    .unwrap_or(&loader.id)
                    .to_string()
            })
    }
}

#[derive(Debug, Deserialize)]
struct ManifestLoader {
    id: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Debug, Deserialize)]
struct ManifestFile {
    #[serde(rename = "projectID")]
    project_id: u64,
    #[serde(rename = "fileID")]
    file_id: u64,
}

#[derive(Default)]
struct AssetCollector {
    languages: BTreeMap<String, String>,
    blockstates: BTreeMap<String, BlockstateAsset>,
    models: BTreeMap<String, ModelAsset>,
    textures: BTreeMap<String, PathBuf>,
    asset_paths_seen: BTreeSet<PathBuf>,
    warnings: Vec<String>,
}

impl AssetCollector {
    fn scan_root_with_cancellation(
        &mut self,
        root: &Path,
        cancellation: &CancellationToken,
    ) -> Result<(), AssetError> {
        ensure_not_cancelled(cancellation)?;
        let mut files = Vec::new();
        collect_files_with_cancellation(root, &mut files, cancellation)?;
        for file in files {
            ensure_not_cancelled(cancellation)?;
            let Ok(relative) = file.strip_prefix(root) else {
                continue;
            };
            let Some(asset_path) = parse_asset_path(relative) else {
                continue;
            };
            self.asset_paths_seen.insert(relative.to_path_buf());
            let result = match asset_path.kind.as_str() {
                "lang" if extension_is(&file, "json") => self.read_language_file(&file),
                "blockstates" if extension_is(&file, "json") => {
                    self.read_blockstate_file(asset_path, &file)
                }
                "models" if extension_is(&file, "json") => self.read_model_file(asset_path, &file),
                "textures" if extension_is(&file, "png") => {
                    let id = format!(
                        "{}:{}",
                        asset_path.namespace,
                        without_extension(&asset_path.relative_asset_path)
                    );
                    self.textures.insert(id, file.clone());
                    Ok(())
                }
                _ => Ok(()),
            };
            if let Err(error) = result {
                self.warnings
                    .push(format!("Skipped asset {}: {error}", file.display()));
            }
        }
        Ok(())
    }

    fn read_language_file(&mut self, path: &Path) -> Result<(), AssetError> {
        let value = read_json(path)?;
        let Some(object) = value.as_object() else {
            return Ok(());
        };
        for (key, value) in object {
            if let Some(text) = value.as_str() {
                self.languages.insert(key.clone(), text.to_string());
            }
        }
        Ok(())
    }

    fn read_blockstate_file(
        &mut self,
        asset_path: AssetPath,
        path: &Path,
    ) -> Result<(), AssetError> {
        let value = read_json(path)?;
        let identifier = format!(
            "{}:{}",
            asset_path.namespace,
            without_extension(&asset_path.relative_asset_path)
        );
        self.blockstates.insert(
            identifier.clone(),
            BlockstateAsset {
                identifier,
                namespace: asset_path.namespace,
                models: collect_string_fields(&value, "model"),
            },
        );
        Ok(())
    }

    fn read_model_file(&mut self, asset_path: AssetPath, path: &Path) -> Result<(), AssetError> {
        let value = read_json(path)?;
        let id = format!(
            "{}:{}",
            asset_path.namespace,
            without_extension(&asset_path.relative_asset_path)
        );
        self.models.insert(
            id,
            ModelAsset {
                textures: collect_model_texture_values(&value),
            },
        );
        Ok(())
    }

    fn block_samples(&self) -> Vec<BlockAssetSample> {
        self.blockstates
            .values()
            .map(|blockstate| {
                let model = blockstate
                    .models
                    .first()
                    .map(|model| normalize_asset_reference(model, &blockstate.namespace));
                let texture_path = model
                    .as_ref()
                    .and_then(|model_id| self.models.get(model_id))
                    .and_then(|model| {
                        model
                            .textures
                            .iter()
                            .find(|texture| !texture.starts_with('#'))
                            .map(|texture| {
                                normalize_asset_reference(texture, &blockstate.namespace)
                            })
                    })
                    .and_then(|texture_id| self.textures.get(&texture_id).cloned());
                let language_key = format!(
                    "block.{}.{}",
                    blockstate.namespace,
                    blockstate
                        .identifier
                        .split_once(':')
                        .map(|(_, path)| path.replace('/', "."))
                        .unwrap_or_else(|| blockstate.identifier.replace('/', "."))
                );
                BlockAssetSample {
                    identifier: blockstate.identifier.clone(),
                    display_name: self
                        .languages
                        .get(&language_key)
                        .cloned()
                        .unwrap_or_else(|| blockstate.identifier.clone()),
                    namespace: blockstate.namespace.clone(),
                    model,
                    texture_path,
                }
            })
            .collect()
    }
}

struct BlockstateAsset {
    identifier: String,
    namespace: String,
    models: Vec<String>,
}

struct ModelAsset {
    textures: Vec<String>,
}

struct AssetPath {
    namespace: String,
    kind: String,
    relative_asset_path: String,
}

fn collect_files_with_cancellation(
    root: &Path,
    files: &mut Vec<PathBuf>,
    cancellation: &CancellationToken,
) -> Result<(), AssetError> {
    ensure_not_cancelled(cancellation)?;
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        ensure_not_cancelled(cancellation)?;
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_with_cancellation(&path, files, cancellation)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

fn parse_asset_path(path: &Path) -> Option<AssetPath> {
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let assets_index = components
        .iter()
        .position(|component| component == "assets")?;
    let namespace = components.get(assets_index + 1)?.clone();
    let kind = components.get(assets_index + 2)?.clone();
    let relative_asset_path = components
        .get(assets_index + 3..)?
        .join("/")
        .trim()
        .to_string();
    if namespace.is_empty() || kind.is_empty() || relative_asset_path.is_empty() {
        return None;
    }
    Some(AssetPath {
        namespace,
        kind,
        relative_asset_path,
    })
}

fn read_json(path: &Path) -> Result<serde_json::Value, AssetError> {
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(|error| AssetError::Api(error.to_string()))
}

fn collect_string_fields(value: &serde_json::Value, field: &str) -> Vec<String> {
    let mut values = Vec::new();
    collect_string_fields_into(value, field, &mut values);
    values
}

fn collect_string_fields_into(value: &serde_json::Value, field: &str, values: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                if key == field {
                    if let Some(text) = value.as_str() {
                        values.push(text.to_string());
                    }
                }
                collect_string_fields_into(value, field, values);
            }
        }
        serde_json::Value::Array(array) => {
            for item in array {
                collect_string_fields_into(item, field, values);
            }
        }
        _ => {}
    }
}

fn collect_model_texture_values(value: &serde_json::Value) -> Vec<String> {
    value
        .get("textures")
        .and_then(|textures| textures.as_object())
        .map(|textures| {
            textures
                .values()
                .filter_map(|texture| texture.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn normalize_asset_reference(value: &str, fallback_namespace: &str) -> String {
    if value.contains(':') {
        value.to_string()
    } else {
        format!("{fallback_namespace}:{value}")
    }
}

fn texture_atlas_metadata(blocks: &[BlockAssetSample]) -> TextureAtlasMetadata {
    let mut seen = BTreeSet::new();
    let textures = blocks
        .iter()
        .filter_map(|block| {
            let path = block.texture_path.as_ref()?;
            let key = path.to_string_lossy().to_string();
            if !seen.insert(key) {
                return None;
            }
            Some((block.identifier.clone(), path.clone()))
        })
        .enumerate()
        .map(
            |(tile_index, (identifier, source_path))| TextureAtlasEntry {
                identifier,
                source_path,
                tile_index,
            },
        )
        .collect();

    TextureAtlasMetadata {
        textures,
        tile_size: 16,
    }
}

fn without_extension(path: &str) -> String {
    Path::new(path)
        .with_extension("")
        .to_string_lossy()
        .replace('\\', "/")
}

fn extension_is(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
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
        "modpack".to_string()
    } else {
        cleaned
    }
}

fn release_to_summary(release: &CurseForgeRelease) -> ReleaseSummary {
    ReleaseSummary {
        file_id: release.file_id,
        version_name: release.display_name.clone(),
        file_name: release.file_name.clone(),
        minecraft_versions: release
            .game_versions
            .iter()
            .filter(|version| looks_like_minecraft_version(version))
            .cloned()
            .collect(),
        loaders: release
            .game_versions
            .iter()
            .filter(|version| looks_like_loader(version))
            .cloned()
            .collect(),
        file_date: release.file_date.clone(),
        file_length: release.file_length,
    }
}

fn looks_like_minecraft_version(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
}

fn looks_like_loader(value: &str) -> bool {
    matches!(value, "Forge" | "NeoForge" | "Fabric" | "Quilt")
}

fn unique_ordered_versions(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .filter(|value| seen.insert(value.clone()))
        .collect::<Vec<_>>()
}

#[derive(Debug, Clone)]
pub struct CurseForgeHttpGateway {
    client: Client,
}

impl CurseForgeHttpGateway {
    pub fn new() -> Result<Self, AssetError> {
        let client = Client::builder()
            .user_agent("MinecraftPackBuilder/0.1")
            .build()
            .map_err(|error| AssetError::Http(error.to_string()))?;
        Ok(Self { client })
    }

    fn api_headers(api_key: &str) -> Result<HeaderMap, AssetError> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(api_key.trim())
                .map_err(|error| AssetError::Http(error.to_string()))?,
        );
        Ok(headers)
    }
}

fn modpack_search_query_params(query: &str) -> Vec<(&'static str, String)> {
    vec![
        ("gameId", CURSEFORGE_GAME_ID.to_string()),
        ("classId", CURSEFORGE_MODPACK_CLASS_ID.to_string()),
        ("searchFilter", query.to_string()),
        ("index", CURSEFORGE_SEARCH_INDEX.to_string()),
        ("pageSize", CURSEFORGE_SEARCH_PAGE_SIZE.to_string()),
        (
            "sortField",
            CURSEFORGE_SEARCH_SORT_FIELD_FEATURED.to_string(),
        ),
        ("sortOrder", "desc".to_string()),
    ]
}

impl CurseForgeGateway for CurseForgeHttpGateway {
    fn search_modpack_projects(
        &self,
        api_key: &str,
        query: &str,
    ) -> Result<Vec<CurseForgeProject>, AssetError> {
        let response = self
            .client
            .get(format!("{CURSEFORGE_API_BASE}/mods/search"))
            .headers(Self::api_headers(api_key)?)
            .query(&modpack_search_query_params(query))
            .send()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .error_for_status()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .json::<CurseForgeSearchResponse>()
            .map_err(|error| AssetError::Api(error.to_string()))?;

        Ok(response
            .data
            .into_iter()
            .map(|modpack| CurseForgeProject {
                id: modpack.id,
                name: modpack.name,
                slug: modpack.slug,
                logo_url: modpack
                    .logo
                    .and_then(|logo| logo.thumbnail_url.or(logo.url)),
            })
            .collect())
    }

    fn find_modpack_project(
        &self,
        api_key: &str,
        slug: &str,
    ) -> Result<Option<CurseForgeProject>, AssetError> {
        let response = self
            .client
            .get(format!("{CURSEFORGE_API_BASE}/mods/search"))
            .headers(Self::api_headers(api_key)?)
            .query(&[
                ("gameId", CURSEFORGE_GAME_ID.to_string()),
                ("classId", CURSEFORGE_MODPACK_CLASS_ID.to_string()),
                ("slug", slug.to_string()),
            ])
            .send()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .error_for_status()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .json::<CurseForgeSearchResponse>()
            .map_err(|error| AssetError::Api(error.to_string()))?;

        Ok(response
            .data
            .into_iter()
            .next()
            .map(|modpack| CurseForgeProject {
                id: modpack.id,
                name: modpack.name,
                slug: modpack.slug,
                logo_url: modpack
                    .logo
                    .and_then(|logo| logo.thumbnail_url.or(logo.url)),
            }))
    }

    fn list_project_files(
        &self,
        api_key: &str,
        project_id: u64,
    ) -> Result<Vec<CurseForgeRelease>, AssetError> {
        let response = self
            .client
            .get(format!("{CURSEFORGE_API_BASE}/mods/{project_id}/files"))
            .headers(Self::api_headers(api_key)?)
            .send()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .error_for_status()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .json::<CurseForgeFilesResponse>()
            .map_err(|error| AssetError::Api(error.to_string()))?;

        Ok(response
            .data
            .into_iter()
            .map(|file| CurseForgeRelease {
                file_id: file.id,
                display_name: file.display_name,
                file_name: file.file_name,
                download_url: file.download_url,
                game_versions: file.game_versions,
                file_date: file.file_date,
                file_length: file.file_length,
            })
            .collect())
    }

    fn open_download(
        &self,
        api_key: &str,
        release: &CurseForgeRelease,
    ) -> Result<Box<dyn Read>, AssetError> {
        let url = release
            .download_url
            .as_ref()
            .ok_or(AssetError::MissingDownloadUrl {
                file_id: release.file_id,
            })?;
        let response = self
            .client
            .get(url)
            .headers(Self::api_headers(api_key)?)
            .send()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .error_for_status()
            .map_err(|error| AssetError::Http(error.to_string()))?;
        Ok(Box::new(response))
    }

    fn open_mod_file_download(
        &self,
        api_key: &str,
        _project_id: u64,
        file_id: u64,
    ) -> Result<Box<dyn Read>, AssetError> {
        let response = self
            .client
            .post(format!("{CURSEFORGE_API_BASE}/mods/files"))
            .headers(Self::api_headers(api_key)?)
            .json(&CurseForgeFilesRequest {
                file_ids: vec![file_id],
            })
            .send()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .error_for_status()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .json::<CurseForgeFilesResponse>()
            .map_err(|error| AssetError::Api(error.to_string()))?;

        let download_url = response
            .data
            .into_iter()
            .find(|file| file.id == file_id)
            .and_then(|file| file.download_url)
            .ok_or(AssetError::MissingDownloadUrl { file_id })?;
        let response = self
            .client
            .get(download_url)
            .headers(Self::api_headers(api_key)?)
            .send()
            .map_err(|error| AssetError::Http(error.to_string()))?
            .error_for_status()
            .map_err(|error| AssetError::Http(error.to_string()))?;
        Ok(Box::new(response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modpack_search_matches_prismlauncher_default_search_params() {
        let params = modpack_search_query_params("create");

        assert!(params.contains(&("index", "0".to_string())));
        assert!(params.contains(&("pageSize", "25".to_string())));
        assert!(params.contains(&("searchFilter", "create".to_string())));
        assert!(params.contains(&("sortField", "1".to_string())));
        assert!(params.contains(&("sortOrder", "desc".to_string())));
        assert!(!params.iter().any(|(key, _)| *key == "page"));
        assert!(!params.iter().any(|(key, _)| *key == "sortBy"));
    }

    #[test]
    fn curseforge_files_request_uses_api_file_ids_field() {
        let json = serde_json::to_value(CurseForgeFilesRequest {
            file_ids: vec![8054109],
        })
        .expect("serialize files request");

        assert_eq!(json, serde_json::json!({ "fileIds": [8054109] }));
    }
}

#[derive(Debug, Deserialize)]
struct CurseForgeSearchResponse {
    data: Vec<CurseForgeModDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeModDto {
    id: u64,
    name: String,
    slug: String,
    logo: Option<CurseForgeLogoDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeLogoDto {
    thumbnail_url: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CurseForgeFilesResponse {
    data: Vec<CurseForgeFileDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeFilesRequest {
    file_ids: Vec<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeFileDto {
    id: u64,
    display_name: String,
    file_name: String,
    download_url: Option<String>,
    game_versions: Vec<String>,
    file_date: String,
    file_length: u64,
}

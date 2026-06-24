use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{AssetError, CancellationToken, CurseForgeGateway};

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


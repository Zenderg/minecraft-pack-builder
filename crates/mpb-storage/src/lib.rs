//! Instance-local MPB runtime storage.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use mpb_core::{ConstructionStage, Coordinate, Dimensions, Scheme, SchemeBlock};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("failed to create directory at {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("file storage error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("scheme id must contain only ASCII letters, digits, '.', '_' or '-', got {0}")]
    InvalidSchemeId(String),
    #[error("stored scheme content is invalid: {0}")]
    InvalidSchemeContent(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceMpbLayout {
    pub instance_root: PathBuf,
    pub mpb_dir: PathBuf,
    pub config_path: PathBuf,
    pub schemes_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub patch_manifest_path: PathBuf,
}

impl InstanceMpbLayout {
    pub fn prepare(instance_root: impl AsRef<Path>) -> Result<Self, StorageError> {
        let instance_root = instance_root.as_ref().to_path_buf();
        let mpb_dir = instance_root.join("mpb");
        let config_path = mpb_dir.join("config.json");
        let schemes_dir = mpb_dir.join("schemes");
        let cache_dir = mpb_dir.join("cache");
        let patch_manifest_path = mpb_dir.join("patch-manifest.json");

        create_dir(&mpb_dir)?;
        create_dir(&schemes_dir)?;
        create_dir(&cache_dir)?;
        if !config_path.is_file() {
            write_json_file(
                &config_path,
                &RuntimeConfigFile {
                    schema_version: 1,
                    lan_mode: false,
                    active_scheme_id: None,
                },
            )?;
        }

        Ok(Self {
            instance_root,
            mpb_dir,
            config_path,
            schemes_dir,
            cache_dir,
            patch_manifest_path,
        })
    }

    fn scheme_path(&self, scheme_id: &str) -> Result<PathBuf, StorageError> {
        validate_scheme_id(scheme_id)?;
        Ok(self.schemes_dir.join(format!("{scheme_id}.mpb.json")))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeConfigFile {
    schema_version: u32,
    lan_mode: bool,
    active_scheme_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSchemeFile {
    pub scheme_id: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedInstanceScheme {
    pub scheme_id: String,
    pub scheme: Scheme,
    pub path: PathBuf,
}

pub struct InstanceSchemeRepository {
    layout: InstanceMpbLayout,
}

impl InstanceSchemeRepository {
    pub fn open(instance_root: impl AsRef<Path>) -> Result<Self, StorageError> {
        Ok(Self {
            layout: InstanceMpbLayout::prepare(instance_root)?,
        })
    }

    pub fn layout(&self) -> &InstanceMpbLayout {
        &self.layout
    }

    pub fn save_scheme(
        &self,
        scheme_id: &str,
        scheme: &Scheme,
    ) -> Result<SavedSchemeFile, StorageError> {
        let path = self.layout.scheme_path(scheme_id)?;
        let existing = read_scheme_file(&path).ok();
        let now = timestamp_string();
        let file = scheme_to_file(
            scheme_id,
            scheme,
            existing
                .as_ref()
                .map(|file| file.created_at.as_str())
                .unwrap_or(now.as_str()),
            now.as_str(),
        );
        write_json_file_atomic(&path, &file)?;
        Ok(SavedSchemeFile {
            scheme_id: scheme_id.to_string(),
            path,
        })
    }

    pub fn load_scheme(&self, scheme_id: &str) -> Result<LoadedInstanceScheme, StorageError> {
        let path = self.layout.scheme_path(scheme_id)?;
        let file = read_scheme_file(&path)?;
        let scheme = file_to_scheme(&file)?;
        Ok(LoadedInstanceScheme {
            scheme_id: file.scheme_id,
            scheme,
            path,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MpbSchemeFile {
    schema_version: u32,
    scheme_id: String,
    name: String,
    created_at: String,
    updated_at: String,
    palette: Vec<SchemePaletteEntry>,
    blocks: Vec<SchemeFileBlock>,
    stages: Vec<ConstructionStage>,
    semantic_regions: Vec<serde_json::Value>,
    agent_metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemePaletteEntry {
    palette_id: u32,
    block_id: String,
    states: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemeFileBlock {
    x: i32,
    y: i32,
    z: i32,
    palette_id: u32,
    stage_id: Option<u32>,
    block_entity_nbt: Option<serde_json::Value>,
}

fn create_dir(path: &Path) -> Result<(), StorageError> {
    fs::create_dir_all(path).map_err(|source| StorageError::CreateDir {
        path: path.to_path_buf(),
        source,
    })
}

fn validate_scheme_id(scheme_id: &str) -> Result<(), StorageError> {
    let valid = !scheme_id.is_empty()
        && scheme_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if valid {
        Ok(())
    } else {
        Err(StorageError::InvalidSchemeId(scheme_id.to_string()))
    }
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), StorageError> {
    let json = serde_json::to_vec_pretty(value)
        .map_err(|error| StorageError::InvalidSchemeContent(error.to_string()))?;
    fs::write(path, json).map_err(|source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn write_json_file_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), StorageError> {
    let tmp_path = path.with_extension("mpb.json.tmp");
    write_json_file(&tmp_path, value)?;
    fs::rename(&tmp_path, path).map_err(|source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn read_scheme_file(path: &Path) -> Result<MpbSchemeFile, StorageError> {
    let json = fs::read_to_string(path).map_err(|source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&json)
        .map_err(|error| StorageError::InvalidSchemeContent(error.to_string()))
}

fn scheme_to_file(
    scheme_id: &str,
    scheme: &Scheme,
    created_at: &str,
    updated_at: &str,
) -> MpbSchemeFile {
    let mut palette_lookup = BTreeMap::<(String, BTreeMap<String, String>), u32>::new();
    let mut palette = Vec::<SchemePaletteEntry>::new();
    let mut blocks = Vec::new();

    for (coordinate, block) in scheme.blocks() {
        let key = (block.block_id.clone(), block.states.clone());
        let palette_id = match palette_lookup.get(&key) {
            Some(palette_id) => *palette_id,
            None => {
                let palette_id = palette.len() as u32;
                palette_lookup.insert(key.clone(), palette_id);
                palette.push(SchemePaletteEntry {
                    palette_id,
                    block_id: key.0,
                    states: key.1,
                });
                palette_id
            }
        };
        blocks.push(SchemeFileBlock {
            x: coordinate.x,
            y: coordinate.y,
            z: coordinate.z,
            palette_id,
            stage_id: match block.stage {
                mpb_core::StageRef::Unassigned => None,
                mpb_core::StageRef::Stage(id) => Some(id),
            },
            block_entity_nbt: None,
        });
    }

    MpbSchemeFile {
        schema_version: 1,
        scheme_id: scheme_id.to_string(),
        name: scheme.name().to_string(),
        created_at: created_at.to_string(),
        updated_at: updated_at.to_string(),
        palette,
        blocks,
        stages: scheme.stages().to_vec(),
        semantic_regions: Vec::new(),
        agent_metadata: serde_json::json!({}),
    }
}

fn file_to_scheme(file: &MpbSchemeFile) -> Result<Scheme, StorageError> {
    if file.schema_version != 1 {
        return Err(StorageError::InvalidSchemeContent(format!(
            "unsupported MPB scheme schema version {}",
            file.schema_version
        )));
    }
    validate_scheme_id(&file.scheme_id)?;
    let palette = file
        .palette
        .iter()
        .map(|entry| {
            (
                entry.palette_id,
                SchemeBlock {
                    block_id: entry.block_id.clone(),
                    states: entry.states.clone(),
                    stage: mpb_core::StageRef::Unassigned,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let blocks = file
        .blocks
        .iter()
        .map(|block| {
            let palette_block = palette.get(&block.palette_id).ok_or_else(|| {
                StorageError::InvalidSchemeContent(format!(
                    "block references missing palette id {}",
                    block.palette_id
                ))
            })?;
            let mut scheme_block = palette_block.clone();
            scheme_block.stage = block
                .stage_id
                .map(mpb_core::StageRef::Stage)
                .unwrap_or(mpb_core::StageRef::Unassigned);
            Ok((Coordinate::new(block.x, block.y, block.z), scheme_block))
        })
        .collect::<Result<Vec<_>, StorageError>>()?;
    let dimensions =
        file.blocks
            .iter()
            .fold(Dimensions { x: 0, y: 0, z: 0 }, |mut dimensions, block| {
                dimensions.x = dimensions.x.max(block.x + 1);
                dimensions.y = dimensions.y.max(block.y + 1);
                dimensions.z = dimensions.z.max(block.z + 1);
                dimensions
            });
    Scheme::from_persisted(&file.name, dimensions, file.stages.clone(), blocks)
        .map_err(|error| StorageError::InvalidSchemeContent(error.to_string()))
}

fn timestamp_string() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("unix:{seconds}")
}

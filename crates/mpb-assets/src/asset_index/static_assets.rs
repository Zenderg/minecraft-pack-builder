use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use crate::blockstate::{
    collect_blockstate_models, BlockstateModelReference, BlockstateModelReferences,
};
use crate::{AssetError, CancellationToken};

use super::{
    ensure_not_cancelled, BlockAssetSample, BlockModelVariantSample, BlockStatePropertySample,
    FaceTexturePaths, FaceUvs, ModelElementRotationSample, ModelElementSample, TextureAtlasEntry,
    TextureAtlasMetadata,
};

pub(crate) struct AssetCollector {
    texture_cache_dir: PathBuf,
    languages: BTreeMap<String, String>,
    blockstates: BTreeMap<String, BlockstateAsset>,
    items: BTreeMap<String, ItemAsset>,
    models: BTreeMap<String, ModelAsset>,
    textures: BTreeMap<String, PathBuf>,
    pub(crate) asset_paths_seen: BTreeSet<PathBuf>,
    pub(crate) warnings: Vec<String>,
}

impl AssetCollector {
    pub(crate) fn new(texture_cache_dir: PathBuf) -> Self {
        Self {
            texture_cache_dir,
            languages: BTreeMap::new(),
            blockstates: BTreeMap::new(),
            items: BTreeMap::new(),
            models: BTreeMap::new(),
            textures: BTreeMap::new(),
            asset_paths_seen: BTreeSet::new(),
            warnings: Vec::new(),
        }
    }

    pub(crate) fn scan_archive(
        &mut self,
        archive_path: &Path,
        cancellation: &CancellationToken,
    ) -> Result<(), AssetError> {
        ensure_not_cancelled(cancellation)?;
        let file = File::open(archive_path)?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|error| AssetError::Zip(error.to_string()))?;
        for index in 0..archive.len() {
            ensure_not_cancelled(cancellation)?;
            let mut entry = archive
                .by_index(index)
                .map_err(|error| AssetError::Zip(error.to_string()))?;
            if entry.is_dir() {
                continue;
            }
            let Some(enclosed_name) = entry.enclosed_name() else {
                continue;
            };
            let enclosed_name = enclosed_name.to_path_buf();
            if parse_asset_path(&enclosed_name).is_none() {
                continue;
            }
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            let display_path = PathBuf::from(format!(
                "{}::{}",
                archive_path.display(),
                enclosed_name.to_string_lossy()
            ));
            self.scan_asset_entry(&enclosed_name, Some(display_path), &bytes);
        }
        Ok(())
    }

    pub(crate) fn scan_asset_entry(
        &mut self,
        relative_path: &Path,
        _source_path: Option<PathBuf>,
        bytes: &[u8],
    ) {
        let Some(asset_path) = parse_asset_path(relative_path) else {
            return;
        };
        self.asset_paths_seen.insert(relative_path.to_path_buf());
        let result = match asset_path.kind.as_str() {
            "lang" if extension_is(relative_path, "json") => self.read_language_bytes(bytes),
            "blockstates" if extension_is(relative_path, "json") => {
                self.read_blockstate_bytes(asset_path, bytes)
            }
            "models" if extension_is(relative_path, "json") => {
                self.read_model_bytes(asset_path, bytes)
            }
            "items" if extension_is(relative_path, "json") => {
                self.read_item_bytes(asset_path, bytes)
            }
            "textures" if extension_is(relative_path, "png") => {
                let id = format!(
                    "{}:{}",
                    asset_path.namespace,
                    without_extension(&asset_path.relative_asset_path)
                );
                match self.cache_texture(&asset_path, bytes) {
                    Ok(path) => {
                        self.textures.insert(id, path);
                        Ok(())
                    }
                    Err(error) => Err(error),
                }
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.warnings.push(format!(
                "Skipped asset {}: {error}",
                relative_path.display()
            ));
        }
    }

    pub(crate) fn block_samples(&self) -> Vec<BlockAssetSample> {
        self.blockstates
            .values()
            .map(|blockstate| {
                let item_id = Some(blockstate.identifier.clone());
                let max_stack_size = item_id
                    .as_ref()
                    .and_then(|id| self.items.get(id))
                    .and_then(|item| item.max_stack_size);
                let model_variants = blockstate
                    .models
                    .models
                    .iter()
                    .map(|variant| self.block_model_variant_sample(blockstate, variant))
                    .collect::<Vec<_>>();
                let model = model_variants
                    .first()
                    .and_then(|variant| variant.model.clone());
                let texture_path = model_variants
                    .first()
                    .and_then(|variant| variant.texture_path.clone());
                let face_texture_paths = model_variants
                    .first()
                    .and_then(|variant| variant.face_texture_paths.clone());
                let model_elements = model_variants
                    .first()
                    .map(|variant| variant.model_elements.clone())
                    .unwrap_or_default();
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
                    item_id,
                    max_stack_size,
                    display_name: self
                        .languages
                        .get(&language_key)
                        .cloned()
                        .unwrap_or_else(|| blockstate.identifier.clone()),
                    namespace: blockstate.namespace.clone(),
                    allowed_states: blockstate
                        .models
                        .state_definitions
                        .iter()
                        .map(|(name, values)| BlockStatePropertySample {
                            name: name.clone(),
                            values: values.iter().cloned().collect(),
                        })
                        .collect(),
                    model,
                    texture_path,
                    face_texture_paths,
                    model_elements,
                    model_variants_are_multipart: blockstate.models.variants_are_multipart,
                    model_variants,
                    render_assets: Vec::new(),
                }
            })
            .collect()
    }

    fn cache_texture(&self, asset_path: &AssetPath, bytes: &[u8]) -> Result<PathBuf, AssetError> {
        let path = self
            .texture_cache_dir
            .join(&asset_path.namespace)
            .join(&asset_path.relative_asset_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, bytes)?;
        Ok(path)
    }

    fn read_language_bytes(&mut self, bytes: &[u8]) -> Result<(), AssetError> {
        let value = read_json_bytes(bytes)?;
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

    fn read_blockstate_bytes(
        &mut self,
        asset_path: AssetPath,
        bytes: &[u8],
    ) -> Result<(), AssetError> {
        let value = read_json_bytes(bytes)?;
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
                models: collect_blockstate_models(&value),
            },
        );
        Ok(())
    }

    fn read_model_bytes(&mut self, asset_path: AssetPath, bytes: &[u8]) -> Result<(), AssetError> {
        let value = read_json_bytes(bytes)?;
        let id = format!(
            "{}:{}",
            asset_path.namespace,
            without_extension(&asset_path.relative_asset_path)
        );
        self.models.insert(
            id,
            ModelAsset {
                parent: value
                    .get("parent")
                    .and_then(serde_json::Value::as_str)
                    .map(|parent| normalize_asset_reference(parent, &asset_path.namespace)),
                textures: collect_model_textures(&value),
                face_textures: collect_model_face_textures(&value),
                elements: collect_model_elements(&value),
            },
        );
        Ok(())
    }

    fn read_item_bytes(&mut self, asset_path: AssetPath, bytes: &[u8]) -> Result<(), AssetError> {
        let value = read_json_bytes(bytes)?;
        let id = format!(
            "{}:{}",
            asset_path.namespace,
            without_extension(&asset_path.relative_asset_path)
        );
        self.items.insert(
            id,
            ItemAsset {
                max_stack_size: explicit_max_stack_size(&value),
            },
        );
        Ok(())
    }

    fn block_model_variant_sample(
        &self,
        blockstate: &BlockstateAsset,
        variant: &BlockstateModelReference,
    ) -> BlockModelVariantSample {
        let model = Some(normalize_asset_reference(
            &variant.model,
            &blockstate.namespace,
        ));
        let resolved_model = model
            .as_ref()
            .and_then(|model_id| self.resolved_model_textures(model_id));
        let texture_path = resolved_model
            .as_ref()
            .and_then(|resolved| resolved.primary_texture_id())
            .and_then(|texture_id| self.textures.get(&texture_id).cloned());
        let face_texture_paths = resolved_model
            .as_ref()
            .and_then(|resolved| resolved.face_paths(&self.textures));
        let model_elements = resolved_model
            .as_ref()
            .map(|resolved| resolved.element_samples(&self.textures))
            .unwrap_or_default();
        BlockModelVariantSample {
            condition: variant.condition.clone(),
            model,
            x: variant.x,
            y: variant.y,
            uv_lock: variant.uv_lock,
            texture_path,
            face_texture_paths,
            model_elements,
        }
    }

    fn resolved_model_textures(&self, model_id: &str) -> Option<ResolvedModelTextures> {
        let mut visiting = BTreeSet::new();
        self.resolved_model_textures_inner(model_id, &mut visiting)
    }

    fn resolved_model_textures_inner(
        &self,
        model_id: &str,
        visiting: &mut BTreeSet<String>,
    ) -> Option<ResolvedModelTextures> {
        if !visiting.insert(model_id.to_string()) {
            return None;
        }
        let model = self.models.get(model_id)?;
        let mut resolved = model
            .parent
            .as_deref()
            .and_then(|parent| self.resolved_model_textures_inner(parent, visiting))
            .unwrap_or_default();
        for (name, texture) in &model.textures {
            let namespace = model_id
                .split_once(':')
                .map(|(namespace, _)| namespace)
                .unwrap_or("minecraft");
            resolved.textures.insert(
                name.clone(),
                normalize_texture_reference(texture, namespace),
            );
        }
        for (face, texture) in &model.face_textures {
            resolved.face_textures.insert(*face, texture.clone());
        }
        if !model.elements.is_empty() {
            resolved.elements = model.elements.clone();
        }
        visiting.remove(model_id);
        Some(resolved)
    }
}

pub(crate) fn scan_asset_entries_in_dir(
    collector: &mut AssetCollector,
    root: &Path,
    cancellation: &CancellationToken,
) -> Result<(), AssetError> {
    ensure_not_cancelled(cancellation)?;
    let mut files = Vec::new();
    collect_files(root, &mut files, cancellation)?;
    for file in files {
        ensure_not_cancelled(cancellation)?;
        let Ok(relative) = file.strip_prefix(root) else {
            continue;
        };
        let bytes = match fs::read(&file) {
            Ok(bytes) => bytes,
            Err(error) => {
                collector
                    .warnings
                    .push(format!("Skipped asset {}: {error}", file.display()));
                continue;
            }
        };
        collector.scan_asset_entry(relative, Some(file.clone()), &bytes);
    }
    Ok(())
}

pub(crate) fn collect_archives(
    root: &Path,
    cancellation: &CancellationToken,
) -> Result<Vec<PathBuf>, AssetError> {
    ensure_not_cancelled(cancellation)?;
    let mut archives = Vec::new();
    collect_archives_into(root, &mut archives, cancellation)?;
    archives.sort();
    Ok(archives)
}

fn collect_archives_into(
    root: &Path,
    archives: &mut Vec<PathBuf>,
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
            collect_archives_into(&path, archives, cancellation)?;
        } else if extension_is(&path, "jar") || extension_is(&path, "zip") {
            archives.push(path);
        }
    }
    Ok(())
}

fn collect_files(
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
            collect_files(&path, files, cancellation)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

pub(crate) fn texture_atlas_metadata(blocks: &[BlockAssetSample]) -> TextureAtlasMetadata {
    let mut seen = BTreeSet::new();
    let textures = blocks
        .iter()
        .flat_map(|block| {
            let mut paths = Vec::new();
            if let Some(path) = &block.texture_path {
                paths.push(path.clone());
            }
            if let Some(faces) = &block.face_texture_paths {
                for path in [
                    &faces.north,
                    &faces.south,
                    &faces.east,
                    &faces.west,
                    &faces.up,
                    &faces.down,
                ]
                .into_iter()
                .flatten()
                {
                    paths.push(path.clone());
                }
            }
            for element in &block.model_elements {
                for path in [
                    &element.face_texture_paths.north,
                    &element.face_texture_paths.south,
                    &element.face_texture_paths.east,
                    &element.face_texture_paths.west,
                    &element.face_texture_paths.up,
                    &element.face_texture_paths.down,
                ]
                .into_iter()
                .flatten()
                {
                    paths.push(path.clone());
                }
            }
            paths
                .into_iter()
                .filter_map(|path| {
                    let key = path.to_string_lossy().to_string();
                    seen.insert(key).then_some((block.identifier.clone(), path))
                })
                .collect::<Vec<_>>()
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

struct BlockstateAsset {
    identifier: String,
    namespace: String,
    models: BlockstateModelReferences,
}

struct ModelAsset {
    parent: Option<String>,
    textures: BTreeMap<String, String>,
    face_textures: BTreeMap<BlockFace, String>,
    elements: Vec<ModelElementAsset>,
}

#[derive(Debug, Clone)]
struct ModelElementAsset {
    from: [f32; 3],
    to: [f32; 3],
    rotation: Option<ModelElementRotationAsset>,
    face_textures: BTreeMap<BlockFace, String>,
    face_uvs: BTreeMap<BlockFace, [f32; 4]>,
}

#[derive(Debug, Clone)]
struct ModelElementRotationAsset {
    origin: [f32; 3],
    axis: String,
    angle: f32,
    rescale: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum BlockFace {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

#[derive(Debug, Clone, Default)]
struct ResolvedModelTextures {
    textures: BTreeMap<String, String>,
    face_textures: BTreeMap<BlockFace, String>,
    elements: Vec<ModelElementAsset>,
}

impl ResolvedModelTextures {
    fn primary_texture_id(&self) -> Option<String> {
        for face in [
            BlockFace::North,
            BlockFace::South,
            BlockFace::East,
            BlockFace::West,
            BlockFace::Up,
            BlockFace::Down,
        ] {
            if let Some(texture) = self.face_texture_id(face) {
                return Some(texture);
            }
        }
        self.textures
            .values()
            .find_map(|texture| self.resolve_texture_reference(texture))
    }

    fn face_paths(&self, textures: &BTreeMap<String, PathBuf>) -> Option<FaceTexturePaths> {
        let paths = FaceTexturePaths {
            north: self
                .face_texture_id(BlockFace::North)
                .and_then(|id| textures.get(&id).cloned()),
            south: self
                .face_texture_id(BlockFace::South)
                .and_then(|id| textures.get(&id).cloned()),
            east: self
                .face_texture_id(BlockFace::East)
                .and_then(|id| textures.get(&id).cloned()),
            west: self
                .face_texture_id(BlockFace::West)
                .and_then(|id| textures.get(&id).cloned()),
            up: self
                .face_texture_id(BlockFace::Up)
                .and_then(|id| textures.get(&id).cloned()),
            down: self
                .face_texture_id(BlockFace::Down)
                .and_then(|id| textures.get(&id).cloned()),
        };
        [
            &paths.north,
            &paths.south,
            &paths.east,
            &paths.west,
            &paths.up,
            &paths.down,
        ]
        .iter()
        .any(|path| path.is_some())
        .then_some(paths)
    }

    fn face_texture_id(&self, face: BlockFace) -> Option<String> {
        self.face_textures
            .get(&face)
            .and_then(|texture| self.resolve_texture_reference(texture))
    }

    fn element_samples(&self, textures: &BTreeMap<String, PathBuf>) -> Vec<ModelElementSample> {
        self.elements
            .iter()
            .map(|element| {
                let face_texture_paths = FaceTexturePaths {
                    north: self.element_face_path(element, BlockFace::North, textures),
                    south: self.element_face_path(element, BlockFace::South, textures),
                    east: self.element_face_path(element, BlockFace::East, textures),
                    west: self.element_face_path(element, BlockFace::West, textures),
                    up: self.element_face_path(element, BlockFace::Up, textures),
                    down: self.element_face_path(element, BlockFace::Down, textures),
                };
                ModelElementSample {
                    from: element.from,
                    to: element.to,
                    rotation: element.rotation.as_ref().map(|rotation| {
                        ModelElementRotationSample {
                            origin: rotation.origin,
                            axis: rotation.axis.clone(),
                            angle: rotation.angle,
                            rescale: rotation.rescale,
                        }
                    }),
                    face_texture_paths,
                    face_uvs: FaceUvs {
                        north: element.face_uvs.get(&BlockFace::North).copied(),
                        south: element.face_uvs.get(&BlockFace::South).copied(),
                        east: element.face_uvs.get(&BlockFace::East).copied(),
                        west: element.face_uvs.get(&BlockFace::West).copied(),
                        up: element.face_uvs.get(&BlockFace::Up).copied(),
                        down: element.face_uvs.get(&BlockFace::Down).copied(),
                    },
                }
            })
            .collect()
    }

    fn element_face_path(
        &self,
        element: &ModelElementAsset,
        face: BlockFace,
        textures: &BTreeMap<String, PathBuf>,
    ) -> Option<PathBuf> {
        element
            .face_textures
            .get(&face)
            .and_then(|texture| self.resolve_texture_reference(texture))
            .and_then(|id| textures.get(&id).cloned())
    }

    fn resolve_texture_reference(&self, texture: &str) -> Option<String> {
        let mut current = texture;
        let mut seen = BTreeSet::new();
        loop {
            if let Some(variable) = current.strip_prefix('#') {
                if !seen.insert(variable.to_string()) {
                    return None;
                }
                current = self.textures.get(variable)?;
                continue;
            }
            return Some(current.to_string());
        }
    }
}

struct ItemAsset {
    max_stack_size: Option<u32>,
}

struct AssetPath {
    namespace: String,
    kind: String,
    relative_asset_path: String,
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

fn read_json_bytes(bytes: &[u8]) -> Result<serde_json::Value, AssetError> {
    serde_json::from_reader(Cursor::new(bytes))
        .map_err(|error| AssetError::InvalidAssetIndex(error.to_string()))
}

fn collect_model_textures(value: &serde_json::Value) -> BTreeMap<String, String> {
    value
        .get("textures")
        .and_then(|textures| textures.as_object())
        .map(|textures| {
            textures
                .iter()
                .filter_map(|(name, texture)| Some((name.clone(), texture.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn collect_model_face_textures(value: &serde_json::Value) -> BTreeMap<BlockFace, String> {
    let mut faces = BTreeMap::new();
    let Some(elements) = value.get("elements").and_then(serde_json::Value::as_array) else {
        return faces;
    };
    for element in elements {
        let Some(face_object) = element.get("faces").and_then(serde_json::Value::as_object) else {
            continue;
        };
        for (face_name, face_value) in face_object {
            let Some(face) = parse_block_face(face_name) else {
                continue;
            };
            let Some(texture) = face_value
                .get("texture")
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            faces.entry(face).or_insert_with(|| texture.to_string());
        }
    }
    faces
}

fn collect_model_elements(value: &serde_json::Value) -> Vec<ModelElementAsset> {
    let Some(elements) = value.get("elements").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    elements
        .iter()
        .filter_map(|element| {
            let from = parse_model_vector(element.get("from")?)?;
            let to = parse_model_vector(element.get("to")?)?;
            if from == to {
                return None;
            }
            let face_object = element
                .get("faces")
                .and_then(serde_json::Value::as_object)?;
            let face_textures = face_object
                .iter()
                .filter_map(|(face_name, face_value)| {
                    let face = parse_block_face(face_name)?;
                    let texture = face_value.get("texture")?.as_str()?.to_string();
                    Some((face, texture))
                })
                .collect::<BTreeMap<_, _>>();
            if face_textures.is_empty() {
                return None;
            }
            let face_uvs = face_object
                .iter()
                .filter_map(|(face_name, face_value)| {
                    let face = parse_block_face(face_name)?;
                    let uv = parse_model_face_uv(face_value.get("uv")?)?;
                    Some((face, uv))
                })
                .collect::<BTreeMap<_, _>>();
            Some(ModelElementAsset {
                from,
                to,
                rotation: collect_model_element_rotation(element),
                face_textures,
                face_uvs,
            })
        })
        .collect()
}

fn collect_model_element_rotation(value: &serde_json::Value) -> Option<ModelElementRotationAsset> {
    let rotation = value.get("rotation")?;
    let axis = rotation.get("axis")?.as_str()?;
    if !matches!(axis, "x" | "y" | "z") {
        return None;
    }
    Some(ModelElementRotationAsset {
        origin: parse_model_vector(rotation.get("origin")?)?,
        axis: axis.to_string(),
        angle: rotation.get("angle")?.as_f64()? as f32,
        rescale: rotation
            .get("rescale")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
    })
}

fn parse_model_vector(value: &serde_json::Value) -> Option<[f32; 3]> {
    let array = value.as_array()?;
    let [x, y, z] = array.as_slice() else {
        return None;
    };
    Some([x.as_f64()? as f32, y.as_f64()? as f32, z.as_f64()? as f32])
}

fn parse_model_face_uv(value: &serde_json::Value) -> Option<[f32; 4]> {
    let array = value.as_array()?;
    let [u0, v0, u1, v1] = array.as_slice() else {
        return None;
    };
    Some([
        u0.as_f64()? as f32,
        v0.as_f64()? as f32,
        u1.as_f64()? as f32,
        v1.as_f64()? as f32,
    ])
}

fn parse_block_face(value: &str) -> Option<BlockFace> {
    match value {
        "north" => Some(BlockFace::North),
        "south" => Some(BlockFace::South),
        "east" => Some(BlockFace::East),
        "west" => Some(BlockFace::West),
        "up" => Some(BlockFace::Up),
        "down" => Some(BlockFace::Down),
        _ => None,
    }
}

fn explicit_max_stack_size(value: &serde_json::Value) -> Option<u32> {
    match value {
        serde_json::Value::Object(object) => {
            for key in ["minecraft:max_stack_size", "max_stack_size", "maxStackSize"] {
                if let Some(size) = object.get(key).and_then(serde_json::Value::as_u64) {
                    return u32::try_from(size).ok();
                }
            }
            object.values().find_map(explicit_max_stack_size)
        }
        serde_json::Value::Array(array) => array.iter().find_map(explicit_max_stack_size),
        _ => None,
    }
}

fn normalize_asset_reference(value: &str, fallback_namespace: &str) -> String {
    if value.contains(':') {
        value.to_string()
    } else {
        format!("{fallback_namespace}:{value}")
    }
}

fn normalize_texture_reference(value: &str, fallback_namespace: &str) -> String {
    if value.starts_with('#') {
        value.to_string()
    } else {
        normalize_asset_reference(value, fallback_namespace)
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

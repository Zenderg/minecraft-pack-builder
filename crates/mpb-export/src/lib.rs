//! Minecraft ecosystem export formats for finalized schemes.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use fastnbt::{to_bytes_with_opts, ByteArray, LongArray, SerOpts};
use flate2::write::GzEncoder;
use flate2::Compression;
use mpb_core::{Coordinate, Dimensions, Scheme, SchemeBlock};
use serde::Serialize;
use thiserror::Error;

const MINECRAFT_DATA_VERSION_1_20_1: i32 = 3465;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    Schem,
    Litematic,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Schem => "schem",
            Self::Litematic => "litematic",
        }
    }

    pub fn from_extension(value: &str) -> Option<Self> {
        match value.trim_start_matches('.').to_ascii_lowercase().as_str() {
            "schem" => Some(Self::Schem),
            "litematic" => Some(Self::Litematic),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportArtifact {
    pub path: PathBuf,
    pub format: ExportFormat,
    pub byte_len: u64,
    pub block_count: usize,
}

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("scheme dimensions {dimensions} are too large for {format}")]
    DimensionsTooLarge {
        format: &'static str,
        dimensions: Dimensions,
    },
    #[error("scheme volume {volume} is too large to export")]
    VolumeTooLarge { volume: i64 },
    #[error("failed to serialize {format} NBT: {source}")]
    Serialize {
        format: &'static str,
        #[source]
        source: fastnbt::error::Error,
    },
    #[error("failed to compress {format} export: {source}")]
    Compress {
        format: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write export file at {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl ExportError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DimensionsTooLarge { .. } => "export_dimensions_too_large",
            Self::VolumeTooLarge { .. } => "export_volume_too_large",
            Self::Serialize { .. } => "export_serialize_failed",
            Self::Compress { .. } => "export_compress_failed",
            Self::Write { .. } => "export_write_failed",
        }
    }
}

pub fn export_scheme_to_bytes(
    scheme: &Scheme,
    format: ExportFormat,
) -> Result<Vec<u8>, ExportError> {
    match format {
        ExportFormat::Schem => serialize_schem(scheme),
        ExportFormat::Litematic => serialize_litematic(scheme),
    }
}

pub fn write_scheme_export(
    scheme: &Scheme,
    format: ExportFormat,
    path: impl AsRef<Path>,
) -> Result<ExportArtifact, ExportError> {
    let bytes = export_scheme_to_bytes(scheme, format)?;
    let path = path.as_ref().to_path_buf();
    std::fs::write(&path, &bytes).map_err(|source| ExportError::Write {
        path: path.clone(),
        source,
    })?;
    Ok(ExportArtifact {
        path,
        format,
        byte_len: bytes.len() as u64,
        block_count: scheme.block_count(),
    })
}

fn serialize_schem(scheme: &Scheme) -> Result<Vec<u8>, ExportError> {
    let dimensions = scheme.dimensions();
    let width = checked_i16_dimension(dimensions.x, dimensions, "schem")?;
    let height = checked_i16_dimension(dimensions.y, dimensions, "schem")?;
    let length = checked_i16_dimension(dimensions.z, dimensions, "schem")?;
    let volume = checked_volume(dimensions)?;
    let palette = build_palette(scheme);
    let mut block_data = vec![0_i8; volume];

    for (coordinate, block) in scheme.blocks() {
        let key = block_state_key(block);
        let palette_index = *palette
            .entries
            .get(&key)
            .expect("palette contains every exported block");
        let index = block_index(*coordinate, dimensions);
        write_varint_i8(palette_index as i32, &mut block_data, index);
    }

    let root = SpongeSchematic {
        version: 3,
        data_version: MINECRAFT_DATA_VERSION_1_20_1,
        width,
        height,
        length,
        offset: [0, 0, 0],
        palette: palette.entries,
        palette_max: palette.len as i32,
        block_data: ByteArray::new(block_data),
        block_entities: Vec::<EmptyCompound>::new(),
        entities: Vec::<EmptyCompound>::new(),
        metadata: Metadata {
            name: scheme_name(scheme),
            minecraft_pack_builder_phase: 10,
        },
    };

    gzip_nbt("schem", &root)
}

fn serialize_litematic(scheme: &Scheme) -> Result<Vec<u8>, ExportError> {
    let dimensions = scheme.dimensions();
    checked_volume(dimensions)?;
    let palette = build_palette(scheme);
    let bits_per_block = bits_per_block(palette.len);
    let packed = pack_litematic_block_states(scheme, &palette, bits_per_block)?;
    let region_name = scheme_name(scheme);

    let mut regions = BTreeMap::new();
    regions.insert(
        region_name.clone(),
        LitematicRegion {
            position: Vec3 { x: 0, y: 0, z: 0 },
            size: Vec3 {
                x: dimensions.x,
                y: dimensions.y,
                z: dimensions.z,
            },
            block_state_palette: palette
                .ordered
                .iter()
                .map(|state| LitematicPaletteEntry {
                    name: state.name.clone(),
                    properties: state.properties.clone(),
                })
                .collect(),
            block_states: LongArray::new(packed),
            tile_entities: Vec::<EmptyCompound>::new(),
            entities: Vec::<EmptyCompound>::new(),
            pending_block_ticks: Vec::<EmptyCompound>::new(),
            pending_fluid_ticks: Vec::<EmptyCompound>::new(),
        },
    );

    let root = LitematicRoot {
        version: 6,
        sub_version: 1,
        minecraft_data_version: MINECRAFT_DATA_VERSION_1_20_1,
        metadata: LitematicMetadata {
            name: region_name,
            author: "Minecraft Pack Builder".to_string(),
            description: "Exported by Minecraft Pack Builder phase 10".to_string(),
            region_count: 1,
            total_blocks: scheme.block_count() as i32,
            total_volume: checked_volume(dimensions)? as i32,
            enclosing_size: Vec3 {
                x: dimensions.x,
                y: dimensions.y,
                z: dimensions.z,
            },
            time_created: 0,
            time_modified: 0,
        },
        regions,
    };

    gzip_nbt("litematic", &root)
}

fn gzip_nbt<T: Serialize>(format: &'static str, root: &T) -> Result<Vec<u8>, ExportError> {
    let nbt = to_bytes_with_opts(root, SerOpts::new())
        .map_err(|source| ExportError::Serialize { format, source })?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&nbt)
        .map_err(|source| ExportError::Compress { format, source })?;
    encoder
        .finish()
        .map_err(|source| ExportError::Compress { format, source })
}

fn checked_i16_dimension(
    value: i32,
    dimensions: Dimensions,
    format: &'static str,
) -> Result<i16, ExportError> {
    i16::try_from(value).map_err(|_| ExportError::DimensionsTooLarge { format, dimensions })
}

fn checked_volume(dimensions: Dimensions) -> Result<usize, ExportError> {
    let volume = i64::from(dimensions.x)
        .checked_mul(i64::from(dimensions.y))
        .and_then(|value| value.checked_mul(i64::from(dimensions.z)))
        .ok_or(ExportError::VolumeTooLarge { volume: i64::MAX })?;
    usize::try_from(volume).map_err(|_| ExportError::VolumeTooLarge { volume })
}

fn scheme_name(scheme: &Scheme) -> String {
    scheme.name().to_string()
}

#[derive(Debug, Clone)]
struct Palette {
    entries: BTreeMap<String, i32>,
    ordered: Vec<PaletteState>,
    len: usize,
}

#[derive(Debug, Clone)]
struct PaletteState {
    key: String,
    name: String,
    properties: BTreeMap<String, String>,
}

fn build_palette(scheme: &Scheme) -> Palette {
    let mut states = BTreeMap::new();
    states.insert(
        "minecraft:air".to_string(),
        PaletteState {
            key: "minecraft:air".to_string(),
            name: "minecraft:air".to_string(),
            properties: BTreeMap::new(),
        },
    );

    for (_, block) in scheme.blocks() {
        let key = block_state_key(block);
        states.entry(key.clone()).or_insert_with(|| PaletteState {
            key,
            name: block.block_id.clone(),
            properties: block.states.clone(),
        });
    }

    let mut ordered = Vec::with_capacity(states.len());
    if let Some(air) = states.remove("minecraft:air") {
        ordered.push(air);
    }
    ordered.extend(states.into_values());

    let entries = ordered
        .iter()
        .enumerate()
        .map(|(index, state)| (state.key.clone(), index as i32))
        .collect::<BTreeMap<_, _>>();

    Palette {
        len: ordered.len(),
        entries,
        ordered,
    }
}

fn block_state_key(block: &SchemeBlock) -> String {
    if block.states.is_empty() {
        return block.block_id.clone();
    }

    let states = block
        .states
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("{}[{states}]", block.block_id)
}

fn block_index(coordinate: Coordinate, dimensions: Dimensions) -> usize {
    (coordinate.y * dimensions.z * dimensions.x + coordinate.z * dimensions.x + coordinate.x)
        as usize
}

fn write_varint_i8(value: i32, target: &mut Vec<i8>, index: usize) {
    let mut encoded = Vec::new();
    let mut value = value as u32;
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        encoded.push(byte as i8);
        if value == 0 {
            break;
        }
    }

    if encoded.len() == 1 {
        target[index] = encoded[0];
        return;
    }

    target.splice(index..=index, encoded);
}

fn bits_per_block(palette_len: usize) -> usize {
    let max_index = palette_len.saturating_sub(1).max(1);
    let bits = usize::BITS as usize - max_index.leading_zeros() as usize;
    bits.max(2)
}

fn pack_litematic_block_states(
    scheme: &Scheme,
    palette: &Palette,
    bits_per_block: usize,
) -> Result<Vec<i64>, ExportError> {
    let dimensions = scheme.dimensions();
    let volume = checked_volume(dimensions)?;
    let long_count = (volume * bits_per_block + 63) / 64;
    let mut longs = vec![0_u64; long_count];

    for (coordinate, block) in scheme.blocks() {
        let key = block_state_key(block);
        let palette_index = *palette
            .entries
            .get(&key)
            .expect("palette contains every exported block") as u64;
        let block_index = block_index(*coordinate, dimensions);
        let bit_index = block_index * bits_per_block;
        let long_index = bit_index / 64;
        let bit_offset = bit_index % 64;
        longs[long_index] |= palette_index << bit_offset;
        if bit_offset + bits_per_block > 64 {
            longs[long_index + 1] |= palette_index >> (64 - bit_offset);
        }
    }

    Ok(longs.into_iter().map(|value| value as i64).collect())
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SpongeSchematic {
    version: i32,
    data_version: i32,
    width: i16,
    height: i16,
    length: i16,
    offset: [i32; 3],
    palette: BTreeMap<String, i32>,
    palette_max: i32,
    block_data: ByteArray,
    block_entities: Vec<EmptyCompound>,
    entities: Vec<EmptyCompound>,
    metadata: Metadata,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct Metadata {
    name: String,
    minecraft_pack_builder_phase: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct LitematicRoot {
    version: i32,
    sub_version: i32,
    minecraft_data_version: i32,
    metadata: LitematicMetadata,
    regions: BTreeMap<String, LitematicRegion>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct LitematicMetadata {
    name: String,
    author: String,
    description: String,
    region_count: i32,
    total_blocks: i32,
    total_volume: i32,
    enclosing_size: Vec3,
    time_created: i64,
    time_modified: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct LitematicRegion {
    position: Vec3,
    size: Vec3,
    block_state_palette: Vec<LitematicPaletteEntry>,
    block_states: LongArray,
    tile_entities: Vec<EmptyCompound>,
    entities: Vec<EmptyCompound>,
    pending_block_ticks: Vec<EmptyCompound>,
    pending_fluid_ticks: Vec<EmptyCompound>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct LitematicPaletteEntry {
    name: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    properties: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct Vec3 {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Serialize)]
struct EmptyCompound {}

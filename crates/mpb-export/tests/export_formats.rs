use std::collections::HashMap;
use std::io::Read;

use fastnbt::{from_bytes, Value};
use flate2::read::GzDecoder;
use mpb_core::{
    BlockPlacement, BlockRegistry, Coordinate, Dimensions, Scheme, SchemeOperation, StageRef,
};
use mpb_export::{export_scheme_to_bytes, ExportFormat};

fn demo_scheme() -> Scheme {
    let registry = BlockRegistry::synthetic_fixture();
    let mut scheme = Scheme::new(
        "Phase 10 Demo",
        Dimensions::new(4, 3, 4).expect("dimensions"),
    );
    let foundation = scheme.add_stage("Foundation").expect("stage");
    let machinery = scheme.add_stage("Machinery").expect("stage");

    for placement in [
        BlockPlacement::new(
            Coordinate::new(0, 0, 0),
            "minecraft:stone_bricks",
            [("cracked", "false")],
            StageRef::Stage(foundation),
        ),
        BlockPlacement::new(
            Coordinate::new(1, 0, 0),
            "thermal:machine_frame",
            [("tier", "basic")],
            StageRef::Stage(machinery),
        ),
        BlockPlacement::new(
            Coordinate::new(2, 0, 0),
            "create:andesite_casing",
            [],
            StageRef::Unassigned,
        ),
    ] {
        scheme
            .apply(&registry, SchemeOperation::Place(placement))
            .expect("place block");
    }

    scheme
}

#[test]
fn exports_gzip_sponge_schematic_with_palette_and_all_blocks() {
    let bytes = export_scheme_to_bytes(&demo_scheme(), ExportFormat::Schem).expect("export");
    assert_eq!(&bytes[..2], &[0x1f, 0x8b]);

    let root = decode_gzip_nbt(&bytes);
    assert_eq!(root["Version"], Value::Int(3));
    assert_eq!(root["Width"], Value::Short(4));
    assert_eq!(root["Height"], Value::Short(3));
    assert_eq!(root["Length"], Value::Short(4));

    let palette = compound(&root["Palette"]);
    assert!(palette.contains_key("minecraft:stone_bricks[cracked=false]"));
    assert!(palette.contains_key("thermal:machine_frame[tier=basic]"));
    assert!(palette.contains_key("create:andesite_casing"));

    let block_data = byte_array(&root["BlockData"]);
    assert_eq!(block_data.len(), 4 * 3 * 4);
    assert_eq!(block_data.iter().filter(|value| **value != 0).count(), 3);
}

#[test]
fn exports_gzip_litematic_region_with_palette_and_packed_block_states() {
    let bytes = export_scheme_to_bytes(&demo_scheme(), ExportFormat::Litematic).expect("export");
    assert_eq!(&bytes[..2], &[0x1f, 0x8b]);

    let root = decode_gzip_nbt(&bytes);
    assert_eq!(root["Version"], Value::Int(6));
    assert_eq!(root["SubVersion"], Value::Int(1));
    assert_eq!(root["MinecraftDataVersion"], Value::Int(3465));

    let regions = compound(&root["Regions"]);
    let region = compound(&regions["Phase 10 Demo"]);
    let size = compound(&region["Size"]);
    assert_eq!(size["x"], Value::Int(4));
    assert_eq!(size["y"], Value::Int(3));
    assert_eq!(size["z"], Value::Int(4));

    let palette = list(&region["BlockStatePalette"]);
    let names = palette
        .iter()
        .map(|entry| compound(entry)["Name"].as_str().expect("palette name"))
        .collect::<Vec<_>>();
    assert_eq!(names[0], "minecraft:air");
    assert!(names.contains(&"minecraft:stone_bricks"));
    assert!(names.contains(&"thermal:machine_frame"));
    assert!(names.contains(&"create:andesite_casing"));

    let block_states = long_array(&region["BlockStates"]);
    assert!(!block_states.is_empty());
    assert!(block_states.iter().any(|value| *value != 0));
}

fn decode_gzip_nbt(bytes: &[u8]) -> HashMap<String, Value> {
    let mut decoder = GzDecoder::new(bytes);
    let mut nbt = Vec::new();
    decoder.read_to_end(&mut nbt).expect("decompress");
    from_bytes::<HashMap<String, Value>>(&nbt).expect("decode nbt")
}

fn compound(value: &Value) -> &HashMap<String, Value> {
    match value {
        Value::Compound(value) => value,
        value => panic!("expected compound, got {value:?}"),
    }
}

fn list(value: &Value) -> &[Value] {
    match value {
        Value::List(value) => value,
        value => panic!("expected list, got {value:?}"),
    }
}

fn byte_array(value: &Value) -> &[i8] {
    match value {
        Value::ByteArray(value) => value,
        value => panic!("expected byte array, got {value:?}"),
    }
}

fn long_array(value: &Value) -> &[i64] {
    match value {
        Value::LongArray(value) => value,
        value => panic!("expected long array, got {value:?}"),
    }
}

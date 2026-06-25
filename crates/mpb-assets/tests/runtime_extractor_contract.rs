use std::io::Read;

use zip::ZipArchive;

#[test]
fn embedded_runtime_extractors_emit_render_asset_contract() {
    assert_extractor_contract(
        include_str!("../src/runtime_extractor_jar.hex"),
        "com/mpb/runtime/NeoForgeRuntimeExtractor.class",
        "META-INF/neoforge.mods.toml",
    );
    assert_extractor_contract(
        include_str!("../src/runtime_extractor_forge_jar.hex"),
        "com/mpb/runtime/ForgeRuntimeExtractor.class",
        "META-INF/mods.toml",
    );
    assert_extractor_contract(
        include_str!("../src/runtime_extractor_fabric_jar.hex"),
        "com/mpb/runtime/FabricRuntimeExtractor.class",
        "fabric.mod.json",
    );
}

fn assert_extractor_contract(hex: &str, entrypoint: &str, metadata: &str) {
    let bytes = decode_hex(hex);
    let mut archive = ZipArchive::new(std::io::Cursor::new(bytes)).expect("extractor jar");
    assert!(archive.by_name(metadata).is_ok(), "{metadata} missing");
    assert!(archive.by_name(entrypoint).is_ok(), "{entrypoint} missing");
    assert!(
        archive
            .by_name("com/mpb/runtime/RuntimeDumper.class")
            .is_ok(),
        "RuntimeDumper missing"
    );
    assert!(
        archive
            .by_name("net/minecraftforge/common/MinecraftForge.class")
            .is_err(),
        "Forge compile stubs must not be packaged"
    );
    assert!(
        archive
            .by_name("net/neoforged/neoforge/common/NeoForge.class")
            .is_err(),
        "NeoForge compile stubs must not be packaged"
    );
    assert!(
        archive
            .by_name("net/fabricmc/api/DedicatedServerModInitializer.class")
            .is_err(),
        "Fabric compile stubs must not be packaged"
    );

    let mut dumper = archive
        .by_name("com/mpb/runtime/RuntimeDumper.class")
        .expect("RuntimeDumper class");
    let mut class_bytes = Vec::new();
    dumper
        .read_to_end(&mut class_bytes)
        .expect("RuntimeDumper bytes");
    let class_text = String::from_utf8_lossy(&class_bytes);
    assert!(class_text.contains("renderAssets"));
    assert!(class_text.contains("minecraft-runtime-shape"));
}

fn decode_hex(value: &str) -> Vec<u8> {
    let digits = value
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    digits
        .chunks(2)
        .filter_map(|pair| {
            let [high, low] = pair else {
                return None;
            };
            Some((hex_digit(*high) << 4) | hex_digit(*low))
        })
        .collect()
}

fn hex_digit(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        b'A'..=b'F' => value - b'A' + 10,
        _ => 0,
    }
}

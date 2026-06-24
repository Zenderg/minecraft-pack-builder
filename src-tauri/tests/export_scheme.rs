use app_tauri_lib::{write_stored_scheme_export, write_stored_scheme_export_with_diagnostics};
use mpb_core::{BlockPlacement, BlockRegistry, Coordinate, SchemeOperation, StageRef};
use mpb_export::ExportFormat;
use mpb_storage::{
    ImportStatus, LibraryDatabase, LibraryRepository, NewImportedModpack, NewScheme,
};
use tempfile::tempdir;

#[test]
fn writes_stored_scheme_exports_for_desktop_command() {
    let temp = tempdir().expect("temp dir");
    let database_path = create_exportable_scheme(temp.path());
    let schem_path = temp.path().join("stored-scheme.schem");
    let litematic_path = temp.path().join("stored-scheme.litematic");

    let schem = write_stored_scheme_export(&database_path, 1, ExportFormat::Schem, &schem_path)
        .expect("write schem");
    let litematic =
        write_stored_scheme_export(&database_path, 1, ExportFormat::Litematic, &litematic_path)
            .expect("write litematic");

    assert_eq!(schem.path, schem_path);
    assert_eq!(litematic.path, litematic_path);
    assert_eq!(schem.block_count, 1);
    assert_eq!(litematic.block_count, 1);
    assert_eq!(
        &std::fs::read(&schem.path).expect("schem bytes")[..2],
        &[0x1f, 0x8b]
    );
    assert_eq!(
        &std::fs::read(&litematic.path).expect("litematic bytes")[..2],
        &[0x1f, 0x8b]
    );
}

#[test]
fn writes_export_diagnostic_reports_for_success_and_failure() {
    let temp = tempdir().expect("temp dir");
    let database_path = create_exportable_scheme(temp.path());
    let diagnostics_dir = temp.path().join("diagnostics");
    let export_path = temp.path().join("stored-scheme.schem");

    let exported = write_stored_scheme_export_with_diagnostics(
        &database_path,
        1,
        ExportFormat::Schem,
        &export_path,
        &diagnostics_dir,
    )
    .expect("write export with diagnostics");
    let success_json =
        std::fs::read_to_string(&exported.diagnostic.path).expect("read success diagnostic");

    assert_eq!(exported.artifact.path, export_path);
    assert!(exported
        .diagnostic
        .path
        .ends_with("export-scheme-1-schem.json"));
    assert!(success_json.contains("\"status\": \"success\""));
    assert!(success_json.contains("\"operation\": \"export\""));
    assert!(success_json.contains("\"recoveryMessage\": null"));

    let blocked_destination = temp.path().join("missing").join("blocked.schem");
    let failed = write_stored_scheme_export_with_diagnostics(
        &database_path,
        1,
        ExportFormat::Schem,
        &blocked_destination,
        &diagnostics_dir,
    )
    .expect_err("missing parent directory should fail");
    let failure_json =
        std::fs::read_to_string(&failed.diagnostic_path).expect("read failure diagnostic");

    assert!(failed.message.contains("Could not export scheme"));
    assert!(failure_json.contains("\"status\": \"failed\""));
    assert!(failure_json.contains("\"recoveryMessage\""));
    assert!(failure_json.contains("Choose another destination"));
}

fn create_exportable_scheme(root: &std::path::Path) -> std::path::PathBuf {
    let database_path = root.join("library.sqlite3");
    let database = LibraryDatabase::open(&database_path).expect("open database");
    let repository = LibraryRepository::new(database);
    let modpack = repository
        .create_imported_modpack(NewImportedModpack {
            local_name: "Stored Pack".to_string(),
            source_slug: None,
            source_url: None,
            version_name: "1.0.0".to_string(),
            minecraft_version: Some("1.20.1".to_string()),
            loader: Some("Forge".to_string()),
            cache_dir: None,
            import_status: ImportStatus::Imported,
        })
        .expect("create modpack");
    let record = repository
        .create_scheme(NewScheme {
            modpack_id: modpack.id,
            name: "Stored Scheme".to_string(),
            size_x: 4,
            size_y: 4,
            size_z: 4,
        })
        .expect("create scheme");
    let mut stored = repository.load_scheme(record.id).expect("load scheme");
    stored
        .scheme
        .apply(
            &BlockRegistry::synthetic_fixture(),
            SchemeOperation::Place(BlockPlacement::new(
                Coordinate::new(0, 0, 0),
                "minecraft:stone_bricks",
                [("cracked", "false")],
                StageRef::Unassigned,
            )),
        )
        .expect("place block");
    repository
        .save_scheme(record.id, &stored.scheme)
        .expect("save scheme");
    database_path
}

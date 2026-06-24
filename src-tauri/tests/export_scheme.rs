use app_tauri_lib::{write_demo_scheme_export, write_demo_scheme_export_with_diagnostics};
use mpb_export::ExportFormat;
use tempfile::tempdir;

#[test]
fn writes_phase_10_demo_exports_for_desktop_command() {
    let temp = tempdir().expect("temp dir");
    let schem_path = temp.path().join("starter-factory.schem");
    let litematic_path = temp.path().join("starter-factory.litematic");

    let schem =
        write_demo_scheme_export(10, ExportFormat::Schem, &schem_path).expect("write schem");
    let litematic = write_demo_scheme_export(10, ExportFormat::Litematic, &litematic_path)
        .expect("write litematic");

    assert_eq!(schem.path, schem_path);
    assert_eq!(litematic.path, litematic_path);
    assert_eq!(schem.block_count, 9);
    assert_eq!(litematic.block_count, 9);
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
    let diagnostics_dir = temp.path().join("diagnostics");
    let export_path = temp.path().join("starter-factory.schem");

    let exported = write_demo_scheme_export_with_diagnostics(
        10,
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
        .ends_with("export-scheme-10-schem.json"));
    assert!(success_json.contains("\"status\": \"success\""));
    assert!(success_json.contains("\"operation\": \"export\""));
    assert!(success_json.contains("\"recoveryMessage\": null"));

    let blocked_destination = temp.path().join("missing").join("blocked.schem");
    let failed = write_demo_scheme_export_with_diagnostics(
        10,
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

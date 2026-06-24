use app_tauri_lib::write_demo_scheme_export;
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

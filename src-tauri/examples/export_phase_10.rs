use std::path::PathBuf;

use app_tauri_lib::write_demo_scheme_export;
use mpb_export::ExportFormat;

fn main() {
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("docs")
        .join("validation");
    std::fs::create_dir_all(&output_dir).expect("validation dir");

    let schem = write_demo_scheme_export(
        10,
        ExportFormat::Schem,
        output_dir.join("phase-10-starter-factory.schem"),
    )
    .expect("write schem");
    let litematic = write_demo_scheme_export(
        10,
        ExportFormat::Litematic,
        output_dir.join("phase-10-starter-factory.litematic"),
    )
    .expect("write litematic");

    println!("{}", schem.path.display());
    println!("{}", litematic.path.display());
}

use std::fs;

use app_tauri_lib::write_domain_demo_report;
use tempfile::tempdir;

#[test]
fn writes_phase_4_domain_demo_report_to_diagnostics_folder() {
    let temp = tempdir().expect("temp dir");

    let artifact = write_domain_demo_report(temp.path()).expect("write report");
    let json = fs::read_to_string(&artifact.path).expect("read report");

    assert!(artifact.path.ends_with("phase-4-domain-demo-report.json"));
    assert_eq!(artifact.report.summary.block_count, 6);
    assert!(json.contains("\"rejectedActions\""));
    assert!(json.contains("\"unknown_block\""));
}

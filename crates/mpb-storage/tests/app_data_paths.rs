use mpb_storage::{ensure_app_data_dirs, AppDataPaths};

#[test]
fn creates_app_data_and_diagnostics_directories() {
    let temp_dir = tempfile::tempdir().expect("temporary directory");
    let paths = ensure_app_data_dirs(temp_dir.path()).expect("app data paths");

    assert!(paths.app_data_dir.exists());
    assert!(paths.diagnostics_dir.exists());
    assert!(paths.diagnostics_dir.ends_with("diagnostics"));
}

#[test]
fn app_data_paths_are_serializable_for_frontend_commands() {
    let paths = AppDataPaths {
        app_data_dir: "/tmp/mpb".into(),
        diagnostics_dir: "/tmp/mpb/diagnostics".into(),
    };

    let json = serde_json::to_value(paths).expect("serializable paths");

    assert_eq!(json["appDataDir"], "/tmp/mpb");
    assert_eq!(json["diagnosticsDir"], "/tmp/mpb/diagnostics");
}

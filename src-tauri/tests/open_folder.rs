use app_tauri_lib::open_folder_command_for_platform;

#[test]
fn chooses_native_folder_open_command_for_current_platform() {
    let command = open_folder_command_for_platform();

    if cfg!(target_os = "macos") {
        assert_eq!(command, "open");
    } else if cfg!(target_os = "windows") {
        assert_eq!(command, "explorer");
    } else {
        assert_eq!(command, "xdg-open");
    }
}

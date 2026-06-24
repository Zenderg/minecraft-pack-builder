use app_tauri_lib::{record_prism_instances_for_background_sync, runtime_prerequisites_present};
use mpb_assets::{PrismInstanceDescriptor, PrismInstanceStatus as AssetPrismInstanceStatus};
use mpb_storage::{LibraryDatabase, LibraryRepository, PrismInstanceStatus};
use tempfile::tempdir;

#[test]
fn registering_prism_instances_for_startup_does_not_build_registry_reports() {
    let temp = tempdir().expect("temp dir");
    let database = LibraryDatabase::open(temp.path().join("library.sqlite3")).expect("database");
    let repository = LibraryRepository::new(database);
    let instance = PrismInstanceDescriptor {
        instance_id: "heavy-pack".to_string(),
        display_name: "Heavy Pack".to_string(),
        instance_path: temp.path().join("PrismLauncher/instances/heavy-pack"),
        minecraft_dir: temp
            .path()
            .join("PrismLauncher/instances/heavy-pack/.minecraft"),
        minecraft_version: Some("1.20.1".to_string()),
        loader: Some("Forge".to_string()),
        loader_version: Some("47.4.20".to_string()),
        identity_fingerprint: "heavy-pack-identity".to_string(),
        content_fingerprint: "heavy-pack-content".to_string(),
        status: AssetPrismInstanceStatus::Pending,
        status_message: None,
    };

    record_prism_instances_for_background_sync(&repository, &[instance], &[])
        .expect("record instances");

    let stored = repository.list_prism_instances().expect("stored instances");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].status, PrismInstanceStatus::Indexing);
    assert_eq!(
        stored[0].status_message.as_deref(),
        Some("Waiting for background PrismLauncher indexing.")
    );
    assert!(!temp.path().join("diagnostics").exists());
}

#[test]
fn detects_forge_runtime_prerequisites_after_prism_generates_mappings() {
    let temp = tempdir().expect("temp dir");
    let instance = prism_instance(temp.path(), "forge-pack", "Forge", "47.4.20", "1.20.1");
    let meta_dir = temp.path().join("PrismLauncher/meta/net.minecraftforge");
    std::fs::create_dir_all(&meta_dir).expect("meta dir");
    std::fs::write(
        meta_dir.join("47.4.20.json"),
        r#"{ "minecraftArguments": "--launchTarget forgeclient --fml.mcpVersion 20230612.114412" }"#,
    )
    .expect("forge meta");
    let mappings = temp.path().join(
        "PrismLauncher/libraries/net/minecraft/client/1.20.1-20230612.114412/client-1.20.1-20230612.114412-mappings.txt",
    );
    std::fs::create_dir_all(mappings.parent().expect("mappings parent")).expect("mappings dir");
    std::fs::write(&mappings, "tiny mappings").expect("mappings");

    assert!(runtime_prerequisites_present(&instance));
}

#[test]
fn detects_fabric_runtime_prerequisites_when_server_jar_is_local() {
    let temp = tempdir().expect("temp dir");
    let instance = prism_instance(temp.path(), "fabric-pack", "Fabric", "0.19.3", "1.21.1");
    let server_jar = temp
        .path()
        .join("PrismLauncher/libraries/com/mojang/minecraft/1.21.1/minecraft-1.21.1-server.jar");
    std::fs::create_dir_all(server_jar.parent().expect("server jar parent"))
        .expect("server jar dir");
    std::fs::write(&server_jar, "server jar").expect("server jar");

    assert!(runtime_prerequisites_present(&instance));
}

fn prism_instance(
    root: &std::path::Path,
    instance_id: &str,
    loader: &str,
    loader_version: &str,
    minecraft_version: &str,
) -> PrismInstanceDescriptor {
    PrismInstanceDescriptor {
        instance_id: instance_id.to_string(),
        display_name: instance_id.to_string(),
        instance_path: root
            .join("PrismLauncher")
            .join("instances")
            .join(instance_id),
        minecraft_dir: root
            .join("PrismLauncher")
            .join("instances")
            .join(instance_id)
            .join(".minecraft"),
        minecraft_version: Some(minecraft_version.to_string()),
        loader: Some(loader.to_string()),
        loader_version: Some(loader_version.to_string()),
        identity_fingerprint: format!("{instance_id}-identity"),
        content_fingerprint: format!("{instance_id}-content"),
        status: AssetPrismInstanceStatus::Pending,
        status_message: None,
    }
}

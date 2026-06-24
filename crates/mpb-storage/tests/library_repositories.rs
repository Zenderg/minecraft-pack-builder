use std::path::PathBuf;

use mpb_storage::{
    LibraryDatabase, LibraryRepository, NewPrismInstance, NewScheme, PrismInstanceStatus,
};
use tempfile::tempdir;

fn open_test_repository() -> LibraryRepository {
    let database = LibraryDatabase::open_in_memory().expect("open database");
    LibraryRepository::new(database)
}

fn prism_instance(instance_id: &str, status: PrismInstanceStatus) -> NewPrismInstance {
    NewPrismInstance {
        instance_id: instance_id.to_string(),
        display_name: format!("{instance_id} Display"),
        instance_path: PathBuf::from(format!("/PrismLauncher/instances/{instance_id}")),
        minecraft_dir: PathBuf::from(format!("/PrismLauncher/instances/{instance_id}/.minecraft")),
        minecraft_version: Some("1.21.1".to_string()),
        loader: Some("NeoForge".to_string()),
        loader_version: Some("21.1.233".to_string()),
        identity_fingerprint: format!("identity-{instance_id}"),
        content_fingerprint: format!("content-{instance_id}"),
        status,
        status_message: None,
    }
}

#[test]
fn migrations_create_prism_library_tables_and_remove_old_import_tables() {
    let temp = tempdir().expect("temp dir");
    let db_path = temp.path().join("library.sqlite3");
    let raw = rusqlite::Connection::open(&db_path).expect("open raw database");
    raw.execute_batch(
        "CREATE TABLE schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE imported_modpacks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            local_name TEXT NOT NULL UNIQUE
        );
        INSERT INTO imported_modpacks (local_name) VALUES ('Old AOC');
        INSERT INTO schema_migrations (version) VALUES (4);",
    )
    .expect("seed old schema");
    drop(raw);

    let database = LibraryDatabase::open(&db_path).expect("open migrated database");
    let tables = database.table_names().expect("table names");

    for expected in [
        "schema_migrations",
        "settings_metadata",
        "prism_instances",
        "asset_indexes",
        "schemes",
        "scheme_dimensions",
        "construction_stages",
        "scheme_documents",
    ] {
        assert!(
            tables.contains(&expected.to_string()),
            "missing table {expected}"
        );
    }
    assert!(!tables.contains(&"imported_modpacks".to_string()));
    assert!(!tables.contains(&"import_status".to_string()));
}

#[test]
fn prism_root_setting_round_trips() {
    let repository = open_test_repository();
    let root = PathBuf::from("/Users/test/Library/Application Support/PrismLauncher");

    repository
        .set_prism_root(Some(root.clone()))
        .expect("set root");

    assert_eq!(repository.get_prism_root().expect("get root"), Some(root));

    repository.set_prism_root(None).expect("clear root");
    assert_eq!(repository.get_prism_root().expect("get root"), None);
}

#[test]
fn upserting_prism_instances_keeps_stable_database_id_for_exact_identity_match() {
    let repository = open_test_repository();
    let first = repository
        .upsert_prism_instance(prism_instance("aoc", PrismInstanceStatus::Ready))
        .expect("insert instance");
    let mut moved = prism_instance("aoc-renamed-folder", PrismInstanceStatus::Ready);
    moved.identity_fingerprint = first.identity_fingerprint.clone();
    moved.instance_path = PathBuf::from("/Moved/instances/aoc-renamed-folder");

    let second = repository
        .upsert_prism_instance(moved)
        .expect("update instance");

    assert_eq!(second.id, first.id);
    assert_eq!(second.instance_id, "aoc-renamed-folder");
    assert_eq!(
        second.instance_path,
        PathBuf::from("/Moved/instances/aoc-renamed-folder")
    );
}

#[test]
fn prism_instance_status_can_be_updated_after_indexing() {
    let repository = open_test_repository();
    let instance = repository
        .upsert_prism_instance(prism_instance("aoc", PrismInstanceStatus::Indexing))
        .expect("insert instance");

    repository
        .update_prism_instance_status(
            instance.id,
            PrismInstanceStatus::Ready,
            Some("Indexed 42 blocks."),
        )
        .expect("update status");

    let stored = repository
        .get_prism_instance_by_identity_fingerprint("identity-aoc")
        .expect("lookup by fingerprint")
        .expect("stored instance");
    assert_eq!(stored.id, instance.id);
    assert_eq!(stored.status, PrismInstanceStatus::Ready);
    assert_eq!(stored.status_message.as_deref(), Some("Indexed 42 blocks."));
}

#[test]
fn prism_instance_can_be_relinked_to_confirmed_identity() {
    let repository = open_test_repository();
    let instance = repository
        .upsert_prism_instance(prism_instance("old-aoc", PrismInstanceStatus::Missing))
        .expect("insert old instance");
    let mut replacement = prism_instance("new-aoc", PrismInstanceStatus::Indexing);
    replacement.identity_fingerprint = "identity-new-aoc".to_string();
    replacement.instance_path = PathBuf::from("/Moved/instances/new-aoc");

    let relinked = repository
        .relink_prism_instance(instance.id, replacement)
        .expect("relink instance");

    assert_eq!(relinked.id, instance.id);
    assert_eq!(relinked.instance_id, "new-aoc");
    assert_eq!(relinked.identity_fingerprint, "identity-new-aoc");
    assert_eq!(relinked.status, PrismInstanceStatus::Indexing);
    assert_eq!(
        relinked.instance_path,
        PathBuf::from("/Moved/instances/new-aoc")
    );
}

#[test]
fn missing_prism_instances_keep_schemes_but_block_new_scheme_creation() {
    let repository = open_test_repository();
    let instance = repository
        .upsert_prism_instance(prism_instance("aoc", PrismInstanceStatus::Ready))
        .expect("insert instance");
    let scheme = repository
        .create_scheme(NewScheme {
            prism_instance_id: instance.id,
            name: "Starter Factory".to_string(),
            size_x: 64,
            size_y: 48,
            size_z: 32,
        })
        .expect("create scheme");

    repository
        .mark_prism_instances_missing_except(&[])
        .expect("mark missing");

    let library = repository.list_library().expect("list library");
    assert_eq!(library[0].status, PrismInstanceStatus::Missing);
    assert_eq!(library[0].schemes.len(), 1);
    assert_eq!(library[0].schemes[0].id, scheme.id);

    let result = repository.create_scheme(NewScheme {
        prism_instance_id: instance.id,
        name: "Blocked".to_string(),
        size_x: 16,
        size_y: 16,
        size_z: 16,
    });
    assert!(result.is_err(), "missing instance must reject new schemes");
}

#[test]
fn non_ready_instances_reject_new_schemes() {
    let repository = open_test_repository();
    for status in [
        PrismInstanceStatus::Pending,
        PrismInstanceStatus::Indexing,
        PrismInstanceStatus::Failed,
        PrismInstanceStatus::Missing,
    ] {
        let instance = repository
            .upsert_prism_instance(prism_instance(status.as_str(), status))
            .expect("insert instance");

        let result = repository.create_scheme(NewScheme {
            prism_instance_id: instance.id,
            name: "Blocked".to_string(),
            size_x: 16,
            size_y: 16,
            size_z: 16,
        });

        assert!(
            result.is_err(),
            "{status:?} instance must reject new schemes"
        );
    }
}

#[test]
fn ready_instance_schemes_are_crud_records() {
    let repository = open_test_repository();
    let instance = repository
        .upsert_prism_instance(prism_instance("aoc", PrismInstanceStatus::Ready))
        .expect("insert instance");

    let scheme = repository
        .create_scheme(NewScheme {
            prism_instance_id: instance.id,
            name: "Starter Factory".to_string(),
            size_x: 64,
            size_y: 48,
            size_z: 32,
        })
        .expect("create scheme");

    assert_eq!(scheme.prism_instance_id, instance.id);
    assert_eq!(scheme.name, "Starter Factory");
    assert_eq!(scheme.dimensions, (64, 48, 32));

    let renamed = repository
        .rename_scheme(scheme.id, "Starter Factory Revised")
        .expect("rename scheme");
    assert_eq!(renamed.name, "Starter Factory Revised");

    let library = repository.list_library().expect("list library");
    assert_eq!(library.len(), 1);
    assert_eq!(library[0].schemes.len(), 1);
    assert_eq!(library[0].schemes[0].name, "Starter Factory Revised");

    repository.delete_scheme(scheme.id).expect("delete scheme");
    let library = repository
        .list_library()
        .expect("list library after delete");
    assert!(library[0].schemes.is_empty());
}

#[test]
fn opening_database_with_future_migration_version_returns_recovery_error() {
    let temp = tempdir().expect("temp dir");
    let db_path = temp.path().join("library.sqlite3");
    let raw = rusqlite::Connection::open(&db_path).expect("open raw database");
    raw.execute_batch(
        "CREATE TABLE schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        INSERT INTO schema_migrations (version) VALUES (999);",
    )
    .expect("seed future migration");
    drop(raw);

    let error = match LibraryDatabase::open(&db_path) {
        Ok(_) => panic!("future migration should fail"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("newer app version"));
    assert!(error.to_string().contains("diagnostics"));
}

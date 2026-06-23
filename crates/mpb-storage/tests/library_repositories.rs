use std::path::PathBuf;

use mpb_storage::{
    ImportStatus, LibraryDatabase, LibraryRepository, NewImportedModpack, NewScheme,
};
use tempfile::tempdir;

fn open_test_repository() -> LibraryRepository {
    let database = LibraryDatabase::open_in_memory().expect("open database");
    LibraryRepository::new(database)
}

fn imported_modpack(name: &str) -> NewImportedModpack {
    NewImportedModpack {
        local_name: name.to_string(),
        source_slug: Some("aoc".to_string()),
        source_url: Some("https://www.curseforge.com/minecraft/modpacks/aoc".to_string()),
        version_name: "1.0.0".to_string(),
        minecraft_version: Some("1.20.1".to_string()),
        loader: Some("Forge".to_string()),
        cache_dir: Some(PathBuf::from("cache/aoc")),
        import_status: ImportStatus::Imported,
    }
}

#[test]
fn migrations_create_phase_3_tables() {
    let temp = tempdir().expect("temp dir");
    let db_path = temp.path().join("library.sqlite3");
    let database = LibraryDatabase::open(&db_path).expect("open database");

    let tables = database.table_names().expect("table names");

    for expected in [
        "schema_migrations",
        "imported_modpacks",
        "schemes",
        "scheme_dimensions",
        "construction_stages",
        "settings_metadata",
        "import_status",
    ] {
        assert!(
            tables.contains(&expected.to_string()),
            "missing table {expected}"
        );
    }
}

#[test]
fn imported_modpack_names_receive_numeric_suffixes() {
    let repository = open_test_repository();

    let first = repository
        .create_imported_modpack(imported_modpack("All the Mods 10 - 2.14.1"))
        .expect("create first");
    let second = repository
        .create_imported_modpack(imported_modpack("All the Mods 10 - 2.14.1"))
        .expect("create duplicate");
    let third = repository
        .create_imported_modpack(imported_modpack("All the Mods 10 - 2.14.1"))
        .expect("create third duplicate");

    assert_eq!(first.local_name, "All the Mods 10 - 2.14.1");
    assert_eq!(second.local_name, "All the Mods 10 - 2.14.1 (2)");
    assert_eq!(third.local_name, "All the Mods 10 - 2.14.1 (3)");
}

#[test]
fn import_status_updates_are_visible_in_the_library_tree() {
    let repository = open_test_repository();
    let modpack = repository
        .create_imported_modpack(NewImportedModpack {
            local_name: "All of Create".to_string(),
            source_slug: Some("aoc".to_string()),
            source_url: Some("https://www.curseforge.com/minecraft/modpacks/aoc".to_string()),
            version_name: "All of Create 1.21.1 - v2.1".to_string(),
            minecraft_version: Some("1.21.1".to_string()),
            loader: Some("NeoForge".to_string()),
            cache_dir: Some(PathBuf::from("cache/aoc-123")),
            import_status: ImportStatus::Importing,
        })
        .expect("create importing modpack");

    repository
        .update_import_status(
            modpack.id,
            ImportStatus::Failed,
            Some("Could not parse modpack assets".to_string()),
        )
        .expect("mark failed");

    let library = repository.list_library().expect("list library");
    assert_eq!(library[0].import_status, ImportStatus::Failed);
    assert_eq!(
        library[0].import_message.as_deref(),
        Some("Could not parse modpack assets")
    );
}

#[test]
fn schemes_are_crud_records_owned_by_one_modpack() {
    let repository = open_test_repository();
    let modpack = repository
        .create_imported_modpack(imported_modpack("AOC"))
        .expect("create modpack");

    let scheme = repository
        .create_scheme(NewScheme {
            modpack_id: modpack.id,
            name: "Starter Factory".to_string(),
            size_x: 64,
            size_y: 48,
            size_z: 32,
        })
        .expect("create scheme");

    assert_eq!(scheme.modpack_id, modpack.id);
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
fn deleting_a_modpack_cascades_schemes_and_returns_cache_path() {
    let repository = open_test_repository();
    let modpack = repository
        .create_imported_modpack(imported_modpack("AOC"))
        .expect("create modpack");
    repository
        .create_scheme(NewScheme {
            modpack_id: modpack.id,
            name: "Starter Factory".to_string(),
            size_x: 64,
            size_y: 64,
            size_z: 64,
        })
        .expect("create scheme");

    let deleted = repository
        .delete_imported_modpack(modpack.id)
        .expect("delete modpack");

    assert_eq!(deleted.removed_scheme_count, 1);
    assert_eq!(deleted.cache_dir, Some(PathBuf::from("cache/aoc")));
    assert!(repository.list_library().expect("list library").is_empty());
}

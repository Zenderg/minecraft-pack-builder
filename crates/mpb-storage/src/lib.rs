//! Local storage, SQLite library repositories, and app data path helpers.

use std::fs;
use std::path::{Path, PathBuf};

use mpb_core::{ConstructionStage, Coordinate, Dimensions, Scheme, SchemeBlock};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("failed to create application data directory at {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("sqlite storage error: {source}")]
    Sqlite {
        #[from]
        source: rusqlite::Error,
    },
    #[error("scheme dimensions must be positive, got {size_x} x {size_y} x {size_z}")]
    InvalidDimensions {
        size_x: i64,
        size_y: i64,
        size_z: i64,
    },
    #[error("record not found: {entity} {id}")]
    NotFound { entity: &'static str, id: i64 },
    #[error("stored scheme content is invalid: {0}")]
    InvalidSchemeContent(String),
    #[error("Prism instance {id} is {status} and cannot be edited until it is ready")]
    InstanceNotReady {
        id: i64,
        status: PrismInstanceStatus,
    },
    #[error(
        "local data was created by a newer app version (schema {found}, supported {supported}); keep the data folder unchanged, check diagnostics, and open it with a compatible Minecraft Pack Builder build"
    )]
    UnsupportedMigrationVersion { found: i64, supported: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDataPaths {
    pub app_data_dir: PathBuf,
    pub diagnostics_dir: PathBuf,
}

pub fn ensure_app_data_dirs(base_dir: impl AsRef<Path>) -> Result<AppDataPaths, StorageError> {
    let app_data_dir = base_dir.as_ref().to_path_buf();
    let diagnostics_dir = app_data_dir.join("diagnostics");

    create_dir(&app_data_dir)?;
    create_dir(&diagnostics_dir)?;

    Ok(AppDataPaths {
        app_data_dir,
        diagnostics_dir,
    })
}

fn create_dir(path: &Path) -> Result<(), StorageError> {
    fs::create_dir_all(path).map_err(|source| StorageError::CreateDir {
        path: path.to_path_buf(),
        source,
    })
}

pub struct LibraryDatabase {
    connection: Connection,
}

const SUPPORTED_SCHEMA_VERSION: i64 = 5;

impl LibraryDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let database = Self { connection };
        database.migrate()?;
        Ok(database)
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let connection = Connection::open_in_memory()?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let database = Self { connection };
        database.migrate()?;
        Ok(database)
    }

    pub fn table_names(&self) -> Result<Vec<String>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    fn migrate(&self) -> Result<(), StorageError> {
        self.connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
              version INTEGER PRIMARY KEY,
              applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )?;
        let current_version = self.current_migration_version()?;
        if current_version > SUPPORTED_SCHEMA_VERSION {
            return Err(StorageError::UnsupportedMigrationVersion {
                found: current_version,
                supported: SUPPORTED_SCHEMA_VERSION,
            });
        }
        if current_version < 5 {
            self.connection
                .execute_batch(DROP_UNRELEASED_IMPORT_SCHEMA)?;
        }
        self.connection.execute_batch(PRISM_SCHEMA)?;
        self.connection.execute(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES (5)",
            [],
        )?;
        Ok(())
    }

    fn current_migration_version(&self) -> Result<i64, StorageError> {
        Ok(self
            .connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get::<_, Option<i64>>(0)
            })?
            .unwrap_or(0))
    }
}

const DROP_UNRELEASED_IMPORT_SCHEMA: &str = r#"
DROP TABLE IF EXISTS import_status;
DROP TABLE IF EXISTS scheme_documents;
DROP TABLE IF EXISTS construction_stages;
DROP TABLE IF EXISTS scheme_dimensions;
DROP TABLE IF EXISTS schemes;
DROP TABLE IF EXISTS imported_modpacks;
"#;

const PRISM_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS settings_metadata (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS prism_instances (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  instance_id TEXT NOT NULL,
  display_name TEXT NOT NULL,
  instance_path TEXT NOT NULL,
  minecraft_dir TEXT NOT NULL,
  minecraft_version TEXT,
  loader TEXT,
  loader_version TEXT,
  identity_fingerprint TEXT NOT NULL UNIQUE,
  content_fingerprint TEXT NOT NULL,
  status TEXT NOT NULL,
  status_message TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS schemes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  prism_instance_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (prism_instance_id) REFERENCES prism_instances(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS scheme_dimensions (
  scheme_id INTEGER PRIMARY KEY,
  size_x INTEGER NOT NULL,
  size_y INTEGER NOT NULL,
  size_z INTEGER NOT NULL,
  FOREIGN KEY (scheme_id) REFERENCES schemes(id) ON DELETE CASCADE,
  CHECK (size_x > 0 AND size_y > 0 AND size_z > 0)
);

CREATE TABLE IF NOT EXISTS construction_stages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  scheme_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  position INTEGER NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (scheme_id) REFERENCES schemes(id) ON DELETE CASCADE,
  UNIQUE (scheme_id, position)
);

CREATE TABLE IF NOT EXISTS scheme_documents (
  scheme_id INTEGER PRIMARY KEY,
  content_json TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (scheme_id) REFERENCES schemes(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS asset_indexes (
  prism_instance_id INTEGER PRIMARY KEY,
  static_status TEXT NOT NULL,
  runtime_status TEXT NOT NULL,
  asset_report_path TEXT,
  registry_report_path TEXT,
  input_fingerprint TEXT,
  message TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (prism_instance_id) REFERENCES prism_instances(id) ON DELETE CASCADE
);
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PrismInstanceStatus {
    Pending,
    Indexing,
    Ready,
    Failed,
    Missing,
}

impl PrismInstanceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Indexing => "indexing",
            Self::Ready => "ready",
            Self::Failed => "failed",
            Self::Missing => "missing",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "indexing" => Self::Indexing,
            "ready" => Self::Ready,
            "failed" => Self::Failed,
            "missing" => Self::Missing,
            _ => Self::Pending,
        }
    }
}

impl std::fmt::Display for PrismInstanceStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPrismInstance {
    pub instance_id: String,
    pub display_name: String,
    pub instance_path: PathBuf,
    pub minecraft_dir: PathBuf,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub identity_fingerprint: String,
    pub content_fingerprint: String,
    pub status: PrismInstanceStatus,
    pub status_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismInstanceRecord {
    pub id: i64,
    pub instance_id: String,
    pub display_name: String,
    pub instance_path: PathBuf,
    pub minecraft_dir: PathBuf,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub identity_fingerprint: String,
    pub content_fingerprint: String,
    pub status: PrismInstanceStatus,
    pub status_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewScheme {
    pub prism_instance_id: i64,
    pub name: String,
    pub size_x: i64,
    pub size_y: i64,
    pub size_z: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemeRecord {
    pub id: i64,
    pub prism_instance_id: i64,
    pub name: String,
    pub dimensions: (i64, i64, i64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredScheme {
    pub record: SchemeRecord,
    pub scheme: Scheme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemeDocument {
    name: String,
    dimensions: [i32; 3],
    stages: Vec<ConstructionStage>,
    blocks: Vec<SchemeDocumentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchemeDocumentBlock {
    coordinate: Coordinate,
    block: SchemeBlock,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInstance {
    pub id: i64,
    pub instance_id: String,
    pub display_name: String,
    pub instance_path: PathBuf,
    pub minecraft_dir: PathBuf,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub loader_version: Option<String>,
    pub status: PrismInstanceStatus,
    pub status_message: Option<String>,
    pub schemes: Vec<SchemeRecord>,
}

pub struct LibraryRepository {
    database: LibraryDatabase,
}

impl LibraryRepository {
    pub fn new(database: LibraryDatabase) -> Self {
        Self { database }
    }

    pub fn set_prism_root(&self, root: Option<PathBuf>) -> Result<(), StorageError> {
        match root {
            Some(root) => {
                self.database.connection.execute(
                    "INSERT INTO settings_metadata (key, value, updated_at)
                     VALUES ('prism_root_path', ?1, CURRENT_TIMESTAMP)
                     ON CONFLICT(key) DO UPDATE SET
                       value = excluded.value,
                       updated_at = CURRENT_TIMESTAMP",
                    params![root.to_string_lossy().to_string()],
                )?;
            }
            None => {
                self.database.connection.execute(
                    "DELETE FROM settings_metadata WHERE key = 'prism_root_path'",
                    [],
                )?;
            }
        }
        Ok(())
    }

    pub fn get_prism_root(&self) -> Result<Option<PathBuf>, StorageError> {
        Ok(self
            .database
            .connection
            .query_row(
                "SELECT value FROM settings_metadata WHERE key = 'prism_root_path'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .map(PathBuf::from))
    }

    pub fn upsert_prism_instance(
        &self,
        instance: NewPrismInstance,
    ) -> Result<PrismInstanceRecord, StorageError> {
        self.database.connection.execute(
            "INSERT INTO prism_instances (
                instance_id, display_name, instance_path, minecraft_dir, minecraft_version,
                loader, loader_version, identity_fingerprint, content_fingerprint, status, status_message
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(identity_fingerprint) DO UPDATE SET
                instance_id = excluded.instance_id,
                display_name = excluded.display_name,
                instance_path = excluded.instance_path,
                minecraft_dir = excluded.minecraft_dir,
                minecraft_version = excluded.minecraft_version,
                loader = excluded.loader,
                loader_version = excluded.loader_version,
                content_fingerprint = excluded.content_fingerprint,
                status = excluded.status,
                status_message = excluded.status_message,
                updated_at = CURRENT_TIMESTAMP",
            params![
                clean_display_name(&instance.instance_id, "instance"),
                clean_display_name(&instance.display_name, "Prism instance"),
                path_to_string(instance.instance_path),
                path_to_string(instance.minecraft_dir),
                instance.minecraft_version,
                instance.loader,
                instance.loader_version,
                instance.identity_fingerprint,
                instance.content_fingerprint,
                instance.status.as_str(),
                instance.status_message,
            ],
        )?;
        self.get_prism_instance_by_identity_fingerprint(&instance.identity_fingerprint)?
            .ok_or(StorageError::NotFound {
                entity: "prism_instance",
                id: 0,
            })
    }

    pub fn mark_prism_instances_missing_except(
        &self,
        active_identity_fingerprints: &[String],
    ) -> Result<(), StorageError> {
        let active = active_identity_fingerprints
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        for instance in self.list_prism_instance_records()? {
            if !active.contains(instance.identity_fingerprint.as_str())
                && instance.status != PrismInstanceStatus::Missing
            {
                self.database.connection.execute(
                    "UPDATE prism_instances
                     SET status = 'missing',
                         status_message = 'Prism instance was not found in the active Launcher Root.',
                         updated_at = CURRENT_TIMESTAMP
                     WHERE id = ?1",
                    params![instance.id],
                )?;
            }
        }
        Ok(())
    }

    pub fn create_scheme(&self, new_scheme: NewScheme) -> Result<SchemeRecord, StorageError> {
        validate_dimensions(new_scheme.size_x, new_scheme.size_y, new_scheme.size_z)?;
        self.require_instance_ready(new_scheme.prism_instance_id)?;
        let transaction = self.database.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO schemes (prism_instance_id, name) VALUES (?1, ?2)",
            params![new_scheme.prism_instance_id, new_scheme.name.as_str()],
        )?;
        let id = transaction.last_insert_rowid();
        transaction.execute(
            "INSERT INTO scheme_dimensions (scheme_id, size_x, size_y, size_z) VALUES (?1, ?2, ?3, ?4)",
            params![id, new_scheme.size_x, new_scheme.size_y, new_scheme.size_z],
        )?;
        transaction.execute(
            "INSERT INTO construction_stages (scheme_id, name, position) VALUES (?1, ?2, 0)",
            params![id, "Unassigned"],
        )?;
        let dimensions = Dimensions::new(
            new_scheme.size_x as i32,
            new_scheme.size_y as i32,
            new_scheme.size_z as i32,
        )
        .map_err(|error| StorageError::InvalidSchemeContent(error.to_string()))?;
        let scheme = Scheme::new(&new_scheme.name, dimensions);
        let content_json = serialize_scheme_document(&scheme)?;
        transaction.execute(
            "INSERT INTO scheme_documents (scheme_id, content_json) VALUES (?1, ?2)",
            params![id, content_json],
        )?;
        let scheme = transaction.query_row(
            "SELECT s.id, s.prism_instance_id, s.name, d.size_x, d.size_y, d.size_z
             FROM schemes s
             JOIN scheme_dimensions d ON d.scheme_id = s.id
             WHERE s.id = ?1",
            params![id],
            row_to_scheme,
        )?;
        transaction.commit()?;
        Ok(scheme)
    }

    pub fn rename_scheme(&self, id: i64, name: &str) -> Result<SchemeRecord, StorageError> {
        let affected = self.database.connection.execute(
            "UPDATE schemes SET name = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![name, id],
        )?;
        if affected == 0 {
            return Err(StorageError::NotFound {
                entity: "scheme",
                id,
            });
        }
        let mut stored = self.load_scheme(id)?;
        stored.scheme.set_name(name);
        self.save_scheme(id, &stored.scheme)?;
        self.get_scheme(id)
    }

    pub fn delete_scheme(&self, id: i64) -> Result<(), StorageError> {
        let affected = self
            .database
            .connection
            .execute("DELETE FROM schemes WHERE id = ?1", params![id])?;
        if affected == 0 {
            return Err(StorageError::NotFound {
                entity: "scheme",
                id,
            });
        }
        Ok(())
    }

    pub fn load_scheme(&self, id: i64) -> Result<StoredScheme, StorageError> {
        let record = self.get_scheme(id)?;
        let content_json = self
            .database
            .connection
            .query_row(
                "SELECT content_json FROM scheme_documents WHERE scheme_id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let scheme = match content_json {
            Some(json) => deserialize_scheme_document(&json)?,
            None => empty_scheme_from_record(&record)?,
        };
        Ok(StoredScheme { record, scheme })
    }

    pub fn save_scheme(&self, id: i64, scheme: &Scheme) -> Result<(), StorageError> {
        self.require_scheme(id)?;
        let dimensions = scheme.dimensions();
        let content_json = serialize_scheme_document(scheme)?;
        let transaction = self.database.connection.unchecked_transaction()?;
        transaction.execute(
            "UPDATE schemes SET name = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![scheme.name(), id],
        )?;
        transaction.execute(
            "UPDATE scheme_dimensions SET size_x = ?1, size_y = ?2, size_z = ?3 WHERE scheme_id = ?4",
            params![dimensions.x, dimensions.y, dimensions.z, id],
        )?;
        transaction.execute(
            "INSERT INTO scheme_documents (scheme_id, content_json, updated_at)
             VALUES (?1, ?2, CURRENT_TIMESTAMP)
             ON CONFLICT(scheme_id) DO UPDATE SET
               content_json = excluded.content_json,
               updated_at = CURRENT_TIMESTAMP",
            params![id, content_json],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn list_library(&self) -> Result<Vec<LibraryInstance>, StorageError> {
        let mut library = self
            .list_prism_instance_records()?
            .into_iter()
            .map(|instance| LibraryInstance {
                id: instance.id,
                instance_id: instance.instance_id,
                display_name: instance.display_name,
                instance_path: instance.instance_path,
                minecraft_dir: instance.minecraft_dir,
                minecraft_version: instance.minecraft_version,
                loader: instance.loader,
                loader_version: instance.loader_version,
                status: instance.status,
                status_message: instance.status_message,
                schemes: Vec::new(),
            })
            .collect::<Vec<_>>();
        for instance in &mut library {
            instance.schemes = self.list_schemes_for_instance(instance.id)?;
        }
        Ok(library)
    }

    pub fn get_prism_instance(&self, id: i64) -> Result<PrismInstanceRecord, StorageError> {
        self.database
            .connection
            .query_row(
                "SELECT
                    id, instance_id, display_name, instance_path, minecraft_dir,
                    minecraft_version, loader, loader_version, identity_fingerprint,
                    content_fingerprint, status, status_message
                 FROM prism_instances
                 WHERE id = ?1",
                params![id],
                row_to_prism_instance,
            )
            .optional()?
            .ok_or(StorageError::NotFound {
                entity: "prism_instance",
                id,
            })
    }

    pub fn list_prism_instances(&self) -> Result<Vec<PrismInstanceRecord>, StorageError> {
        self.list_prism_instance_records()
    }

    pub fn get_prism_instance_by_identity_fingerprint(
        &self,
        identity_fingerprint: &str,
    ) -> Result<Option<PrismInstanceRecord>, StorageError> {
        self.database
            .connection
            .query_row(
                "SELECT
                    id, instance_id, display_name, instance_path, minecraft_dir,
                    minecraft_version, loader, loader_version, identity_fingerprint,
                    content_fingerprint, status, status_message
                 FROM prism_instances
                 WHERE identity_fingerprint = ?1",
                params![identity_fingerprint],
                row_to_prism_instance,
            )
            .optional()
            .map_err(StorageError::from)
    }

    pub fn update_prism_instance_status(
        &self,
        id: i64,
        status: PrismInstanceStatus,
        status_message: Option<&str>,
    ) -> Result<(), StorageError> {
        let affected = self.database.connection.execute(
            "UPDATE prism_instances
             SET status = ?1,
                 status_message = ?2,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?3",
            params![status.as_str(), status_message, id],
        )?;
        if affected == 0 {
            return Err(StorageError::NotFound {
                entity: "prism_instance",
                id,
            });
        }
        Ok(())
    }

    pub fn relink_prism_instance(
        &self,
        existing_id: i64,
        instance: NewPrismInstance,
    ) -> Result<PrismInstanceRecord, StorageError> {
        let affected = self.database.connection.execute(
            "UPDATE prism_instances
             SET instance_id = ?1,
                 display_name = ?2,
                 instance_path = ?3,
                 minecraft_dir = ?4,
                 minecraft_version = ?5,
                 loader = ?6,
                 loader_version = ?7,
                 identity_fingerprint = ?8,
                 content_fingerprint = ?9,
                 status = ?10,
                 status_message = ?11,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?12",
            params![
                clean_display_name(&instance.instance_id, "instance"),
                clean_display_name(&instance.display_name, "Prism instance"),
                path_to_string(instance.instance_path),
                path_to_string(instance.minecraft_dir),
                instance.minecraft_version,
                instance.loader,
                instance.loader_version,
                instance.identity_fingerprint,
                instance.content_fingerprint,
                instance.status.as_str(),
                instance.status_message,
                existing_id,
            ],
        )?;
        if affected == 0 {
            return Err(StorageError::NotFound {
                entity: "prism_instance",
                id: existing_id,
            });
        }
        self.get_prism_instance(existing_id)
    }

    fn list_prism_instance_records(&self) -> Result<Vec<PrismInstanceRecord>, StorageError> {
        let mut statement = self.database.connection.prepare(
            "SELECT
                id, instance_id, display_name, instance_path, minecraft_dir,
                minecraft_version, loader, loader_version, identity_fingerprint,
                content_fingerprint, status, status_message
             FROM prism_instances
             ORDER BY display_name COLLATE NOCASE, id",
        )?;
        let rows = statement.query_map([], row_to_prism_instance)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    fn get_scheme(&self, id: i64) -> Result<SchemeRecord, StorageError> {
        self.database
            .connection
            .query_row(
                "SELECT s.id, s.prism_instance_id, s.name, d.size_x, d.size_y, d.size_z
                 FROM schemes s
                 JOIN scheme_dimensions d ON d.scheme_id = s.id
                 WHERE s.id = ?1",
                params![id],
                row_to_scheme,
            )
            .optional()?
            .ok_or(StorageError::NotFound {
                entity: "scheme",
                id,
            })
    }

    fn require_scheme(&self, id: i64) -> Result<(), StorageError> {
        let exists: bool = self.database.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM schemes WHERE id = ?1)",
            params![id],
            |row| row.get(0),
        )?;
        if exists {
            Ok(())
        } else {
            Err(StorageError::NotFound {
                entity: "scheme",
                id,
            })
        }
    }

    fn require_instance_ready(&self, id: i64) -> Result<(), StorageError> {
        let instance = self.get_prism_instance(id)?;
        if instance.status == PrismInstanceStatus::Ready {
            Ok(())
        } else {
            Err(StorageError::InstanceNotReady {
                id,
                status: instance.status,
            })
        }
    }

    fn list_schemes_for_instance(
        &self,
        prism_instance_id: i64,
    ) -> Result<Vec<SchemeRecord>, StorageError> {
        let mut statement = self.database.connection.prepare(
            "SELECT s.id, s.prism_instance_id, s.name, d.size_x, d.size_y, d.size_z
             FROM schemes s
             JOIN scheme_dimensions d ON d.scheme_id = s.id
             WHERE s.prism_instance_id = ?1
             ORDER BY s.name COLLATE NOCASE, s.id",
        )?;
        let rows = statement.query_map(params![prism_instance_id], row_to_scheme)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }
}

fn row_to_prism_instance(row: &rusqlite::Row<'_>) -> rusqlite::Result<PrismInstanceRecord> {
    let instance_path: String = row.get(3)?;
    let minecraft_dir: String = row.get(4)?;
    let status: String = row.get(10)?;
    Ok(PrismInstanceRecord {
        id: row.get(0)?,
        instance_id: row.get(1)?,
        display_name: row.get(2)?,
        instance_path: PathBuf::from(instance_path),
        minecraft_dir: PathBuf::from(minecraft_dir),
        minecraft_version: row.get(5)?,
        loader: row.get(6)?,
        loader_version: row.get(7)?,
        identity_fingerprint: row.get(8)?,
        content_fingerprint: row.get(9)?,
        status: PrismInstanceStatus::from_str(&status),
        status_message: row.get(11)?,
    })
}

fn row_to_scheme(row: &rusqlite::Row<'_>) -> rusqlite::Result<SchemeRecord> {
    Ok(SchemeRecord {
        id: row.get(0)?,
        prism_instance_id: row.get(1)?,
        name: row.get(2)?,
        dimensions: (row.get(3)?, row.get(4)?, row.get(5)?),
    })
}

fn validate_dimensions(size_x: i64, size_y: i64, size_z: i64) -> Result<(), StorageError> {
    if size_x <= 0 || size_y <= 0 || size_z <= 0 {
        return Err(StorageError::InvalidDimensions {
            size_x,
            size_y,
            size_z,
        });
    }
    Ok(())
}

fn serialize_scheme_document(scheme: &Scheme) -> Result<String, StorageError> {
    let dimensions = scheme.dimensions();
    let document = SchemeDocument {
        name: scheme.name().to_string(),
        dimensions: [dimensions.x, dimensions.y, dimensions.z],
        stages: scheme.stages().to_vec(),
        blocks: scheme
            .blocks()
            .map(|(coordinate, block)| SchemeDocumentBlock {
                coordinate: *coordinate,
                block: block.clone(),
            })
            .collect(),
    };
    serde_json::to_string(&document)
        .map_err(|error| StorageError::InvalidSchemeContent(error.to_string()))
}

fn deserialize_scheme_document(json: &str) -> Result<Scheme, StorageError> {
    let document: SchemeDocument = serde_json::from_str(json)
        .map_err(|error| StorageError::InvalidSchemeContent(error.to_string()))?;
    let dimensions = Dimensions::new(
        document.dimensions[0],
        document.dimensions[1],
        document.dimensions[2],
    )
    .map_err(|error| StorageError::InvalidSchemeContent(error.to_string()))?;
    Scheme::from_persisted(
        &document.name,
        dimensions,
        document.stages,
        document
            .blocks
            .into_iter()
            .map(|block| (block.coordinate, block.block))
            .collect(),
    )
    .map_err(|error| StorageError::InvalidSchemeContent(error.to_string()))
}

fn empty_scheme_from_record(record: &SchemeRecord) -> Result<Scheme, StorageError> {
    let dimensions = Dimensions::new(
        record.dimensions.0 as i32,
        record.dimensions.1 as i32,
        record.dimensions.2 as i32,
    )
    .map_err(|error| StorageError::InvalidSchemeContent(error.to_string()))?;
    Ok(Scheme::new(&record.name, dimensions))
}

fn clean_display_name(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().to_string()
}

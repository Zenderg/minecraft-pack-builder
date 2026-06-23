//! Local storage, SQLite library repositories, and app data path helpers.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
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
        self.connection.execute_batch(PHASE_3_SCHEMA)?;
        self.connection.execute(
            "INSERT OR IGNORE INTO schema_migrations (version) VALUES (3)",
            [],
        )?;
        Ok(())
    }
}

const PHASE_3_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS imported_modpacks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  local_name TEXT NOT NULL UNIQUE,
  source_slug TEXT,
  source_url TEXT,
  version_name TEXT NOT NULL,
  minecraft_version TEXT,
  loader TEXT,
  cache_dir TEXT,
  import_status TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS import_status (
  imported_modpack_id INTEGER PRIMARY KEY,
  status TEXT NOT NULL,
  message TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (imported_modpack_id) REFERENCES imported_modpacks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS schemes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  imported_modpack_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (imported_modpack_id) REFERENCES imported_modpacks(id) ON DELETE CASCADE
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

CREATE TABLE IF NOT EXISTS settings_metadata (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportStatus {
    Imported,
    Importing,
    Failed,
}

impl ImportStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Imported => "imported",
            Self::Importing => "importing",
            Self::Failed => "failed",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "importing" => Self::Importing,
            "failed" => Self::Failed,
            _ => Self::Imported,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewImportedModpack {
    pub local_name: String,
    pub source_slug: Option<String>,
    pub source_url: Option<String>,
    pub version_name: String,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub cache_dir: Option<PathBuf>,
    pub import_status: ImportStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedModpack {
    pub id: i64,
    pub local_name: String,
    pub source_slug: Option<String>,
    pub source_url: Option<String>,
    pub version_name: String,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub cache_dir: Option<PathBuf>,
    pub import_status: ImportStatus,
    pub import_message: Option<String>,
    pub scheme_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewScheme {
    pub modpack_id: i64,
    pub name: String,
    pub size_x: i64,
    pub size_y: i64,
    pub size_z: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemeRecord {
    pub id: i64,
    pub modpack_id: i64,
    pub name: String,
    pub dimensions: (i64, i64, i64),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryModpack {
    pub id: i64,
    pub local_name: String,
    pub source_url: Option<String>,
    pub version_name: String,
    pub minecraft_version: Option<String>,
    pub loader: Option<String>,
    pub import_status: ImportStatus,
    pub import_message: Option<String>,
    pub schemes: Vec<SchemeRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedImportedModpack {
    pub cache_dir: Option<PathBuf>,
    pub removed_scheme_count: i64,
}

pub struct LibraryRepository {
    database: LibraryDatabase,
}

impl LibraryRepository {
    pub fn new(database: LibraryDatabase) -> Self {
        Self { database }
    }

    pub fn create_imported_modpack(
        &self,
        new_modpack: NewImportedModpack,
    ) -> Result<ImportedModpack, StorageError> {
        let local_name = self.next_unique_modpack_name(&new_modpack.local_name)?;
        self.database.connection.execute(
            "INSERT INTO imported_modpacks (
                local_name, source_slug, source_url, version_name, minecraft_version, loader, cache_dir, import_status
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                local_name,
                new_modpack.source_slug,
                new_modpack.source_url,
                new_modpack.version_name,
                new_modpack.minecraft_version,
                new_modpack.loader,
                new_modpack.cache_dir.map(|path| path.to_string_lossy().to_string()),
                new_modpack.import_status.as_str(),
            ],
        )?;
        let id = self.database.connection.last_insert_rowid();
        self.database.connection.execute(
            "INSERT INTO import_status (imported_modpack_id, status) VALUES (?1, ?2)",
            params![id, new_modpack.import_status.as_str()],
        )?;
        self.get_imported_modpack(id)
    }

    pub fn rename_imported_modpack(
        &self,
        id: i64,
        requested_name: &str,
    ) -> Result<ImportedModpack, StorageError> {
        self.require_modpack(id)?;
        let local_name = self.next_unique_modpack_name_excluding(requested_name, Some(id))?;
        let affected = self.database.connection.execute(
            "UPDATE imported_modpacks SET local_name = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![local_name, id],
        )?;
        if affected == 0 {
            return Err(StorageError::NotFound {
                entity: "imported_modpack",
                id,
            });
        }
        self.get_imported_modpack(id)
    }

    pub fn delete_imported_modpack(&self, id: i64) -> Result<DeletedImportedModpack, StorageError> {
        let cache_dir = self
            .database
            .connection
            .query_row(
                "SELECT cache_dir FROM imported_modpacks WHERE id = ?1",
                params![id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .ok_or(StorageError::NotFound {
                entity: "imported_modpack",
                id,
            })?
            .map(PathBuf::from);
        let removed_scheme_count = self.database.connection.query_row(
            "SELECT COUNT(*) FROM schemes WHERE imported_modpack_id = ?1",
            params![id],
            |row| row.get::<_, i64>(0),
        )?;
        self.database
            .connection
            .execute("DELETE FROM imported_modpacks WHERE id = ?1", params![id])?;

        Ok(DeletedImportedModpack {
            cache_dir,
            removed_scheme_count,
        })
    }

    pub fn update_import_status(
        &self,
        id: i64,
        status: ImportStatus,
        message: Option<String>,
    ) -> Result<ImportedModpack, StorageError> {
        self.require_modpack(id)?;
        self.database.connection.execute(
            "UPDATE imported_modpacks SET import_status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![status.as_str(), id],
        )?;
        self.database.connection.execute(
            "INSERT INTO import_status (imported_modpack_id, status, message, updated_at)
             VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)
             ON CONFLICT(imported_modpack_id) DO UPDATE SET
               status = excluded.status,
               message = excluded.message,
               updated_at = CURRENT_TIMESTAMP",
            params![id, status.as_str(), message],
        )?;
        self.get_imported_modpack(id)
    }

    pub fn create_scheme(&self, new_scheme: NewScheme) -> Result<SchemeRecord, StorageError> {
        validate_dimensions(new_scheme.size_x, new_scheme.size_y, new_scheme.size_z)?;
        self.require_modpack(new_scheme.modpack_id)?;
        self.database.connection.execute(
            "INSERT INTO schemes (imported_modpack_id, name) VALUES (?1, ?2)",
            params![new_scheme.modpack_id, new_scheme.name],
        )?;
        let id = self.database.connection.last_insert_rowid();
        self.database.connection.execute(
            "INSERT INTO scheme_dimensions (scheme_id, size_x, size_y, size_z) VALUES (?1, ?2, ?3, ?4)",
            params![id, new_scheme.size_x, new_scheme.size_y, new_scheme.size_z],
        )?;
        self.database.connection.execute(
            "INSERT INTO construction_stages (scheme_id, name, position) VALUES (?1, ?2, 0)",
            params![id, "Unassigned"],
        )?;
        self.get_scheme(id)
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

    pub fn list_library(&self) -> Result<Vec<LibraryModpack>, StorageError> {
        let mut statement = self.database.connection.prepare(
            "SELECT m.id, m.local_name, m.source_url, m.version_name, m.minecraft_version, m.loader, m.import_status, s.message
             FROM imported_modpacks m
             LEFT JOIN import_status s ON s.imported_modpack_id = m.id
             ORDER BY local_name COLLATE NOCASE, id",
        )?;
        let modpack_rows = statement.query_map([], |row| {
            Ok(LibraryModpack {
                id: row.get(0)?,
                local_name: row.get(1)?,
                source_url: row.get(2)?,
                version_name: row.get(3)?,
                minecraft_version: row.get(4)?,
                loader: row.get(5)?,
                import_status: ImportStatus::from_str(row.get::<_, String>(6)?.as_str()),
                import_message: row.get(7)?,
                schemes: Vec::new(),
            })
        })?;
        let mut library = modpack_rows.collect::<Result<Vec<_>, _>>()?;
        for modpack in &mut library {
            modpack.schemes = self.list_schemes_for_modpack(modpack.id)?;
        }
        Ok(library)
    }

    pub fn get_imported_modpack(&self, id: i64) -> Result<ImportedModpack, StorageError> {
        self.database
            .connection
            .query_row(
                "SELECT
                    m.id, m.local_name, m.source_slug, m.source_url, m.version_name,
                    m.minecraft_version, m.loader, m.cache_dir, m.import_status, ist.message,
                    COUNT(s.id) AS scheme_count
                 FROM imported_modpacks m
                 LEFT JOIN schemes s ON s.imported_modpack_id = m.id
                 LEFT JOIN import_status ist ON ist.imported_modpack_id = m.id
                 WHERE m.id = ?1
                 GROUP BY m.id",
                params![id],
                |row| {
                    let cache_dir: Option<String> = row.get(7)?;
                    Ok(ImportedModpack {
                        id: row.get(0)?,
                        local_name: row.get(1)?,
                        source_slug: row.get(2)?,
                        source_url: row.get(3)?,
                        version_name: row.get(4)?,
                        minecraft_version: row.get(5)?,
                        loader: row.get(6)?,
                        cache_dir: cache_dir.map(PathBuf::from),
                        import_status: ImportStatus::from_str(row.get::<_, String>(8)?.as_str()),
                        import_message: row.get(9)?,
                        scheme_count: row.get(10)?,
                    })
                },
            )
            .optional()?
            .ok_or(StorageError::NotFound {
                entity: "imported_modpack",
                id,
            })
    }

    fn get_scheme(&self, id: i64) -> Result<SchemeRecord, StorageError> {
        self.database
            .connection
            .query_row(
                "SELECT s.id, s.imported_modpack_id, s.name, d.size_x, d.size_y, d.size_z
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

    fn list_schemes_for_modpack(&self, modpack_id: i64) -> Result<Vec<SchemeRecord>, StorageError> {
        let mut statement = self.database.connection.prepare(
            "SELECT s.id, s.imported_modpack_id, s.name, d.size_x, d.size_y, d.size_z
             FROM schemes s
             JOIN scheme_dimensions d ON d.scheme_id = s.id
             WHERE s.imported_modpack_id = ?1
             ORDER BY s.name COLLATE NOCASE, s.id",
        )?;
        let rows = statement.query_map(params![modpack_id], row_to_scheme)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)
    }

    fn require_modpack(&self, id: i64) -> Result<(), StorageError> {
        let exists = self.database.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM imported_modpacks WHERE id = ?1)",
            params![id],
            |row| row.get::<_, bool>(0),
        )?;
        if exists {
            Ok(())
        } else {
            Err(StorageError::NotFound {
                entity: "imported_modpack",
                id,
            })
        }
    }

    fn next_unique_modpack_name(&self, requested_name: &str) -> Result<String, StorageError> {
        self.next_unique_modpack_name_excluding(requested_name, None)
    }

    fn next_unique_modpack_name_excluding(
        &self,
        requested_name: &str,
        excluding_id: Option<i64>,
    ) -> Result<String, StorageError> {
        let base_name = clean_display_name(requested_name, "Imported modpack");
        let mut candidate = base_name.clone();
        let mut suffix = 2;
        while self.modpack_name_exists(&candidate, excluding_id)? {
            candidate = format!("{base_name} ({suffix})");
            suffix += 1;
        }
        Ok(candidate)
    }

    fn modpack_name_exists(
        &self,
        local_name: &str,
        excluding_id: Option<i64>,
    ) -> Result<bool, StorageError> {
        let count = match excluding_id {
            Some(id) => self.database.connection.query_row(
                "SELECT COUNT(*) FROM imported_modpacks WHERE local_name = ?1 AND id != ?2",
                params![local_name, id],
                |row| row.get::<_, i64>(0),
            )?,
            None => self.database.connection.query_row(
                "SELECT COUNT(*) FROM imported_modpacks WHERE local_name = ?1",
                params![local_name],
                |row| row.get::<_, i64>(0),
            )?,
        };
        Ok(count > 0)
    }
}

fn row_to_scheme(row: &rusqlite::Row<'_>) -> rusqlite::Result<SchemeRecord> {
    Ok(SchemeRecord {
        id: row.get(0)?,
        modpack_id: row.get(1)?,
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

fn clean_display_name(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

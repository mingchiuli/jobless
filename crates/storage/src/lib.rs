use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use jobless_config::AppPaths;
use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[];

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("failed to create data directory `{path}`: {source}")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("migration {version} `{name}` failed: {source}")]
    Migration {
        version: i64,
        name: &'static str,
        #[source]
        source: rusqlite::Error,
    },
    #[error("database lock poisoned")]
    LockPoisoned,
}

#[derive(Debug)]
pub struct Database {
    connection: Mutex<Connection>,
    path: PathBuf,
}

impl Database {
    pub fn open_from_paths(paths: &AppPaths) -> Result<Self, StorageError> {
        Self::open(&paths.database_file)
    }

    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| StorageError::CreateDirectory {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let connection = Connection::open(path)?;
        let database = Self {
            connection: Mutex::new(connection),
            path: path.to_path_buf(),
        };
        database.configure()?;
        database.apply_migrations(MIGRATIONS)?;
        Ok(database)
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let database = Self {
            connection: Mutex::new(Connection::open_in_memory()?),
            path: PathBuf::from(":memory:"),
        };
        database.configure()?;
        database.apply_migrations(MIGRATIONS)?;
        Ok(database)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn connection(&self) -> Result<MutexGuard<'_, Connection>, StorageError> {
        self.connection
            .lock()
            .map_err(|_| StorageError::LockPoisoned)
    }

    fn configure(&self) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::LockPoisoned)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        if self.path != Path::new(":memory:") {
            connection.pragma_update(None, "journal_mode", "WAL")?;
        }
        Ok(())
    }

    fn apply_migrations(&self, migrations: &[Migration]) -> Result<(), StorageError> {
        let mut connection = self.connection()?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )?;

        for migration in migrations {
            let applied = connection
                .query_row(
                    "SELECT 1 FROM schema_migrations WHERE version = ?1",
                    [migration.version],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if applied {
                continue;
            }

            let transaction = connection.transaction()?;
            transaction
                .execute_batch(migration.sql)
                .map_err(|source| StorageError::Migration {
                    version: migration.version,
                    name: migration.name,
                    source,
                })?;
            transaction
                .execute(
                    "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
                    params![migration.version, migration.name],
                )
                .map_err(|source| StorageError::Migration {
                    version: migration.version,
                    name: migration.name,
                    source,
                })?;
            transaction
                .commit()
                .map_err(|source| StorageError::Migration {
                    version: migration.version,
                    name: migration.name,
                    source,
                })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configures_foreign_keys_and_wal() {
        let root = tempfile::tempdir().unwrap();
        let database = Database::open(&root.path().join("db/jobless.sqlite3")).unwrap();
        let connection = database.connection().unwrap();

        let foreign_keys: i64 = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        let journal_mode: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();

        assert_eq!(foreign_keys, 1);
        assert_eq!(journal_mode.to_lowercase(), "wal");
    }

    #[test]
    fn opens_in_memory_without_touching_disk() {
        let database = Database::open_in_memory().unwrap();
        assert_eq!(database.path(), Path::new(":memory:"));
    }

    #[test]
    fn applies_each_migration_once() {
        let database = Database::open_in_memory().unwrap();
        let migrations = [Migration {
            version: 1,
            name: "create_test_table",
            sql: "CREATE TABLE test_migration (id INTEGER PRIMARY KEY);",
        }];

        database.apply_migrations(&migrations).unwrap();
        database.apply_migrations(&migrations).unwrap();

        let count: i64 = database
            .connection()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }
}

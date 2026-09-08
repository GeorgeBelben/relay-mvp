use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

/// A fresh, migrated SQLite DB backed by a throwaway file. The `TempDir` must be
/// kept alive alongside the pool -- dropping it deletes the file out from under
/// any open connections.
pub async fn throwaway_pool() -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let db_path = dir.path().join("test.db");

    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .expect("failed to connect to throwaway sqlite file");

    // Schema now lives in relay-core, which this crate no longer duplicates.
    sqlx::migrate!("../../relay-core/migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    (pool, dir)
}

use thiserror::Error;

#[derive(Debug)]
pub struct Report {
    pub data_dir: std::path::PathBuf,
    pub migrations_run: u32,
    pub pool: sqlx::SqlitePool,
}

#[derive(Debug, Error)]
pub enum InitError {
    #[error("could not determine a data directory for this platform")]
    NoDataDir,

    #[error("couldn't set up the data directory")]
    Io(#[from] std::io::Error),

    #[error("failed to run database migrations")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error("database setup failed")]
    Database(#[from] sqlx::Error),
}

pub async fn ensure_environment() -> Result<Report, InitError> {
    let data_dir = dirs::data_dir().ok_or(InitError::NoDataDir)?.join("relay");

    tokio::fs::create_dir_all(&data_dir).await?;

    let db_path = data_dir.join("relay.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let pool = sqlx::SqlitePool::connect(&db_url).await?;

    let migrator = sqlx::migrate!("./migrations");
    migrator.run(&pool).await?;

    let library_root = crate::library::library_root();
    crate::library::ensure_library_dirs(&library_root).await?;

    Ok(Report {
        data_dir,
        migrations_run: migrator.migrations.len() as u32,
        pool,
    })
}

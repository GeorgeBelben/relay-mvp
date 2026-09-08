//! Pure filesystem browsing of a game's media folder -- backs the drawer's "Artwork" view. Unlike
//! `game_actions`'s reidentify flow, nothing here ever touches the network: every scanned game
//! already has a folder on disk (see `ingestion::pipeline::upsert_rom_and_game`), a user can drop
//! any image into it directly (SFTP/USB/file manager), and this just lists what's there and lets
//! one already-present file be picked as the active box art.

use std::path::Path;

use sqlx::SqlitePool;
use thiserror::Error;

use crate::db::{game_media, games};
use crate::library::{self, IMAGE_EXTENSIONS};

#[derive(Debug, Error)]
pub enum GameMediaFilesError {
    #[error("game not found")]
    GameNotFound,
    #[error("invalid filename")]
    InvalidFilename,
    #[error("file not found in this game's media folder")]
    FileNotFound,
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

/// Filenames only, alphabetical -- a missing directory (a game scanned before this feature
/// shipped, and never since rescanned or reidentified) is just an empty list, not an error.
pub async fn list_media_files(pool: &SqlitePool, media_root: &Path, game_id: &str) -> Result<Vec<String>, GameMediaFilesError> {
    let system_id = games::system_id_for_game(pool, game_id).await?.ok_or(GameMediaFilesError::GameNotFound)?;
    let dir = library::game_media_dir(media_root, &system_id, game_id);

    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(dir) => dir,
        Err(_) => return Ok(Vec::new()),
    };

    let mut names = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let is_file = entry.file_type().await.map(|t| t.is_file()).unwrap_or(false);
        if !is_file {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let extension = name.rsplit('.').next().unwrap_or("").to_lowercase();
        if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
            names.push(name);
        }
    }

    names.sort();
    Ok(names)
}

/// Makes an already-present file in the game's media folder the active box art -- no download, no
/// network call, just a DB row pointing at a file that's already there (`source_url: None`, since
/// it has no remote origin -- see `db::game_media::upsert_boxart`). `filename` is the one place
/// frontend-supplied text reaches a filesystem join in this module, so it's rejected outright if
/// it could escape the game's own media directory, and the file's existence is checked before the
/// DB is ever touched.
pub async fn select_boxart_file(pool: &SqlitePool, media_root: &Path, game_id: &str, filename: &str) -> Result<(), GameMediaFilesError> {
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err(GameMediaFilesError::InvalidFilename);
    }

    let system_id = games::system_id_for_game(pool, game_id).await?.ok_or(GameMediaFilesError::GameNotFound)?;
    let dir = library::game_media_dir(media_root, &system_id, game_id);
    let file_path = dir.join(filename);

    if tokio::fs::metadata(&file_path).await.is_err() {
        return Err(GameMediaFilesError::FileNotFound);
    }

    let local_path = library::to_forward_slash(file_path.strip_prefix(media_root).unwrap_or(&file_path));
    game_media::upsert_boxart(pool, game_id, &local_path, None).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::roms::{self, NewRom};
    use std::fs;

    async fn throwaway_pool() -> (SqlitePool, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new().filename(&db_path).create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new().connect_with(options).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        (pool, dir)
    }

    async fn seed_game(pool: &SqlitePool, system_id: &str) -> String {
        let rom = roms::create(pool, NewRom { system_id: system_id.into(), path: format!("{system_id}/game.bin"), crc32: None, size_bytes: None, discs: None })
            .await
            .unwrap();
        let game = games::create(pool, games::NewGame { rom_id: rom.id, title: "Some Game".into() }).await.unwrap();
        game.id
    }

    #[tokio::test]
    async fn list_media_files_returns_empty_for_a_missing_directory() {
        let (pool, _dir) = throwaway_pool().await;
        let game_id = seed_game(&pool, "snes").await;
        let media_root = tempfile::tempdir().unwrap();

        let names = list_media_files(&pool, media_root.path(), &game_id).await.unwrap();
        assert!(names.is_empty());
    }

    #[tokio::test]
    async fn list_media_files_filters_by_image_extension_and_sorts() {
        let (pool, _dir) = throwaway_pool().await;
        let game_id = seed_game(&pool, "snes").await;
        let media_root = tempfile::tempdir().unwrap();

        let dir = media_root.path().join("snes").join(&game_id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("cover.PNG"), b"").unwrap();
        fs::write(dir.join("alt.jpg"), b"").unwrap();
        fs::write(dir.join("notes.txt"), b"").unwrap();

        let names = list_media_files(&pool, media_root.path(), &game_id).await.unwrap();
        assert_eq!(names, vec!["alt.jpg", "cover.PNG"]);
    }

    #[tokio::test]
    async fn list_media_files_returns_game_not_found_for_an_unknown_id() {
        let (pool, _dir) = throwaway_pool().await;
        let media_root = tempfile::tempdir().unwrap();

        let err = list_media_files(&pool, media_root.path(), "nope").await.unwrap_err();
        assert!(matches!(err, GameMediaFilesError::GameNotFound));
    }

    #[tokio::test]
    async fn select_boxart_file_points_the_active_boxart_at_an_existing_file() {
        let (pool, _dir) = throwaway_pool().await;
        let game_id = seed_game(&pool, "snes").await;
        let media_root = tempfile::tempdir().unwrap();

        let dir = media_root.path().join("snes").join(&game_id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("custom.png"), b"fake-bytes").unwrap();

        select_boxart_file(&pool, media_root.path(), &game_id, "custom.png").await.unwrap();

        let media = game_media::list_for_game(&pool, &game_id).await.unwrap();
        assert_eq!(media.len(), 1);
        assert_eq!(media[0].local_path, format!("snes/{game_id}/custom.png"));
        assert_eq!(media[0].source_url, None);
    }

    #[tokio::test]
    async fn select_boxart_file_rejects_a_path_traversal_filename() {
        let (pool, _dir) = throwaway_pool().await;
        let game_id = seed_game(&pool, "snes").await;
        let media_root = tempfile::tempdir().unwrap();

        let err = select_boxart_file(&pool, media_root.path(), &game_id, "../../etc/passwd").await.unwrap_err();
        assert!(matches!(err, GameMediaFilesError::InvalidFilename));
    }

    #[tokio::test]
    async fn select_boxart_file_returns_file_not_found_for_a_nonexistent_file() {
        let (pool, _dir) = throwaway_pool().await;
        let game_id = seed_game(&pool, "snes").await;
        let media_root = tempfile::tempdir().unwrap();
        fs::create_dir_all(media_root.path().join("snes").join(&game_id)).unwrap();

        let err = select_boxart_file(&pool, media_root.path(), &game_id, "nope.png").await.unwrap_err();
        assert!(matches!(err, GameMediaFilesError::FileNotFound));
    }
}

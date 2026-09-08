use sqlx::SqlitePool;
use tauri::State;

use crate::db::game_media::{self, GameMedia};
use crate::game_media_files;
use crate::ingestion::paths;

#[tauri::command]
pub async fn list_game_media(pool: State<'_, SqlitePool>, game_id: String) -> Result<Vec<GameMedia>, String> {
    game_media::list_for_game(pool.inner(), &game_id).await.map_err(crate::logging::err_to_string)
}

/// Backs the drawer's "Artwork" view -- every image file currently sitting in this game's media
/// folder, whether it got there via auto-enrich, a reidentify, or the user dropping it in
/// themselves. Filenames only; the frontend resolves each into a loadable URL itself (same as
/// `get_media_root_path`'s own doc comment).
#[tauri::command]
pub async fn list_game_media_files(pool: State<'_, SqlitePool>, game_id: String) -> Result<Vec<String>, String> {
    game_media_files::list_media_files(pool.inner(), &paths::media_path(), &game_id).await.map_err(crate::logging::err_to_string)
}

/// Makes an already-present file in the game's media folder the active box art -- no download, no
/// network call (see `game_media_files::select_boxart_file`).
#[tauri::command]
pub async fn select_boxart_file(pool: State<'_, SqlitePool>, game_id: String, filename: String) -> Result<(), String> {
    game_media_files::select_boxart_file(pool.inner(), &paths::media_path(), &game_id, &filename)
        .await
        .map_err(crate::logging::err_to_string)
}

/// `game_media::local_path` is stored relative to this root (forward-slash-normalized, see
/// ingestion::enrich) -- the frontend needs it to reconstruct an absolute path before deciding how
/// to load the image (asset-protocol scope, a byte-serving command, etc -- a frontend-loading
/// decision, not something this command makes for it).
#[tauri::command]
pub fn get_media_root_path() -> String {
    paths::media_path().to_string_lossy().into_owned()
}

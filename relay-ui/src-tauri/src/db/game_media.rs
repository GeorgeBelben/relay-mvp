use serde::Serialize;
use sqlx::SqlitePool;

use super::time::now_unix;

#[derive(Debug, Serialize)]
pub struct GameMedia {
    pub id: String,
    pub game_id: String,
    pub kind: String,
    pub local_path: String,
    pub source_url: Option<String>,
    pub created_at: i64,
}

pub struct NewGameMedia {
    pub game_id: String,
    pub kind: String,
    pub local_path: String,
    pub source_url: Option<String>,
}

pub async fn list_for_game(pool: &SqlitePool, game_id: &str) -> Result<Vec<GameMedia>, sqlx::Error> {
    sqlx::query_as!(
        GameMedia,
        r#"SELECT id, game_id, kind, local_path, source_url, created_at as "created_at!: i64"
           FROM game_media WHERE game_id = ?"#,
        game_id
    )
    .fetch_all(pool)
    .await
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<GameMedia>, sqlx::Error> {
    sqlx::query_as!(
        GameMedia,
        r#"SELECT id, game_id, kind, local_path, source_url, created_at as "created_at!: i64"
           FROM game_media WHERE id = ?"#,
        id
    )
    .fetch_optional(pool)
    .await
}

pub async fn create(pool: &SqlitePool, new: NewGameMedia) -> Result<GameMedia, sqlx::Error> {
    let id = nanoid::nanoid!();
    let now = now_unix();
    sqlx::query_as!(
        GameMedia,
        r#"INSERT INTO game_media (id, game_id, kind, local_path, source_url, created_at)
           VALUES (?, ?, ?, ?, ?, ?)
           RETURNING id, game_id, kind, local_path, source_url, created_at as "created_at!: i64""#,
        id,
        new.game_id,
        new.kind,
        new.local_path,
        new.source_url,
        now,
    )
    .fetch_one(pool)
    .await
}

/// Insert-or-replace keyed on the table's `(game_id, kind)` unique constraint -- unlike `create`
/// above (a plain insert, correct for the automatic enrichment pipeline's one-shot-per-game call
/// site), this is what applying a *different* box art to an already-enriched game needs: replace
/// the existing boxart row, don't collide with the unique constraint or leave the old row
/// orphaned. Used both by `game_actions::apply_reidentify` (a downloaded SteamGridDB image, so
/// `source_url` is `Some`) and `game_media_files::select_boxart_file` (a file the user dropped
/// into the media folder directly, with no remote origin, so `source_url` is `None`). Ported from
/// the Electron MVP's `gameMediaRepository.upsertBoxart`.
pub async fn upsert_boxart(pool: &SqlitePool, game_id: &str, local_path: &str, source_url: Option<&str>) -> Result<GameMedia, sqlx::Error> {
    let id = nanoid::nanoid!();
    let now = now_unix();
    sqlx::query_as!(
        GameMedia,
        r#"INSERT INTO game_media (id, game_id, kind, local_path, source_url, created_at)
           VALUES (?, ?, 'boxart', ?, ?, ?)
           ON CONFLICT (game_id, kind) DO UPDATE SET
             local_path = excluded.local_path,
             source_url = excluded.source_url,
             created_at = excluded.created_at
           RETURNING id, game_id, kind, local_path, source_url, created_at as "created_at!: i64""#,
        id,
        game_id,
        local_path,
        source_url,
        now,
    )
    .fetch_one(pool)
    .await
}

/// Same insert-or-replace shape as `upsert_boxart`, just keyed on the `"backdrop"` kind instead --
/// see that function's doc comment for why this is a separate literal-SQL function rather than a
/// generic `kind`-parameterized one (this codebase's existing convention, and `sqlx::query_as!`'s
/// compile-time checking wants the kind spelled out in the SQL itself).
pub async fn upsert_backdrop(pool: &SqlitePool, game_id: &str, local_path: &str, source_url: Option<&str>) -> Result<GameMedia, sqlx::Error> {
    let id = nanoid::nanoid!();
    let now = now_unix();
    sqlx::query_as!(
        GameMedia,
        r#"INSERT INTO game_media (id, game_id, kind, local_path, source_url, created_at)
           VALUES (?, ?, 'backdrop', ?, ?, ?)
           ON CONFLICT (game_id, kind) DO UPDATE SET
             local_path = excluded.local_path,
             source_url = excluded.source_url,
             created_at = excluded.created_at
           RETURNING id, game_id, kind, local_path, source_url, created_at as "created_at!: i64""#,
        id,
        game_id,
        local_path,
        source_url,
        now,
    )
    .fetch_one(pool)
    .await
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM game_media WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(())
}

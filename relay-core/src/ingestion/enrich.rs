use std::path::Path;

use sqlx::SqlitePool;
use thiserror::Error;

use crate::db::time::now_unix;
use crate::db::{game_media, games};
use crate::library;

use super::identify::matching::best_match;
use super::identify::steamgriddb::{SteamGridDbClient, SteamGridDbError};

// A title has to be a very close match to get auto-applied -- a wrong cover is worse than no
// cover, and titles this close together are effectively always the right game. Ported from the
// Electron MVP's metadata/enrichLibrary.ts.
const MATCH_THRESHOLD: f64 = 0.82;

#[derive(Debug, PartialEq)]
pub enum EnrichOutcome {
    Matched { steamgriddb_id: i64, confidence: f64, boxart_downloaded: bool, backdrop_downloaded: bool },
    NoMatch,
}

#[derive(Debug, Error)]
pub enum EnrichError {
    #[error("SteamGridDB error: {0}")]
    SteamGridDb(#[from] SteamGridDbError),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("box art download error: {0}")]
    Download(#[from] reqwest::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
}

/// Identifies and enriches exactly one game: searches SteamGridDB, applies the match threshold,
/// writes the outcome back to the DB, and -- on a match -- downloads box art and a backdrop under
/// `media_root/<system_id>/<game_id>/`. Ported from the Electron MVP's
/// `enrichLibrary.ts#enrichOne` (backdrop downloading is new in this rewrite -- the Electron MVP
/// only ever fetched box art); the multi-game loop with rate-limiting and progress reporting is a
/// separate concern layered on top of this. `media_root` is threaded through explicitly (rather
/// than resolved internally) so tests can point it at a tempdir -- production call sites pass
/// `library::media_path()`.
pub async fn enrich_one(
    client: &SteamGridDbClient,
    http: &reqwest::Client,
    pool: &SqlitePool,
    game: &games::UnenrichedGame,
    media_root: &Path,
) -> Result<EnrichOutcome, EnrichError> {
    let candidates = client.search_games(&game.title).await?;
    let matched = best_match(&game.title, &candidates, |c| c.name.as_str(), MATCH_THRESHOLD)
        .filter(|m| m.score >= MATCH_THRESHOLD);

    let Some(matched) = matched else {
        games::mark_no_match(pool, &game.id).await?;
        return Ok(EnrichOutcome::NoMatch);
    };

    let steamgriddb_id = matched.candidate.id;
    let matched_title = matched.candidate.name.clone();
    let score = matched.score;

    games::mark_matched(pool, &game.id, steamgriddb_id, &matched_title, score).await?;

    let dest_dir = library::game_media_dir(media_root, &game.system_id, &game.id);

    let boxart_downloaded = match client.get_boxart_url(steamgriddb_id).await? {
        Some(boxart_url) => {
            let local_path = download_media_file(http, &boxart_url, &dest_dir, media_root, "boxart").await?;
            game_media::create(
                pool,
                game_media::NewGameMedia {
                    game_id: game.id.clone(),
                    kind: "boxart".to_string(),
                    local_path,
                    source_url: Some(boxart_url),
                },
            )
            .await?;
            true
        }
        None => false,
    };

    let backdrop_downloaded = match client.get_backdrop_url(steamgriddb_id).await? {
        Some(backdrop_url) => {
            let local_path = download_media_file(http, &backdrop_url, &dest_dir, media_root, "backdrop").await?;
            game_media::create(
                pool,
                game_media::NewGameMedia {
                    game_id: game.id.clone(),
                    kind: "backdrop".to_string(),
                    local_path,
                    source_url: Some(backdrop_url),
                },
            )
            .await?;
            true
        }
        None => false,
    };

    Ok(EnrichOutcome::Matched { steamgriddb_id, confidence: score, boxart_downloaded, backdrop_downloaded })
}

/// Downloads `url` into `dest_dir` as `<filename_prefix>-<unix-timestamp><ext>` and returns the
/// path stored in the database, relative to `media_root` and forward-slash-normalized -- it's read
/// back into a URL later, not passed straight to the filesystem. The timestamped filename matters:
/// a re-download that reused a fixed name would leave an `<img>`'s `src` unchanged across a swap,
/// and browsers only refetch an image when `src` actually changes. `filename_prefix` is what
/// distinguishes a game's box art from its backdrop within the same directory (`"boxart"` /
/// `"backdrop"`) -- otherwise identical download logic for both.
///
/// `pub(crate)` rather than private: `game_actions::apply_reidentify` (a manual re-match, not
/// part of the automatic enrichment pipeline) reuses this rather than duplicating the download
/// logic.
pub(crate) async fn download_media_file(
    http: &reqwest::Client,
    url: &str,
    dest_dir: &Path,
    media_root: &Path,
    filename_prefix: &str,
) -> Result<String, EnrichError> {
    let res = http.get(url).send().await?.error_for_status()?;
    let bytes = res.bytes().await?;

    let ext = reqwest::Url::parse(url)
        .ok()
        .and_then(|u| Path::new(u.path()).extension().map(|e| e.to_string_lossy().into_owned()))
        .map(|e| format!(".{e}"))
        .unwrap_or_else(|| ".png".to_string());

    tokio::fs::create_dir_all(dest_dir).await?;
    let dest = dest_dir.join(format!("{filename_prefix}-{}{ext}", now_unix()));
    tokio::fs::write(&dest, &bytes).await?;

    let relative = dest.strip_prefix(media_root).unwrap_or(&dest);
    Ok(library::to_forward_slash(relative))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::games as games_db;
    use crate::db::roms::{self, NewRom};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn throwaway_pool() -> (SqlitePool, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new().filename(&db_path).create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new().connect_with(options).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        (pool, dir)
    }

    async fn seed_unenriched_game(pool: &SqlitePool, title: &str) -> games::UnenrichedGame {
        let rom = roms::create(
            pool,
            NewRom { system_id: "snes".into(), path: "snes/game.sfc".into(), crc32: None, size_bytes: None, discs: None },
        )
        .await
        .unwrap();
        let game = games_db::create(pool, games_db::NewGame { rom_id: rom.id, title: title.into() }).await.unwrap();

        games::UnenrichedGame { id: game.id, title: game.title, system_id: "snes".into() }
    }

    #[tokio::test]
    async fn enrich_one_marks_no_match_when_nothing_clears_the_threshold() {
        let (pool, _dir) = throwaway_pool().await;
        let game = seed_unenriched_game(&pool, "Totally Unmatched Title").await;
        let media_root = tempfile::tempdir().unwrap();

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/search/autocomplete/Totally%20Unmatched%20Title"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true,
                "data": [{ "id": 1, "name": "Something Completely Different" }],
            })))
            .mount(&server)
            .await;

        let client = SteamGridDbClient::with_base_url("test-key", &format!("{}/api/v2", server.uri()));
        let http = reqwest::Client::new();

        let outcome = enrich_one(&client, &http, &pool, &game, media_root.path()).await.unwrap();
        assert_eq!(outcome, EnrichOutcome::NoMatch);

        let stored = games_db::get(&pool, &game.id).await.unwrap().unwrap();
        assert!(stored.enriched_at.is_some());
        assert!(stored.steamgriddb_id.is_none());
    }

    #[tokio::test]
    async fn enrich_one_marks_matched_and_downloads_boxart_and_backdrop() {
        let (pool, _dir) = throwaway_pool().await;
        let game = seed_unenriched_game(&pool, "Chrono Trigger").await;
        let media_root = tempfile::tempdir().unwrap();

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/search/autocomplete/Chrono%20Trigger"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true,
                "data": [{ "id": 99, "name": "Chrono Trigger" }],
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v2/grids/game/99"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true,
                "data": [{ "id": 1, "url": format!("{}/images/boxart.png", server.uri()) }],
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/images/boxart.png"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"fake-png-bytes".to_vec()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v2/heroes/game/99"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true,
                "data": [{ "id": 2, "url": format!("{}/images/backdrop.png", server.uri()) }],
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/images/backdrop.png"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"fake-backdrop-bytes".to_vec()))
            .mount(&server)
            .await;

        let client = SteamGridDbClient::with_base_url("test-key", &format!("{}/api/v2", server.uri()));
        let http = reqwest::Client::new();

        let outcome = enrich_one(&client, &http, &pool, &game, media_root.path()).await.unwrap();
        assert_eq!(
            outcome,
            EnrichOutcome::Matched { steamgriddb_id: 99, confidence: 1.0, boxart_downloaded: true, backdrop_downloaded: true }
        );

        let stored = games_db::get(&pool, &game.id).await.unwrap().unwrap();
        assert_eq!(stored.steamgriddb_id, Some(99));
        assert!(stored.enriched_at.is_some());

        let media = game_media::list_for_game(&pool, &game.id).await.unwrap();
        assert_eq!(media.len(), 2);
        let boxart = media.iter().find(|m| m.kind == "boxart").expect("boxart row");
        let backdrop = media.iter().find(|m| m.kind == "backdrop").expect("backdrop row");
        assert!(boxart.local_path.starts_with("snes/"));
        assert!(backdrop.local_path.starts_with("snes/"));
        assert!(media_root.path().join(&boxart.local_path).exists());
        assert!(media_root.path().join(&backdrop.local_path).exists());
    }
}

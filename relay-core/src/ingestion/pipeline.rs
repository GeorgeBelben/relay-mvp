use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Serialize;
use sqlx::SqlitePool;
use thiserror::Error;

use crate::db::{games, roms};
use crate::library::to_forward_slash;
use crate::scan::{self, ScanTarget};
use crate::systems;
use crate::title::title_from_filename;

use super::enrich;
use super::identify::no_intro::NoIntroDatLookup;
use super::identify::retroachievements::RaHashLookup;
use super::identify::steamgriddb::SteamGridDbClient;
use super::probe;

// Mirrors the Electron MVP's shared/types.ts#ScanStatus exactly (field names included, via
// serde's internal tagging + kebab-case rename) so a frontend event contract stays unchanged.
// `matching-achievements` is omitted -- no RetroAchievements pipeline exists in this rewrite yet.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum ScanStatus {
    Idle,
    ScanningFiles,
    EnrichingArt { current: u32, total: u32 },
    Done,
    Error { message: String },
}

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

#[allow(clippy::too_many_arguments)]
async fn upsert_rom_and_game(
    pool: &SqlitePool,
    system_id: &str,
    path: String,
    crc32: Option<String>,
    md5: Option<String>,
    size_bytes: Option<i64>,
    discs: Option<String>,
    title: &str,
    retroachievements_game_id: Option<i64>,
) -> Result<(), sqlx::Error> {
    let rom = roms::upsert(pool, roms::NewRom { system_id: system_id.to_string(), path, crc32, md5, size_bytes, discs }).await?;
    let game = games::upsert_for_rom(pool, &rom.id, title).await?;

    if let Some(ra_game_id) = retroachievements_game_id {
        games::mark_ra_matched(pool, &game.id, ra_game_id).await?;
    }

    Ok(())
}

/// Scan stage + probe stage tied together: walks every known system's rom folder, hashes what it
/// finds, identifies it against RetroAchievements' hash list (exact MD5 match -- this is what
/// actually links a game and its achievements, not SteamGridDB) and No-Intro's DAT (CRC32 title
/// correction only), and upserts roms/games. Anything not found this pass is marked missing, not
/// deleted. `ra_hashes` is `None` when no RetroAchievements API key is configured, which skips
/// identification entirely (matching how SteamGridDB enrichment is already skipped when unset).
/// Ported from the Electron MVP's `scanLibrary.ts` (its dev-seed branch is deliberately not
/// ported -- that's a dev-only convenience, not a pipeline concern).
pub async fn scan_and_probe(
    pool: &SqlitePool,
    roms_root: &Path,
    no_intro: &NoIntroDatLookup,
    ra_hashes: Option<&RaHashLookup>,
) -> Result<usize, sqlx::Error> {
    let mut found_paths = Vec::new();

    for system in systems::ALL {
        let system_folder = roms_root.join(system.id);
        let targets = scan::walk_system_folder(&system_folder, system.extensions).await;

        for target in targets {
            match target {
                ScanTarget::Single { file_path, title } => {
                    let relative = to_forward_slash(file_path.strip_prefix(roms_root).unwrap_or(&file_path));
                    found_paths.push(relative.clone());

                    let (crc32, md5, size_bytes) = match probe::probe_file(&file_path).await {
                        Ok(p) => (Some(p.crc32), Some(p.md5), Some(p.size_bytes)),
                        Err(_) => (None, None, None),
                    };

                    // An exact CRC32 match against No-Intro's own data beats guessing a title
                    // from whatever the filename happens to look like -- falls back to the
                    // filename-derived title exactly as before on any miss.
                    let title = match &crc32 {
                        Some(crc) => match no_intro.lookup(system.id, crc).await {
                            Some(dat_title) => title_from_filename(&dat_title),
                            None => title,
                        },
                        None => title,
                    };

                    // An exact MD5 match against RetroAchievements' own hash list is even more
                    // authoritative than No-Intro's -- it's not just a title, it's the actual
                    // link a game's achievements hang off of.
                    let ra_match = match (&md5, ra_hashes) {
                        (Some(hash), Some(lookup)) => lookup.lookup(system.id, system.name, hash).await,
                        _ => None,
                    };
                    let ra_game_id = ra_match.as_ref().map(|(id, _)| *id);
                    let title = match ra_match {
                        Some((_, ra_title)) => ra_title,
                        None => title,
                    };

                    upsert_rom_and_game(pool, system.id, relative, crc32, md5, size_bytes, None, &title, ra_game_id).await?;
                }
                ScanTarget::MultiDisc { m3u_path, disc_paths, title } => {
                    let relative = to_forward_slash(m3u_path.strip_prefix(roms_root).unwrap_or(&m3u_path));
                    found_paths.push(relative.clone());

                    let mut discs = Vec::new();
                    for disc_path in &disc_paths {
                        // A disc listed in the .m3u but missing on disk is skipped, not fatal --
                        // matches the MVP's warn-and-continue behavior.
                        if let Ok(p) = probe::probe_file(disc_path).await {
                            discs.push(serde_json::json!({
                                "path": to_forward_slash(disc_path.strip_prefix(roms_root).unwrap_or(disc_path)),
                                "crc32": p.crc32,
                                "sizeBytes": p.size_bytes,
                            }));
                        }
                    }
                    let discs_json = if discs.is_empty() { None } else { serde_json::to_string(&discs).ok() };

                    // RA/No-Intro hash matching doesn't apply to multi-disc sets, same reasoning
                    // as no_intro's disc-based-system exclusion (the row-level crc32/md5 stay
                    // None; per-disc hashes still live in discs_json above).
                    upsert_rom_and_game(pool, system.id, relative, None, None, None, discs_json, &title, None).await?;
                }
            }
        }
    }

    roms::mark_missing(pool, &found_paths).await?;
    Ok(found_paths.len())
}

/// The full "Rescan Library" operation: scan+probe+identify, then fetch art for anything
/// unenriched -- sequential, not fire-and-forget, so it's one honest operation with one status
/// stream rather than a second background process a caller can't reflect. `running` guards
/// against overlapping runs (a second "Rescan" call mid-run is a no-op, not an error) -- callers
/// own the flag's lifetime (the real app manages one alongside the DB pool; tests use a fresh
/// local one, so runs in different tests never see each other's state). `steamgriddb`/`ra_hashes`
/// are `None` when no API key is configured yet, which skips art-fetching/identification
/// respectively (matching the MVP's behavior) -- taking already-constructed collaborators (rather
/// than building them from a key internally) is what lets tests point them at a mock server
/// instead of the real API; `no_intro` follows the same pattern for scan-time title correction.
/// `on_status` is called with each state transition -- production wires it to a UI event; tests
/// just collect it.
#[allow(clippy::too_many_arguments)]
pub async fn rescan(
    pool: &SqlitePool,
    roms_root: &Path,
    no_intro: &NoIntroDatLookup,
    ra_hashes: Option<&RaHashLookup>,
    steamgriddb: Option<&SteamGridDbClient>,
    running: &AtomicBool,
    mut on_status: impl FnMut(ScanStatus),
) -> Result<(), PipelineError> {
    if running.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let result = run_rescan(pool, roms_root, no_intro, ra_hashes, steamgriddb, &mut on_status).await;
    running.store(false, Ordering::SeqCst);

    if let Err(e) = &result {
        on_status(ScanStatus::Error { message: e.to_string() });
    }
    result
}

#[allow(clippy::too_many_arguments)]
async fn run_rescan(
    pool: &SqlitePool,
    roms_root: &Path,
    no_intro: &NoIntroDatLookup,
    ra_hashes: Option<&RaHashLookup>,
    steamgriddb: Option<&SteamGridDbClient>,
    on_status: &mut impl FnMut(ScanStatus),
) -> Result<(), PipelineError> {
    on_status(ScanStatus::ScanningFiles);
    scan_and_probe(pool, roms_root, no_intro, ra_hashes).await?;

    if let Some(client) = steamgriddb {
        let http = reqwest::Client::new();
        let pending = games::list_unenriched(pool).await?;
        let total = pending.len() as u32;
        on_status(ScanStatus::EnrichingArt { current: 0, total });

        for (i, game) in pending.iter().enumerate() {
            if let Err(e) = enrich::enrich_one(client, &http, pool, game, roms_root).await {
                eprintln!("enrich: failed to enrich \"{}\" ({}): {e}", game.title, game.id);
            }
            on_status(ScanStatus::EnrichingArt { current: (i + 1) as u32, total });
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }

    on_status(ScanStatus::Done);
    Ok(())
}

/// Convenience wrapper around `rescan` that resolves its optional collaborators (a SteamGridDB
/// client, the No-Intro DAT cache, a RetroAchievements hash lookup) from Settings/cache dirs,
/// rather than requiring every caller to do that resolution itself. This is the piece relay-ui's
/// Tauri `rescan_library` command used to do inline (`rescan_library_at`), redesigned to take
/// plain parameters and a callback instead of an AppHandle/Emitter/State -- a Tauri command wraps
/// `on_status` in `app.emit` and stores `running`/status in Tauri state; a CLI invocation can just
/// print it and use a fresh `running`.
pub async fn rescan_from_settings(
    pool: &SqlitePool,
    roms_root: &Path,
    dats_cache_dir: &Path,
    ra_cache_dir: &Path,
    running: &AtomicBool,
    on_status: impl FnMut(ScanStatus),
) -> Result<(), PipelineError> {
    let steamgriddb_key = crate::db::settings::get(pool, "steamgriddbApiKey").await?;
    let client = steamgriddb_key.map(SteamGridDbClient::new);
    let no_intro = NoIntroDatLookup::new(dats_cache_dir.to_path_buf());

    let ra_key = crate::db::settings::get(pool, "retroachievementsApiKey").await?;
    let ra_hashes = ra_key.map(|key| RaHashLookup::new(key, ra_cache_dir.to_path_buf()));

    rescan(pool, roms_root, &no_intro, ra_hashes.as_ref(), client.as_ref(), running, on_status).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::game_media;
    use std::fs;
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

    // Points at an unreachable address (nothing listens on 127.0.0.1:1) so a lookup fails fast
    // with no real network call and no cache file -- these tests don't care about DAT lookup
    // behavior (that's no_intro.rs's job), just that titles stay filename-derived without it.
    fn no_intro_stub() -> NoIntroDatLookup {
        NoIntroDatLookup::with_base_url(tempfile::tempdir().unwrap().keep(), "http://127.0.0.1:1/")
    }

    #[tokio::test]
    async fn scan_and_probe_creates_roms_and_games_then_marks_vanished_files_missing() {
        let (pool, _db_dir) = throwaway_pool().await;

        let roms_root = tempfile::tempdir().unwrap();
        let snes_dir = roms_root.path().join("snes");
        fs::create_dir(&snes_dir).unwrap();
        fs::write(snes_dir.join("Chrono Trigger (USA).sfc"), b"fake-rom-bytes").unwrap();
        let no_intro = no_intro_stub();

        let found = scan_and_probe(&pool, roms_root.path(), &no_intro, None).await.unwrap();
        assert_eq!(found, 1);

        let all_roms = roms::list(&pool).await.unwrap();
        assert_eq!(all_roms.len(), 1);
        assert_eq!(all_roms[0].status, "ok");
        assert!(all_roms[0].crc32.is_some());
        assert!(all_roms[0].md5.is_some());

        let all_games = games::list(&pool).await.unwrap();
        assert_eq!(all_games.len(), 1);
        assert_eq!(all_games[0].title, "Chrono Trigger");
        assert!(all_games[0].retroachievements_game_id.is_none()); // no RA lookup configured

        // Rescanning after the file's gone marks the rom missing rather than deleting it.
        fs::remove_file(snes_dir.join("Chrono Trigger (USA).sfc")).unwrap();
        let found_again = scan_and_probe(&pool, roms_root.path(), &no_intro, None).await.unwrap();
        assert_eq!(found_again, 0);

        let all_roms = roms::list(&pool).await.unwrap();
        assert_eq!(all_roms[0].status, "missing");
    }

    #[tokio::test]
    async fn scan_and_probe_identifies_via_retroachievements_hash_match() {
        let (pool, _db_dir) = throwaway_pool().await;

        let roms_root = tempfile::tempdir().unwrap();
        fs::create_dir(roms_root.path().join("nes")).unwrap();
        fs::write(roms_root.path().join("nes/game.nes"), b"fake-rom-bytes").unwrap();
        let no_intro = no_intro_stub();

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(wiremock::matchers::query_param("g", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "ID": 7, "Name": "NES/Famicom", "IconURL": "", "Active": true, "IsGameSystem": true },
            ])))
            .mount(&server)
            .await;
        // MD5 of b"fake-rom-bytes", computed once via a throwaway probe rather than hand-copied,
        // so this test can't silently drift if probe_file's hashing ever changed.
        let expected_md5 = super::super::probe::probe_file(&roms_root.path().join("nes/game.nes")).await.unwrap().md5;
        Mock::given(method("GET"))
            .and(wiremock::matchers::query_param("i", "7"))
            .and(wiremock::matchers::query_param("h", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "ID": 555, "Title": "The Real Game Title", "Hashes": [expected_md5] },
            ])))
            .mount(&server)
            .await;

        let ra_hashes = RaHashLookup::with_base_url("test-key", tempfile::tempdir().unwrap().keep(), &server.uri());

        scan_and_probe(&pool, roms_root.path(), &no_intro, Some(&ra_hashes)).await.unwrap();

        let game = &games::list(&pool).await.unwrap()[0];
        assert_eq!(game.title, "The Real Game Title");
        assert_eq!(game.retroachievements_game_id, Some(555));
        assert!(game.retroachievements_matched_at.is_some());
    }

    #[tokio::test]
    async fn rescan_without_api_key_skips_enrichment() {
        let (pool, _db_dir) = throwaway_pool().await;

        let roms_root = tempfile::tempdir().unwrap();
        fs::create_dir(roms_root.path().join("snes")).unwrap();
        fs::write(roms_root.path().join("snes/game.sfc"), b"data").unwrap();

        let no_intro = no_intro_stub();
        let mut statuses = Vec::new();
        let running = AtomicBool::new(false);
        rescan(&pool, roms_root.path(), &no_intro, None, None, &running, |s| statuses.push(s)).await.unwrap();

        assert_eq!(statuses, vec![ScanStatus::ScanningFiles, ScanStatus::Done]);

        let game = &games::list(&pool).await.unwrap()[0];
        assert!(game.enriched_at.is_none());
    }

    #[tokio::test]
    async fn rescan_with_api_key_enriches_and_downloads_boxart_colocated_with_the_rom() {
        let (pool, _db_dir) = throwaway_pool().await;

        let roms_root = tempfile::tempdir().unwrap();
        fs::create_dir(roms_root.path().join("snes")).unwrap();
        fs::write(roms_root.path().join("snes/Chrono Trigger.sfc"), b"data").unwrap();

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v2/search/autocomplete/Chrono%20Trigger"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true,
                "data": [{ "id": 7, "name": "Chrono Trigger" }],
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v2/grids/game/7"))
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

        let client = SteamGridDbClient::with_base_url("test-key", &format!("{}/api/v2", server.uri()));
        let no_intro = no_intro_stub();
        let mut statuses = Vec::new();
        let running = AtomicBool::new(false);
        rescan(&pool, roms_root.path(), &no_intro, None, Some(&client), &running, |s| statuses.push(s)).await.unwrap();

        assert_eq!(statuses[0], ScanStatus::ScanningFiles);
        assert_eq!(statuses[1], ScanStatus::EnrichingArt { current: 0, total: 1 });
        assert_eq!(statuses[2], ScanStatus::EnrichingArt { current: 1, total: 1 });
        assert_eq!(statuses[3], ScanStatus::Done);

        let game = &games::list(&pool).await.unwrap()[0];
        assert_eq!(game.steamgriddb_id, Some(7));

        // Colocated: the boxart lands as a sibling of the loose rom file, not in a separate
        // media/ tree, prefixed with the rom's own stem.
        let media = game_media::list_for_game(&pool, &game.id).await.unwrap();
        assert_eq!(media.len(), 1);
        assert!(media[0].local_path.starts_with("snes/Chrono Trigger."));
        assert!(roms_root.path().join(&media[0].local_path).exists());
    }

    #[tokio::test]
    async fn rescan_is_a_no_op_while_already_running() {
        let (pool, _db_dir) = throwaway_pool().await;

        let roms_root = tempfile::tempdir().unwrap();
        fs::create_dir(roms_root.path().join("snes")).unwrap();
        fs::write(roms_root.path().join("snes/game.sfc"), b"data").unwrap();

        let no_intro = no_intro_stub();
        let running = AtomicBool::new(true); // simulate an in-flight run
        let mut statuses = Vec::new();
        rescan(&pool, roms_root.path(), &no_intro, None, None, &running, |s| statuses.push(s)).await.unwrap();

        assert!(statuses.is_empty());
        assert!(games::list(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn rescan_from_settings_resolves_collaborators_and_reports_progress() {
        let (pool, _db_dir) = throwaway_pool().await;

        let roms_root = tempfile::tempdir().unwrap();
        fs::create_dir(roms_root.path().join("snes")).unwrap();
        fs::write(roms_root.path().join("snes/game.sfc"), b"data").unwrap();
        let dats_cache_dir = tempfile::tempdir().unwrap();
        let ra_cache_dir = tempfile::tempdir().unwrap();

        // No steamgriddbApiKey/retroachievementsApiKey settings configured -- resolves to no
        // client/lookup, enrichment and identification both skipped.
        let running = AtomicBool::new(false);
        let mut statuses = Vec::new();
        rescan_from_settings(&pool, roms_root.path(), dats_cache_dir.path(), ra_cache_dir.path(), &running, |s| statuses.push(s))
            .await
            .unwrap();

        assert_eq!(statuses, vec![ScanStatus::ScanningFiles, ScanStatus::Done]);
        let game = &games::list(&pool).await.unwrap()[0];
        assert!(game.enriched_at.is_none());
        assert!(game.retroachievements_game_id.is_none());
    }
}

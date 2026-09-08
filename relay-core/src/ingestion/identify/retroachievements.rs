//! ROM-hash (MD5) -> RetroAchievements game ID matching. Verified against the real RA API docs
//! (api-docs.retroachievements.org): `API_GetConsoleIDs.php` gives RA's own numeric console ids,
//! `API_GetGameList.php?h=1` gives every game + its known MD5 hashes for one console. Mirrors
//! `identify::no_intro`'s fetch-once-cache-forever shape.
//!
//! RA's console ids don't line up with Relay's own system ids, and there's no verified mapping
//! table between them (that would need a real API key to check against live data, which this
//! codebase doesn't have yet) -- so instead of hardcoding guessed ids/names, a system is resolved
//! to its RA console by exact whole-word matching: `SystemDef::name` (e.g. "SNES", a short label)
//! against each RA console name (e.g. "SNES/Super Famicom") split into words. Deliberately NOT
//! the Jaro-Winkler fuzzy matcher `identify::matching` uses for SteamGridDB titles -- tried that
//! first, and it badly underscores a short label against a much longer name purely from the
//! length disparity (Jaro-Winkler is built for near-equal-length typo tolerance, not
//! prefix/abbreviation containment). Whole-word matching also avoids a real collision a naive
//! substring check would hit: "NES" is literally a substring of "Genesis", but never a whole word
//! in "Sega Genesis/Mega Drive". A system whose name matches no console's words just never gets
//! RA-matched -- same "no coverage" shape as no_intro's disc-based-system exclusion. This
//! resolution strategy itself is unverified against real RA console names; confirm it once a real
//! API key is available.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::retroachievements::client::{RaConsole, RetroAchievementsClient};

use super::matching::normalize_for_match;

#[derive(Serialize, Deserialize, Clone)]
struct HashEntry {
    game_id: i64,
    title: String,
}

pub struct RaHashLookup {
    client: RetroAchievementsClient,
    cache_dir: PathBuf,
    consoles: Mutex<Option<Vec<RaConsole>>>,
    resolved: Mutex<HashMap<String, i64>>,
    hashes: Mutex<HashMap<i64, HashMap<String, HashEntry>>>,
}

impl RaHashLookup {
    pub fn new(api_key: impl Into<String>, cache_dir: PathBuf) -> Self {
        Self {
            client: RetroAchievementsClient::new(api_key),
            cache_dir,
            consoles: Mutex::new(None),
            resolved: Mutex::new(HashMap::new()),
            hashes: Mutex::new(HashMap::new()),
        }
    }

    /// Exposed so tests can point the client at a local mock server instead of the real API.
    pub fn with_base_url(api_key: impl Into<String>, cache_dir: PathBuf, base_url: &str) -> Self {
        Self {
            client: RetroAchievementsClient::with_base_url(api_key, base_url),
            cache_dir,
            consoles: Mutex::new(None),
            resolved: Mutex::new(HashMap::new()),
            hashes: Mutex::new(HashMap::new()),
        }
    }

    /// `None` means "couldn't identify" (no RA console match for this system, an API error, or
    /// this exact dump isn't in RA's hash set) -- callers keep whatever title they already had
    /// either way, same shape as `no_intro::lookup`.
    pub async fn lookup(&self, system_id: &str, system_name: &str, md5: &str) -> Option<(i64, String)> {
        let console_id = self.console_id_for(system_id, system_name).await?;
        let map = self.hash_map_for(console_id).await?;
        map.get(&md5.to_uppercase()).map(|entry| (entry.game_id, entry.title.clone()))
    }

    async fn console_id_for(&self, system_id: &str, system_name: &str) -> Option<i64> {
        if let Some(id) = self.resolved.lock().unwrap().get(system_id) {
            return Some(*id);
        }

        let consoles = self.consoles_list().await?;
        // Padding both sides with a space and checking substring containment matches a whole
        // *sequence* of words (needed for multi-word targets like "game boy advance" -- checking
        // per-word equality, as an earlier version of this did, can never match a single word
        // against a 3-word target) while still only matching on real word boundaries, not
        // mid-word: " nes " is never a substring of " sega genesis mega drive " even though the
        // letters "nes" are, since "genesis" has no spaces around just that part.
        let target = format!(" {} ", normalize_for_match(system_name));
        let matched = consoles.iter().find(|c| format!(" {} ", normalize_for_match(&c.name)).contains(&target))?;
        let id = matched.id;

        self.resolved.lock().unwrap().insert(system_id.to_string(), id);
        Some(id)
    }

    async fn consoles_list(&self) -> Option<Vec<RaConsole>> {
        if let Some(cached) = self.consoles.lock().unwrap().as_ref() {
            return Some(cached.clone());
        }

        let fetched = self.client.get_console_ids().await.ok()?;
        *self.consoles.lock().unwrap() = Some(fetched.clone());
        Some(fetched)
    }

    async fn hash_map_for(&self, console_id: i64) -> Option<HashMap<String, HashEntry>> {
        if let Some(cached) = self.hashes.lock().unwrap().get(&console_id) {
            return Some(cached.clone());
        }

        if let Some(from_disk) = self.load_from_disk(console_id).await {
            self.hashes.lock().unwrap().insert(console_id, from_disk.clone());
            return Some(from_disk);
        }

        let games = self.client.get_game_list_with_hashes(console_id).await.ok()?;
        let mut map = HashMap::new();
        for game in games {
            for hash in &game.hashes {
                map.insert(hash.to_uppercase(), HashEntry { game_id: game.id, title: game.title.clone() });
            }
        }

        self.hashes.lock().unwrap().insert(console_id, map.clone());
        if let Err(e) = self.save_to_disk(console_id, &map).await {
            eprintln!("identify: failed to cache RA hash list for console {console_id}: {e}");
        }
        Some(map)
    }

    fn cache_path(&self, console_id: i64) -> PathBuf {
        self.cache_dir.join(format!("ra-{console_id}.json"))
    }

    async fn load_from_disk(&self, console_id: i64) -> Option<HashMap<String, HashEntry>> {
        let raw = tokio::fs::read_to_string(self.cache_path(console_id)).await.ok()?;
        serde_json::from_str(&raw).ok()
    }

    async fn save_to_disk(&self, console_id: i64, map: &HashMap<String, HashEntry>) -> std::io::Result<()> {
        tokio::fs::create_dir_all(&self.cache_dir).await?;
        let json = serde_json::to_string(map).unwrap_or_default();
        tokio::fs::write(self.cache_path(console_id), json).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn client_against(server: &MockServer) -> RetroAchievementsClient {
        RetroAchievementsClient::with_base_url("test-key", &format!("{}/", server.uri()))
    }

    async fn lookup_against(server: &MockServer, cache_dir: &std::path::Path) -> RaHashLookup {
        RaHashLookup {
            client: client_against(server),
            cache_dir: cache_dir.to_path_buf(),
            consoles: Mutex::new(None),
            resolved: Mutex::new(HashMap::new()),
            hashes: Mutex::new(HashMap::new()),
        }
    }

    #[tokio::test]
    async fn lookup_resolves_console_by_whole_word_name_and_finds_a_matching_hash() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("g", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "ID": 3, "Name": "SNES/Super Famicom", "IconURL": "", "Active": true, "IsGameSystem": true },
                { "ID": 7, "Name": "NES/Famicom", "IconURL": "", "Active": true, "IsGameSystem": true },
            ])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(query_param("i", "3"))
            .and(query_param("h", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "ID": 99, "Title": "Chrono Trigger", "Hashes": ["ABCDEF1234567890ABCDEF1234567890"] },
            ])))
            .mount(&server)
            .await;

        let cache_dir = tempfile::tempdir().unwrap();
        let lookup = lookup_against(&server, cache_dir.path()).await;

        // "SNES" is what relay-core's own systems::SystemDef.name actually holds for this system
        // -- a short label, not the console's full name -- matched fuzzily against RA's own
        // "SNES/Super Famicom" naming.
        let result = lookup.lookup("snes", "SNES", "abcdef1234567890abcdef1234567890").await;
        assert_eq!(result, Some((99, "Chrono Trigger".to_string())));
    }

    #[tokio::test]
    async fn lookup_returns_none_when_no_console_name_matches_at_all() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("g", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "ID": 1, "Name": "Mega Drive", "IconURL": "", "Active": true, "IsGameSystem": true },
            ])))
            .mount(&server)
            .await;

        let cache_dir = tempfile::tempdir().unwrap();
        let lookup = lookup_against(&server, cache_dir.path()).await;

        assert_eq!(lookup.lookup("psx", "PlayStation", "ABCDEF1234567890ABCDEF1234567890").await, None);
    }

    #[tokio::test]
    async fn console_resolution_does_not_false_positive_on_nes_being_a_substring_of_genesis() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("g", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "ID": 1, "Name": "Sega Genesis/Mega Drive", "IconURL": "", "Active": true, "IsGameSystem": true },
            ])))
            .mount(&server)
            .await;

        let cache_dir = tempfile::tempdir().unwrap();
        let lookup = lookup_against(&server, cache_dir.path()).await;

        // "NES" is a literal substring of "Genesis" but must never match it as a whole word.
        assert_eq!(lookup.console_id_for("nes", "NES").await, None);
    }

    #[tokio::test]
    async fn console_resolution_matches_a_multi_word_system_name() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("g", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "ID": 5, "Name": "Game Boy Advance", "IconURL": "", "Active": true, "IsGameSystem": true },
                { "ID": 21, "Name": "PlayStation 2", "IconURL": "", "Active": true, "IsGameSystem": true },
            ])))
            .mount(&server)
            .await;

        let cache_dir = tempfile::tempdir().unwrap();
        let lookup = lookup_against(&server, cache_dir.path()).await;

        // Relay's own SystemDef::name values for these systems, verbatim -- a naive per-word
        // equality check (an earlier version of this function) can never match a 3-word target
        // like "game boy advance" against any single word of a console's name.
        assert_eq!(lookup.console_id_for("gba", "Game Boy Advance").await, Some(5));
        assert_eq!(lookup.console_id_for("ps2", "PlayStation 2").await, Some(21));
    }

    #[tokio::test]
    async fn lookup_returns_none_for_a_hash_not_in_the_console_list() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("g", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "ID": 7, "Name": "NES", "IconURL": "", "Active": true, "IsGameSystem": true },
            ])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(query_param("i", "7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "ID": 1, "Title": "Super Mario Bros.", "Hashes": ["1111111111111111111111111111111"] },
            ])))
            .mount(&server)
            .await;

        let cache_dir = tempfile::tempdir().unwrap();
        let lookup = lookup_against(&server, cache_dir.path()).await;

        assert_eq!(lookup.lookup("nes", "NES", "DEADBEEFDEADBEEFDEADBEEFDEADBEEF").await, None);
    }

    #[tokio::test]
    async fn hash_list_is_cached_to_disk_and_reused_across_instances() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("g", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "ID": 7, "Name": "NES", "IconURL": "", "Active": true, "IsGameSystem": true },
            ])))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(query_param("i", "7"))
            .and(query_param("h", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                { "ID": 1, "Title": "Super Mario Bros.", "Hashes": ["AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"] },
            ])))
            .expect(1) // exactly once -- the second lookup below must come from the disk cache
            .mount(&server)
            .await;

        let cache_dir = tempfile::tempdir().unwrap();

        let first = lookup_against(&server, cache_dir.path()).await;
        assert_eq!(first.lookup("nes", "NES", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").await, Some((1, "Super Mario Bros.".to_string())));

        assert!(cache_dir.path().join("ra-7.json").exists());

        let second = lookup_against(&server, cache_dir.path()).await;
        assert_eq!(second.lookup("nes", "NES", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").await, Some((1, "Super Mario Bros.".to_string())));
    }
}

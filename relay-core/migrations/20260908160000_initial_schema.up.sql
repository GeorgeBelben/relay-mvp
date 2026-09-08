-- Squashed from relay-ui's 9 migrations (2026-08-29 through 2026-08-31) into one initial
-- schema for relay-core -- this is the final live shape after all of relay-ui's history, not a
-- replay of it. No production data exists yet to preserve (per relay-ui's own CLAUDE.md: "no
-- legacy DB to preserve, re-ingest fresh").
--
-- systems is deliberately not a table: the set of supported systems is a fixed Rust catalog
-- (relay_core::systems::ALL), not user-editable data. roms.system_id stores one of its ids as a
-- plain string with no DB-level foreign key (SQLite's `foreign_keys` pragma is never enabled, so
-- no FK here would be enforced anyway).

CREATE TABLE roms (
    id TEXT PRIMARY KEY NOT NULL,
    system_id TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    crc32 TEXT,
    size_bytes INTEGER,
    discs TEXT,
    status TEXT NOT NULL DEFAULT 'ok',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE games (
    id TEXT PRIMARY KEY NOT NULL,
    rom_id TEXT NOT NULL UNIQUE REFERENCES roms(id),
    title TEXT NOT NULL,
    scanned_title TEXT,
    steamgriddb_id INTEGER,
    match_confidence REAL,
    enriched_at INTEGER,
    retroachievements_game_id INTEGER,
    retroachievements_matched_at INTEGER,
    ra_highest_award_kind TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE game_media (
    id TEXT PRIMARY KEY NOT NULL,
    game_id TEXT NOT NULL REFERENCES games(id),
    kind TEXT NOT NULL,
    local_path TEXT NOT NULL,
    source_url TEXT,
    created_at INTEGER NOT NULL,
    UNIQUE (game_id, kind)
);

CREATE TABLE profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    ra_username TEXT,
    ra_web_api_key_encrypted TEXT,
    ra_token_encrypted TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE ra_stats (
    profile_id TEXT PRIMARY KEY NOT NULL REFERENCES profiles(id),
    points INTEGER NOT NULL,
    rank TEXT NOT NULL,
    recent_unlocks_json TEXT NOT NULL,
    refreshed_at INTEGER NOT NULL
);

CREATE TABLE settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

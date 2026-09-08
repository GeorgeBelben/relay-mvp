use std::path::{Path, PathBuf};

// ~/Relay -- ported from the Electron MVP's libraryPaths.ts. Only the paths the ingestion
// pipeline actually needs so far (roms + downloaded media) are ported here; the remaining
// subdirectories (bios/saves/savestates/screenshots) are system::storage's concern (it just needs
// this one root, not a helper per subdirectory), not ingestion's.
pub fn library_root() -> PathBuf {
    dirs::home_dir().expect("could not resolve home directory").join("Relay")
}

pub fn roms_path() -> PathBuf {
    library_root().join("roms")
}

// Every scanned ROM's media lives here, one subfolder per game -- created unconditionally at scan
// time (see ingestion::pipeline::upsert_rom_and_game), independent of whether identification ever
// succeeds, so a game's folder always exists for a user to drop images into or for enrich/reidentify
// to seed with downloaded art. `media_root` is accepted as a parameter rather than resolving
// `media_path()` internally so tests can point it at a tempdir instead.
pub fn media_path() -> PathBuf {
    library_root().join("media")
}

pub fn game_media_dir(media_root: &Path, system_id: &str, game_id: &str) -> PathBuf {
    media_root.join(system_id).join(game_id)
}

// Shared with game_media_files' folder listing so the allowlist can't drift between them.
pub const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

pub fn to_forward_slash(path: &Path) -> String {
    path.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

//! `~/Relay` and its subdirectories -- ported/reconciled from relay-ui's
//! `ingestion::paths` (roms_path/media_path/game_media_dir/IMAGE_EXTENSIONS/to_forward_slash)
//! and `system::storage::ensure_library_dirs`.

use std::path::{Path, PathBuf};

pub fn library_root() -> PathBuf {
    dirs::home_dir()
        .expect("could not determine home directory")
        .join("Relay")
}

pub fn roms_path() -> PathBuf {
    library_root().join("roms")
}

// Every scanned ROM's media lives here, one subfolder per game -- created unconditionally at scan
// time, independent of whether identification ever succeeds, so a game's folder always exists for
// a user to drop images into or for enrich/reidentify to seed with downloaded art. `media_root` is
// accepted as a parameter rather than resolving `media_path()` internally so tests can point it at
// a tempdir instead.
pub fn media_path() -> PathBuf {
    library_root().join("media")
}

pub fn game_media_dir(media_root: &Path, system_id: &str, game_id: &str) -> PathBuf {
    media_root.join(system_id).join(game_id)
}

// Shared with game_media_files' folder listing so the allowlist can't drift between them.
pub const IMAGE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

/// Where a scanned ROM's own artwork should live, colocated with the ROM file rather than in a
/// separate media tree. If the ROM already lives in its own folder (a multi-disc set, or one a
/// user organized by hand), art goes inside that same folder -- it's the only game there, so no
/// extra naming is needed. If the ROM is a loose file sitting directly in `roms/<system_id>/`,
/// art becomes a sibling file in that same folder, prefixed with the ROM's own filename stem
/// (plus a trailing `.`) so multiple loose ROMs' art doesn't collide in one flat directory.
///
/// Returns `(directory, filename_prefix)` -- `filename_prefix` is empty for an already-foldered
/// ROM, or `"<rom-stem>."` for a loose one; an empty prefix is also how `is_loose` below decides.
pub fn art_destination(roms_root: &Path, system_id: &str, rom_relative_path: &str) -> (PathBuf, String) {
    let rom_abs = roms_root.join(rom_relative_path);
    let rom_dir = rom_abs.parent().map(Path::to_path_buf).unwrap_or_else(|| roms_root.to_path_buf());
    let system_dir = roms_root.join(system_id);

    if rom_dir == system_dir {
        let stem = Path::new(rom_relative_path).file_stem().and_then(|s| s.to_str()).unwrap_or("game");
        (rom_dir, format!("{stem}."))
    } else {
        (rom_dir, String::new())
    }
}

/// True if a ROM's file sits directly in its system's folder rather than in a folder of its own
/// -- surfaced by `relay scan` so a user knows which games they'd need to move into their own
/// folder if they want to add a manual/extra art alongside one.
pub fn is_loose(roms_root: &Path, system_id: &str, rom_relative_path: &str) -> bool {
    !art_destination(roms_root, system_id, rom_relative_path).1.is_empty()
}

pub fn to_forward_slash(path: &Path) -> String {
    path.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

/// Creates the full `~/Relay` directory tree up front, including one `roms/<system_id>`
/// subfolder per system in the fixed catalog, so a user knows exactly where to drop files for a
/// given system without having to first read docs or guess the id the scanner expects.
/// `create_dir_all` no-ops on subdirectories that already exist, so this is safe to call on every
/// startup, not just the first.
pub async fn ensure_library_dirs(root: &Path) -> std::io::Result<()> {
    for subdir in [
        "roms",
        "bios",
        "media",
        "saves",
        "savestates",
        "screenshots",
    ] {
        tokio::fs::create_dir_all(root.join(subdir)).await?;
    }

    let roms_dir = root.join("roms");
    for system in crate::systems::ALL {
        tokio::fs::create_dir_all(roms_dir.join(system.id)).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn art_destination_for_a_loose_rom_is_a_sibling_prefixed_with_its_stem() {
        let roms_root = PathBuf::from("/roms");
        let (dir, prefix) = art_destination(&roms_root, "snes", "snes/Chrono Trigger.sfc");

        assert_eq!(dir, roms_root.join("snes"));
        assert_eq!(prefix, "Chrono Trigger.");
        assert!(is_loose(&roms_root, "snes", "snes/Chrono Trigger.sfc"));
    }

    #[test]
    fn art_destination_for_an_already_foldered_rom_has_no_prefix() {
        let roms_root = PathBuf::from("/roms");
        let (dir, prefix) = art_destination(&roms_root, "psx", "psx/Chrono Cross/Chrono Cross.cue");

        assert_eq!(dir, roms_root.join("psx").join("Chrono Cross"));
        assert_eq!(prefix, "");
        assert!(!is_loose(&roms_root, "psx", "psx/Chrono Cross/Chrono Cross.cue"));
    }

    #[tokio::test]
    async fn ensure_library_dirs_creates_the_full_tree_including_the_root_itself() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Relay");

        ensure_library_dirs(&root).await.unwrap();

        for subdir in ["roms", "bios", "media", "saves", "savestates", "screenshots"] {
            assert!(root.join(subdir).is_dir(), "expected {subdir} to exist");
        }
    }

    #[tokio::test]
    async fn ensure_library_dirs_creates_a_roms_subfolder_per_supported_system() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Relay");

        ensure_library_dirs(&root).await.unwrap();

        for system in crate::systems::ALL {
            assert!(root.join("roms").join(system.id).is_dir(), "expected roms/{} to exist", system.id);
        }
    }

    #[tokio::test]
    async fn ensure_library_dirs_is_idempotent_on_a_tree_that_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Relay");
        tokio::fs::create_dir_all(root.join("roms")).await.unwrap();
        tokio::fs::write(root.join("roms").join("game.sfc"), vec![0u8; 5]).await.unwrap();

        ensure_library_dirs(&root).await.unwrap();

        assert_eq!(tokio::fs::read(root.join("roms").join("game.sfc")).await.unwrap(), vec![0u8; 5]);
    }
}

//! Scan stage of the ingestion pipeline (walk -> probe -> identify -> enrich, see
//! `ingestion::pipeline`) -- ported from relay-ui's `ingestion::scan`, superseding an earlier,
//! simpler flat-WalkDir scanner hand-written before this migration.

use std::path::{Path, PathBuf};

use crate::title::title_from_filename;

/// A file (or multi-disc unit) found by [`walk_system_folder`], not yet hashed or identified --
/// that happens in later ingestion stages (probe, identify).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanTarget {
    Single {
        file_path: PathBuf,
        title: String,
    },
    MultiDisc {
        m3u_path: PathBuf,
        disc_paths: Vec<PathBuf>,
        title: String,
    },
}

// Filters out dotfiles -- most importantly macOS's AppleDouble sidecar files (`._Some Game.gb`),
// which get silently created alongside every real file when copying a ROM library onto a
// non-HFS+ filesystem (SMB, exFAT, a plain USB stick) from a Mac, and otherwise match the real
// file's own extension exactly, indexing as a second bogus, near-duplicate game per real one.
// `.DS_Store` and friends are caught the same way, on general "hidden file" principle.
fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

/// Walks one system's rom folder (one level deep) and returns candidate scan targets.
///
/// A system folder contains loose rom files matching `extensions` (compared case-insensitively,
/// without a leading dot -- e.g. `"nes"` not `".nes"`), and/or subfolders -- one per game, which
/// is how most of a real library is laid out (a game's disc image plus companion files like a
/// scraped `.txt` or a stray `.DS_Store`, or an actual multi-disc set). A subfolder is resolved,
/// in order: an `.m3u` playlist inside it wins (a multi-disc set); otherwise, if exactly one file
/// in it matches `extensions`, that's the game (covers both a lone single-file rom in its own
/// folder, and a `.cue`+`.bin` set where only `.cue` is in the system's own extension list, so the
/// `.bin` tracks never even enter the count); anything else -- zero matches, or more than one with
/// no playlist to disambiguate -- is skipped entirely, since there's no reliable way to guess which
/// loose files belong together. A missing or unreadable system folder yields an empty list rather
/// than an error -- ported from the Electron MVP's `scanner/walk.ts`.
pub async fn walk_system_folder(system_folder: &Path, extensions: &[&str]) -> Vec<ScanTarget> {
    let Ok(mut entries) = tokio::fs::read_dir(system_folder).await else {
        return Vec::new();
    };

    let mut targets = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        let Ok(file_type) = entry.file_type().await else {
            continue;
        };
        let entry_path = entry.path();

        if is_hidden(&entry.file_name().to_string_lossy()) {
            continue;
        }

        if file_type.is_file() {
            let matches_extension = entry_path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| extensions.iter().any(|allowed| allowed.eq_ignore_ascii_case(ext)));

            if matches_extension {
                let stem = entry_path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
                targets.push(ScanTarget::Single {
                    file_path: entry_path.clone(),
                    title: title_from_filename(stem),
                });
            }
            continue;
        }

        if file_type.is_dir() {
            if let Some(target) = read_subfolder(&entry_path, extensions).await {
                targets.push(target);
            }
        }
    }

    targets
}

async fn read_subfolder(folder_path: &Path, extensions: &[&str]) -> Option<ScanTarget> {
    let mut inner = tokio::fs::read_dir(folder_path).await.ok()?;

    let mut m3u_path = None;
    let mut matching_files = Vec::new();
    while let Ok(Some(entry)) = inner.next_entry().await {
        let is_file = matches!(entry.file_type().await, Ok(ft) if ft.is_file());
        if !is_file {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_hidden(&name) {
            continue;
        }
        if name.to_lowercase().ends_with(".m3u") {
            m3u_path = Some(entry.path());
            break;
        }

        let entry_path = entry.path();
        let matches_extension = entry_path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| extensions.iter().any(|allowed| allowed.eq_ignore_ascii_case(ext)));
        if matches_extension {
            matching_files.push(entry_path);
        }
    }

    if let Some(m3u_path) = m3u_path {
        let playlist = tokio::fs::read_to_string(&m3u_path).await.ok()?;
        let disc_paths: Vec<PathBuf> = playlist
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| folder_path.join(line))
            .collect();

        let folder_name = folder_path.file_name()?.to_string_lossy().into_owned();
        return Some(ScanTarget::MultiDisc {
            m3u_path,
            disc_paths,
            title: title_from_filename(&folder_name),
        });
    }

    // No playlist -- a single unambiguous rom file in its own folder (the common one-game-per-
    // folder layout, and a `.cue`+`.bin` set whose `.bin` tracks aren't in the system's own
    // extension list) is still a valid target. Anything else is ambiguous and skipped.
    let mut matching_files = matching_files.into_iter();
    let file_path = matching_files.next()?;
    if matching_files.next().is_some() {
        return None;
    }

    let folder_name = folder_path.file_name()?.to_string_lossy().into_owned();
    Some(ScanTarget::Single { file_path, title: title_from_filename(&folder_name) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn missing_folder_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");

        let targets = walk_system_folder(&missing, &["nes"]).await;
        assert!(targets.is_empty());
    }

    #[tokio::test]
    async fn skips_macos_appledouble_sidecar_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Pokemon - Red Version.gb"), "real-rom-bytes").unwrap();
        // The exact AppleDouble naming macOS produces copying onto a non-HFS+ filesystem --
        // same extension as the real file, so only the leading dot distinguishes it.
        fs::write(dir.path().join("._Pokemon - Red Version.gb"), "resource-fork-junk").unwrap();

        let targets = walk_system_folder(dir.path(), &["gb"]).await;

        assert_eq!(targets.len(), 1);
        match &targets[0] {
            ScanTarget::Single { file_path, .. } => {
                assert_eq!(file_path.file_name().unwrap(), "Pokemon - Red Version.gb");
            }
            other => panic!("expected Single, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn skips_an_appledouble_sidecar_for_a_multi_disc_m3u() {
        let dir = tempfile::tempdir().unwrap();
        let game_dir = dir.path().join("Chrono Cross (USA)");
        fs::create_dir(&game_dir).unwrap();
        fs::write(game_dir.join("Chrono Cross.m3u"), "Chrono Cross (Disc 1).cue\n").unwrap();
        // A hidden sidecar for the playlist itself -- must not be mistaken for the real m3u.
        fs::write(game_dir.join("._Chrono Cross.m3u"), "junk").unwrap();

        let targets = walk_system_folder(dir.path(), &["cue"]).await;

        assert_eq!(targets.len(), 1);
        match &targets[0] {
            ScanTarget::MultiDisc { m3u_path, .. } => {
                assert_eq!(m3u_path.file_name().unwrap(), "Chrono Cross.m3u");
            }
            other => panic!("expected MultiDisc, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn matches_files_by_extension_case_insensitively() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Super Mario Bros. (USA).NES"), "").unwrap();
        fs::write(dir.path().join("readme.txt"), "").unwrap();

        let targets = walk_system_folder(dir.path(), &["nes"]).await;

        assert_eq!(targets.len(), 1);
        match &targets[0] {
            ScanTarget::Single { file_path, title } => {
                assert_eq!(file_path.file_name().unwrap(), "Super Mario Bros. (USA).NES");
                assert_eq!(title, "Super Mario Bros.");
            }
            other => panic!("expected Single, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn folder_with_no_m3u_and_multiple_matching_files_is_ambiguous_and_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let game_dir = dir.path().join("Some Game");
        fs::create_dir(&game_dir).unwrap();
        fs::write(game_dir.join("Some Game (Disc 1).cue"), "").unwrap();
        fs::write(game_dir.join("Some Game (Disc 2).cue"), "").unwrap();

        let targets = walk_system_folder(dir.path(), &["cue"]).await;
        assert!(targets.is_empty());
    }

    #[tokio::test]
    async fn single_file_in_its_own_folder_is_found() {
        let dir = tempfile::tempdir().unwrap();
        let game_dir = dir.path().join("Mario & Luigi - Superstar Saga (USA)");
        fs::create_dir(&game_dir).unwrap();
        fs::write(game_dir.join("Mario & Luigi - Superstar Saga (USA).gba"), "").unwrap();
        // A companion file scrapers/download sites drop alongside the rom -- must not make the
        // folder look ambiguous.
        fs::write(game_dir.join("Vimm's Lair.txt"), "").unwrap();

        let targets = walk_system_folder(dir.path(), &["gba"]).await;

        assert_eq!(targets.len(), 1);
        match &targets[0] {
            ScanTarget::Single { file_path, title } => {
                assert_eq!(file_path.file_name().unwrap(), "Mario & Luigi - Superstar Saga (USA).gba");
                assert_eq!(title, "Mario & Luigi - Superstar Saga");
            }
            other => panic!("expected Single, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn cue_bin_set_without_m3u_picks_the_cue_and_ignores_the_bins() {
        let dir = tempfile::tempdir().unwrap();
        let game_dir = dir.path().join("Sheep (Europe)");
        fs::create_dir(&game_dir).unwrap();
        fs::write(game_dir.join("Sheep (Europe).cue"), "").unwrap();
        fs::write(game_dir.join("Sheep (Europe) (Track 01).bin"), "").unwrap();
        fs::write(game_dir.join("Sheep (Europe) (Track 02).bin"), "").unwrap();

        // Mirrors a real system definition where `.bin` isn't in the extension allowlist (only
        // used by disc-less systems like Genesis) -- so the `.cue` is the only match.
        let targets = walk_system_folder(dir.path(), &["cue", "chd", "pbp"]).await;

        assert_eq!(targets.len(), 1);
        match &targets[0] {
            ScanTarget::Single { file_path, .. } => {
                assert_eq!(file_path.file_name().unwrap(), "Sheep (Europe).cue");
            }
            other => panic!("expected Single, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn multi_disc_folder_with_m3u_parses_disc_list() {
        let dir = tempfile::tempdir().unwrap();
        let game_dir = dir.path().join("Chrono Cross (USA)");
        fs::create_dir(&game_dir).unwrap();
        fs::write(
            game_dir.join("Chrono Cross.m3u"),
            "#EXTM3U\nChrono Cross (Disc 1).cue\n\nChrono Cross (Disc 2).cue\n",
        )
        .unwrap();

        let targets = walk_system_folder(dir.path(), &["cue"]).await;

        assert_eq!(targets.len(), 1);
        match &targets[0] {
            ScanTarget::MultiDisc { m3u_path, disc_paths, title } => {
                assert_eq!(m3u_path.file_name().unwrap(), "Chrono Cross.m3u");
                assert_eq!(title, "Chrono Cross");
                assert_eq!(
                    disc_paths,
                    &[
                        game_dir.join("Chrono Cross (Disc 1).cue"),
                        game_dir.join("Chrono Cross (Disc 2).cue"),
                    ]
                );
            }
            other => panic!("expected MultiDisc, got {other:?}"),
        }
    }
}

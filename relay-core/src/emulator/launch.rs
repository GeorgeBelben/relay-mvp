//! The full "launch a game" operation, redesigned out of relay-ui's Tauri `commands::emulator`
//! module (`launch_game` command + its `build_retroarch_options` helper). The original tangled
//! real policy (resolving settings, computing per-game save/state/screenshot dirs) with a Tauri
//! `AppHandle`/`Emitter`/`State` -- this version takes plain parameters and callbacks instead, so
//! both a Tauri command and a CLI invocation can share one implementation: a Tauri command wraps
//! the callbacks in `app.emit`/Tauri state, a CLI can just print them.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32};

use sqlx::SqlitePool;
use thiserror::Error;

use crate::db::{games, roms, settings};
use crate::emulator::command::{build_launch_command, BuildLaunchCommandError, RetroarchOptions, SystemLaunchConfig};
use crate::emulator::process::{self, LaunchError, LauncherStatus, LogLine};
use crate::emulator::retroarch_config::{self, GameLaunchDirs, RetroarchPreferences};
use crate::systems;

#[derive(Debug, Error)]
pub enum LaunchGameError {
    #[error("game not found: {0}")]
    GameNotFound(String),
    #[error("rom not found: {0}")]
    RomNotFound(String),
    #[error("system not found: {0}")]
    SystemNotFound(String),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("failed to write launch config: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    BuildCommand(#[from] BuildLaunchCommandError),
    #[error(transparent)]
    Launch(#[from] LaunchError),
}

/// `cores_path` comes from Settings, falling back to Relay's own default (a stock Ubuntu apt
/// install's libretro cores directory) rather than erroring when unset. `append_config_path`
/// overlays this game's save/state/screenshot directories on top of the user's own retroarch.cfg
/// -- overwritten fresh on every launch, so no per-launch cleanup is needed. `library_root` and
/// `config_dir` are taken as parameters rather than resolved internally (e.g. via a Tauri
/// AppHandle) so tests, the CLI, and a Tauri command can each point them wherever makes sense.
pub async fn build_retroarch_options(
    pool: &SqlitePool,
    system_id: &str,
    game_id: &str,
    library_root: &Path,
    config_dir: &Path,
) -> Result<RetroarchOptions, LaunchGameError> {
    let general_settings = settings::get_general_settings(pool).await?;

    tokio::fs::create_dir_all(config_dir).await?;
    let append_config_path = config_dir.join(format!("{game_id}.cfg"));

    let saves_dir = library_root.join("saves").join(system_id).join(game_id);
    let save_states_dir = library_root.join("savestates").join(system_id).join(game_id);
    let screenshots_dir = library_root.join("screenshots").join(system_id).join(game_id);
    let dirs = GameLaunchDirs { saves_dir: &saves_dir, save_states_dir: &save_states_dir, screenshots_dir: &screenshots_dir };
    let preferences = RetroarchPreferences {
        video_smooth: general_settings.video_smooth,
        video_scale_integer: general_settings.video_scale_integer,
        run_ahead_enabled: general_settings.run_ahead_enabled,
    };
    retroarch_config::write_launch_config(&append_config_path, &dirs, &preferences).await?;

    Ok(RetroarchOptions { cores_path: PathBuf::from(general_settings.retroarch_cores_path), append_config_path })
}

/// Resolves game/rom/system from the DB, builds the right launch command (RetroArch core vs.
/// standalone binary), and runs it to completion. `running`/`active_pid` are taken as parameters
/// rather than owned here -- a Tauri app holds them in long-lived state shared across commands
/// (kill/pause act on the same atomics mid-launch); a one-shot CLI invocation can just construct
/// fresh ones since there's nothing else to coordinate with within a single process run.
#[allow(clippy::too_many_arguments)]
pub async fn launch_game(
    pool: &SqlitePool,
    library_root: &Path,
    config_dir: &Path,
    game_id: &str,
    running: &AtomicBool,
    active_pid: &AtomicU32,
    on_status: impl FnMut(LauncherStatus),
    on_log: impl Fn(LogLine),
) -> Result<(), LaunchGameError> {
    let game = games::get(pool, game_id).await?.ok_or_else(|| LaunchGameError::GameNotFound(game_id.to_string()))?;
    let rom = roms::get(pool, &game.rom_id).await?.ok_or_else(|| LaunchGameError::RomNotFound(game.rom_id.clone()))?;
    let system = systems::get(&rom.system_id).ok_or_else(|| LaunchGameError::SystemNotFound(rom.system_id.clone()))?;

    let rom_path = library_root.join("roms").join(&rom.path).to_string_lossy().into_owned();
    let system_config = SystemLaunchConfig { retroarch_core: system.retroarch_core, standalone_binary: system.standalone_binary };

    let retroarch_options = match &system.retroarch_core {
        Some(_) => Some(build_retroarch_options(pool, &rom.system_id, game_id, library_root, config_dir).await?),
        None => None,
    };

    let launch_command = build_launch_command(&system_config, &rom_path, retroarch_options.as_ref())?;

    process::launch(&launch_command.command, &launch_command.args, running, active_pid, on_status, on_log).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn throwaway_pool() -> (SqlitePool, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let options = sqlx::sqlite::SqliteConnectOptions::new().filename(&db_path).create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new().connect_with(options).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        (pool, dir)
    }

    #[tokio::test]
    async fn build_retroarch_options_creates_per_game_save_dirs_and_a_populated_appendconfig() {
        let (pool, _db_dir) = throwaway_pool().await;
        let library_root = tempfile::tempdir().unwrap();
        let config_dir = tempfile::tempdir().unwrap();

        let options = build_retroarch_options(&pool, "nes", "game-1", library_root.path(), config_dir.path()).await.unwrap();

        assert!(library_root.path().join("saves/nes/game-1").is_dir());
        assert!(library_root.path().join("savestates/nes/game-1").is_dir());
        assert!(library_root.path().join("screenshots/nes/game-1").is_dir());

        let contents = tokio::fs::read_to_string(&options.append_config_path).await.unwrap();
        assert!(contents.contains(&format!(
            "savefile_directory = \"{}\"",
            library_root.path().join("saves/nes/game-1").to_string_lossy()
        )));
        // No retroarchCoresPath setting configured -- falls back to the default.
        assert_eq!(options.cores_path, PathBuf::from("/usr/lib/x86_64-linux-gnu/libretro"));
    }

    #[tokio::test]
    async fn launch_game_errors_clearly_when_the_game_id_is_unknown() {
        let (pool, _db_dir) = throwaway_pool().await;
        let library_root = tempfile::tempdir().unwrap();
        let config_dir = tempfile::tempdir().unwrap();
        let running = AtomicBool::new(false);
        let active_pid = AtomicU32::new(0);

        let err = launch_game(&pool, library_root.path(), config_dir.path(), "does-not-exist", &running, &active_pid, |_| {}, |_| {})
            .await
            .unwrap_err();

        assert!(matches!(err, LaunchGameError::GameNotFound(id) if id == "does-not-exist"));
    }
}

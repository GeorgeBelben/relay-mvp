use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex};

use relay_core::emulator::launch::launch_game as core_launch_game;
use relay_core::emulator::process::{self, LauncherStatus};
use relay_core::emulator::retroarch_command;
use relay_core::library;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Manager, State};

/// Shared launch state, managed as Tauri state so it lives for the app's lifetime -- one game (so
/// one child process) can run at a time, matching the MVP's single-window/single-device model.
pub struct LauncherState {
    pub running: Arc<AtomicBool>,
    pub active_pid: Arc<AtomicU32>,
    pub status: Arc<Mutex<LauncherStatus>>,
}

impl Default for LauncherState {
    fn default() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            active_pid: Arc::new(AtomicU32::new(0)),
            status: Arc::new(Mutex::new(LauncherStatus::Idle)),
        }
    }
}

#[tauri::command]
pub fn get_launcher_status(state: State<'_, LauncherState>) -> LauncherStatus {
    state.status.lock().unwrap().clone()
}

#[tauri::command]
pub fn kill_game(state: State<'_, LauncherState>) -> Result<(), String> {
    process::kill(&state.active_pid).map_err(crate::logging::err_to_string)
}

/// Fire-and-forget over RetroArch's UDP command port (REL-23) -- a silent no-op for a
/// standalone-emulator game (Dolphin/PCSX2/yabause-qt have no such interface) or when nothing's
/// running, rather than an error, since the frontend can't always know which case applies before
/// the player presses the quick-menu's Pause/Save action.
#[tauri::command]
pub async fn pause_toggle_game() -> Result<(), String> {
    retroarch_command::send_command("PAUSE_TOGGLE").await.map_err(crate::logging::err_to_string)
}

#[tauri::command]
pub async fn save_state_game() -> Result<(), String> {
    retroarch_command::send_command("SAVE_STATE").await.map_err(crate::logging::err_to_string)
}

#[tauri::command]
pub async fn load_state_game() -> Result<(), String> {
    retroarch_command::send_command("LOAD_STATE").await.map_err(crate::logging::err_to_string)
}

#[tauri::command]
pub async fn reset_game() -> Result<(), String> {
    retroarch_command::send_command("RESET").await.map_err(crate::logging::err_to_string)
}

/// Freezes a standalone-emulator process (PCSX2/Dolphin/yabause-qt) at the OS level -- REL-147.
/// These have no remote command interface the way a RetroArch core does, so `pause_toggle_game`
/// above is a silent no-op for them; the quick menu falls back to this instead so the game
/// actually stops running behind the menu rather than continuing unpaused.
#[tauri::command]
pub fn pause_standalone_game(state: State<'_, LauncherState>) -> Result<(), String> {
    process::pause_process(&state.active_pid).map_err(crate::logging::err_to_string)
}

#[tauri::command]
pub fn resume_standalone_game(state: State<'_, LauncherState>) -> Result<(), String> {
    process::resume_process(&state.active_pid).map_err(crate::logging::err_to_string)
}

/// Thin Tauri wrapper: resolves the two Tauri-specific paths (app data dir for the launch-config
/// overlay, `~/Relay` as the library root) and forwards status/log events onto the frontend, then
/// delegates the actual game/rom/system lookup + launch-command building + process lifecycle to
/// `relay_core::emulator::launch::launch_game`.
#[tauri::command]
pub async fn launch_game<R: tauri::Runtime>(
    app: AppHandle<R>,
    pool: State<'_, SqlitePool>,
    state: State<'_, LauncherState>,
    game_id: String,
) -> Result<(), String> {
    let library_root = library::library_root();
    let config_dir = app.path().app_data_dir().map_err(crate::logging::err_to_string)?.join("launch-configs");

    core_launch_game(
        pool.inner(),
        &library_root,
        &config_dir,
        &game_id,
        &state.running,
        &state.active_pid,
        |next| {
            if let Ok(mut guard) = state.status.lock() {
                *guard = next.clone();
            }
            let _ = app.emit("launcher:status", &next);
        },
        |log_line| {
            let _ = app.emit("launcher:log", &log_line);
        },
    )
    .await
    .map_err(crate::logging::err_to_string)
}

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use tauri::State;

/// Tracks which gamepad ids are currently connected, kept in sync (see lib.rs's setup) with the
/// same events pushed to the frontend as "gamepad:event" -- lets `list_connected_gamepads` answer
/// correctly for a listener that attaches after some pads already connected, since the watcher
/// itself starts at app setup, before the frontend has necessarily mounted. Same "pull the current
/// state on mount, then stay live via the push event" shape as ScanStatusState/get_scan_status.
pub type ConnectedGamepads = Arc<Mutex<BTreeSet<usize>>>;

#[tauri::command]
pub fn list_connected_gamepads(state: State<'_, ConnectedGamepads>) -> Vec<usize> {
    state.lock().unwrap().iter().copied().collect()
}

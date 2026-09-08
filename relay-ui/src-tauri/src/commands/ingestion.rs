use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use relay_core::ingestion::pipeline::{self, ScanStatus};
use relay_core::library;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, Manager, State};

/// Guards against overlapping "Rescan Library" runs -- managed as Tauri state so it lives for
/// the app's lifetime, shared across every `rescan_library` invocation.
pub struct RescanGuard(pub Arc<AtomicBool>);

impl Default for RescanGuard {
    fn default() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
}

/// The most recently observed [`ScanStatus`], for `get_scan_status`'s pull-on-mount read -- a
/// listener that attaches after a fast scan already finished would otherwise never see anything.
pub struct ScanStatusState(pub Mutex<ScanStatus>);

impl Default for ScanStatusState {
    fn default() -> Self {
        Self(Mutex::new(ScanStatus::Idle))
    }
}

#[tauri::command]
pub fn get_scan_status(status: State<'_, ScanStatusState>) -> ScanStatus {
    status.0.lock().unwrap().clone()
}

/// Thin Tauri wrapper: resolves the Tauri-specific DATs cache location and forwards status events
/// onto the frontend (and into `ScanStatusState`, for a listener that attaches after a fast scan
/// already finished), delegating the actual settings-resolution + scan/probe/identify/enrich work
/// to `relay_core::ingestion::pipeline::rescan_from_settings`.
#[tauri::command]
pub async fn rescan_library(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    guard: State<'_, RescanGuard>,
    status: State<'_, ScanStatusState>,
) -> Result<(), String> {
    // Mirrors the Electron MVP's <userData>/dats/ cache location.
    let app_data_dir = app.path().app_data_dir().map_err(crate::logging::err_to_string)?;
    let dats_cache_dir = app_data_dir.join("dats");
    let ra_cache_dir = app_data_dir.join("ra-hashes");

    pipeline::rescan_from_settings(pool.inner(), &library::roms_path(), &dats_cache_dir, &ra_cache_dir, &guard.0, |next| {
        if let Ok(mut guard) = status.0.lock() {
            *guard = next.clone();
        }
        let _ = app.emit("scanner:status", &next);
    })
    .await
    .map(|_summary| ())
    .map_err(crate::logging::err_to_string)
}

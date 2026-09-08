pub mod commands;
pub mod logging;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init();
    logging::install_panic_hook();
    tracing::info!("Relay starting up");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                // relay_core::init::ensure_environment resolves/creates the app data dir, opens
                // the DB, runs migrations, and ensures the full ~/Relay tree -- everything the
                // Tauri setup used to do inline here by hand. It doesn't hand back a pool (it
                // opens and drops its own internally), so we connect a second one against the
                // same file to hold in Tauri state -- SQLite is fine with multiple connections
                // to one file, and migrations are already applied by the time this runs.
                let report = relay_core::init::ensure_environment()
                    .await
                    .expect("failed to set up relay-core environment");

                let db_path = report.data_dir.join("relay.db");
                let options = SqliteConnectOptions::new().filename(&db_path).create_if_missing(true);
                let pool = SqlitePoolOptions::new().connect_with(options).await.expect("failed to connect to database");

                app_handle.manage(pool);
            });
            app.manage(commands::ingestion::RescanGuard::default());
            app.manage(commands::ingestion::ScanStatusState::default());
            app.manage(commands::emulator::LauncherState::default());

            // Kiosk has no mouse input; hide the cursor and only reveal the window once it's
            // hidden, so there's never a frame with GTK's default arrow visible (see main.css's
            // `cursor: none`, which can't take effect until the page's own CSS has loaded).
            let window = app.get_webview_window("main").expect("main window must exist");
            window.set_cursor_visible(false)?;
            window.show()?;

            // Dev runs in a normal windowed desktop, not the kiosk's cage compositor -- fullscreen
            // there is just a convenience. Never applied in a release build: requesting native
            // fullscreen at window *creation* (tauri.conf.json's `fullscreen`) is what crashes
            // cage/wlroots on the real kiosk device (see CLAUDE.md's Key Decisions table); toggling
            // it post-creation, gated to debug builds only, never touches that path.
            #[cfg(debug_assertions)]
            window.set_fullscreen(true)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::ingestion::rescan_library,
            commands::ingestion::get_scan_status,
            commands::emulator::launch_game,
            commands::emulator::kill_game,
            commands::emulator::get_launcher_status,
            commands::emulator::pause_toggle_game,
            commands::emulator::pause_standalone_game,
            commands::emulator::resume_standalone_game,
            commands::emulator::save_state_game,
            commands::emulator::load_state_game,
            commands::emulator::reset_game,
            commands::games::list_games,
            commands::games::get_game,
            commands::games::create_game,
            commands::games::update_game,
            commands::games::delete_game,
            commands::roms::list_roms,
            commands::roms::get_rom,
            commands::roms::create_rom,
            commands::roms::update_rom,
            commands::roms::delete_rom,
            commands::systems::list_systems,
            commands::systems::get_system,
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::get_general_settings,
            commands::settings::set_onboarding_completed,
            commands::settings::set_controller_type,
            commands::settings::set_active_profile_id,
            commands::settings::set_retroarch_cores_path,
            commands::settings::set_sound_volume,
            commands::settings::set_rumble_enabled,
            commands::settings::set_video_smooth,
            commands::settings::set_video_scale_integer,
            commands::settings::set_run_ahead_enabled,
            commands::storage::get_storage_usage,
            commands::network::list_wifi_networks,
            commands::network::connect_to_wifi_network,
            commands::bluetooth::scan_for_bluetooth_devices,
            commands::bluetooth::list_paired_bluetooth_devices,
            commands::bluetooth::pair_bluetooth_device,
            commands::bluetooth::remove_bluetooth_device,
            commands::profiles::list_profiles,
            commands::profiles::get_profile,
            commands::profiles::create_profile,
            commands::profiles::rename_profile,
            commands::profiles::delete_profile,
            commands::profiles::link_ra_web_api,
            commands::profiles::link_ra_connect_account,
            commands::profiles::unlink_ra,
            commands::profiles::get_ra_stats,
            commands::profiles::refresh_ra_stats,
            commands::game_media::list_game_media,
            commands::game_media::get_media_root_path,
            commands::game_media::list_game_media_files,
            commands::game_media::select_boxart_file,
            commands::library::list_library_shelves,
            commands::library::list_all_games_in_library,
            commands::library::list_recently_added_games,
            commands::game_actions::search_for_reidentify,
            commands::game_actions::apply_reidentify,
            commands::game_actions::get_achievements,
            commands::system::get_username,
            commands::system::quit,
            commands::system::hide_cursor,
            commands::system::shutdown_device,
            commands::system::reboot_device,
            commands::datetime::get_datetime_status,
            commands::datetime::list_timezones,
            commands::datetime::set_timezone,
            commands::datetime::set_ntp_enabled,
            commands::datetime::set_time,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

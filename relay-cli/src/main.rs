use std::sync::atomic::{AtomicBool, AtomicU32};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "relay")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan the ROM library, identify games, and download artwork
    Scan,
    /// List every game currently known to the database
    Games,
    /// List every system relay supports
    Systems,
    /// Show a disk usage breakdown of the ~/Relay library
    Storage,
    /// Check the environment (data dir, database, migrations) and report its state
    Doctor,
    /// Manage local user profiles
    Profiles {
        #[command(subcommand)]
        action: ProfilesAction,
    },
    /// Read or write app settings
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Launch a game by id (see `relay games`)
    Play { game_id: String },
    /// Manage Wi-Fi
    Wifi {
        #[command(subcommand)]
        action: WifiAction,
    },
    /// Manage Bluetooth controllers
    Bluetooth {
        #[command(subcommand)]
        action: BluetoothAction,
    },
}

#[derive(Subcommand)]
enum WifiAction {
    /// List nearby Wi-Fi networks
    List,
    /// Connect to a Wi-Fi network
    Connect {
        ssid: String,
        #[arg(long)]
        password: Option<String>,
    },
}

#[derive(Subcommand)]
enum BluetoothAction {
    /// Scan for nearby devices
    Scan,
    /// List already-paired devices
    List,
    /// Pair with a device by address
    Pair { address: String },
}

#[derive(Subcommand)]
enum ProfilesAction {
    /// List all profiles
    List,
    /// Create a new profile
    Create { name: String },
    /// Delete a profile by id (see `relay profiles list`)
    Delete { id: String },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Read a setting's raw value
    Get { key: String },
    /// Write a setting's raw value
    Set { key: String, value: String },
}

#[tokio::main]
async fn main() {
    let report = match relay_core::init::ensure_environment().await {
        Ok(report) => report,
        Err(err) => {
            eprintln!("startup failed: {err}");
            std::process::exit(1);
        }
    };

    let result = match Cli::parse().command {
        Commands::Scan => scan(&report).await,
        Commands::Games => games(&report).await,
        Commands::Systems => systems(),
        Commands::Storage => storage().await,
        Commands::Doctor => doctor(&report),
        Commands::Profiles { action } => profiles(&report, action).await,
        Commands::Config { action } => config(&report, action).await,
        Commands::Play { game_id } => play(&report, &game_id).await,
        Commands::Wifi { action } => wifi(action).await,
        Commands::Bluetooth { action } => bluetooth(action).await,
    };

    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn scan(report: &relay_core::init::Report) -> Result<(), String> {
    use relay_core::ingestion::pipeline::ScanStatus;

    let roms_root = relay_core::library::roms_path();
    let dats_cache_dir = report.data_dir.join("dats");
    let ra_cache_dir = report.data_dir.join("ra-hashes");
    let running = AtomicBool::new(false);

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));
    spinner.set_message("Scanning library...");

    let on_status = |status| match status {
        ScanStatus::ScanningFiles => spinner.set_message("Scanning library..."),
        ScanStatus::EnrichingArt { current, total } => spinner.set_message(format!("Downloading artwork... {current}/{total}")),
        ScanStatus::Idle | ScanStatus::Done | ScanStatus::Error { .. } => {}
    };
    let result = relay_core::ingestion::pipeline::rescan_from_settings(&report.pool, &roms_root, &dats_cache_dir, &ra_cache_dir, &running, on_status).await;

    if let Err(err) = result {
        spinner.finish_and_clear();
        return Err(format!("scan failed: {err}"));
    }

    spinner.finish_and_clear();

    let found = relay_core::db::games::list(&report.pool).await.map_err(|err| format!("scan finished but couldn't count games: {err}"))?.len();
    println!("Found {found} rom(s)");

    // Loose ROMs (sitting directly in roms/<system>/, not in a folder of their own) can't have a
    // manual/extra art colocated with them the way a foldered one can -- flagged so you know
    // which ones you'd need to move yourself if you want that.
    let roms = relay_core::db::roms::list(&report.pool).await.map_err(|err| format!("couldn't check for loose roms: {err}"))?;
    let loose: Vec<_> = roms.iter().filter(|rom| relay_core::library::is_loose(&roms_root, &rom.system_id, &rom.path)).collect();
    if !loose.is_empty() {
        println!("{} rom(s) not in their own folder:", loose.len());
        for rom in loose {
            println!("  {}", rom.path);
        }
    }

    Ok(())
}

async fn games(report: &relay_core::init::Report) -> Result<(), String> {
    let games = relay_core::db::games::list(&report.pool).await.map_err(|err| format!("couldn't list games: {err}"))?;
    let roms = relay_core::db::roms::list(&report.pool).await.map_err(|err| format!("couldn't list roms: {err}"))?;

    println!("{} game(s):", games.len());
    for game in &games {
        // RetroAchievements is what actually identifies a game (exact hash match); SteamGridDB
        // only ever fetches optional box art, independent of identification.
        let identified = if game.retroachievements_game_id.is_some() { "✅" } else { "❌" };
        let art = if game.steamgriddb_id.is_some() { " 🖼️" } else { "" };
        let missing = match roms.iter().find(|rom| rom.id == game.rom_id) {
            Some(rom) if rom.status == "missing" => " ⚠️",
            _ => "",
        };
        println!("  {identified}{art}{missing}  {} [{}]", game.title, game.id);
    }

    Ok(())
}

fn systems() -> Result<(), String> {
    println!("{} system(s):", relay_core::systems::ALL.len());
    for system in relay_core::systems::ALL {
        let emulator = system.retroarch_core.or(system.standalone_binary).unwrap_or("none configured");
        println!("  [{}] {} ({emulator})", system.id, system.name);
    }
    Ok(())
}

async fn storage() -> Result<(), String> {
    let library_root = relay_core::library::library_root();
    let usage = relay_core::storage::get_storage_usage(&library_root).await.map_err(|err| format!("couldn't read storage usage: {err}"))?;

    let gb = |bytes: u64| bytes as f64 / 1_073_741_824.0;
    println!("Disk: {:.1} GB free of {:.1} GB", gb(usage.free_bytes), gb(usage.total_bytes));
    println!("  roms:        {:.2} GB", gb(usage.games_bytes));
    println!("  bios:        {:.2} GB", gb(usage.bios_bytes));
    println!("  media:       {:.2} GB", gb(usage.media_bytes));
    println!("  saves/states: {:.2} GB", gb(usage.saves_bytes));
    println!("  everything else: {:.2} GB", gb(usage.system_bytes));

    Ok(())
}

fn doctor(report: &relay_core::init::Report) -> Result<(), String> {
    println!("data dir:       {}", report.data_dir.display());
    println!("library root:   {}", relay_core::library::library_root().display());
    println!("migrations run: {}", report.migrations_run);
    println!("environment looks healthy.");
    Ok(())
}

async fn profiles(report: &relay_core::init::Report, action: ProfilesAction) -> Result<(), String> {
    match action {
        ProfilesAction::List => {
            let profiles = relay_core::db::profiles::list(&report.pool).await.map_err(|err| format!("couldn't list profiles: {err}"))?;
            println!("{} profile(s):", profiles.len());
            for profile in profiles {
                let summary: relay_core::db::profiles::ProfileSummary = profile.into();
                println!("  {} ({})", summary.name, summary.id);
            }
        }
        ProfilesAction::Create { name } => {
            let profile = relay_core::db::profiles::create(&report.pool, &name).await.map_err(|err| format!("couldn't create profile: {err}"))?;
            println!("Created profile \"{}\" ({})", profile.name, profile.id);
        }
        ProfilesAction::Delete { id } => {
            relay_core::db::profiles::delete(&report.pool, &id).await.map_err(|err| format!("couldn't delete profile: {err}"))?;
            println!("Deleted profile {id}");
        }
    }
    Ok(())
}

async fn config(report: &relay_core::init::Report, action: ConfigAction) -> Result<(), String> {
    match action {
        ConfigAction::Get { key } => match relay_core::db::settings::get(&report.pool, &key).await.map_err(|err| format!("couldn't read setting: {err}"))? {
            Some(value) => println!("{value}"),
            None => println!("(not set)"),
        },
        ConfigAction::Set { key, value } => {
            relay_core::db::settings::set(&report.pool, &key, &value).await.map_err(|err| format!("couldn't write setting: {err}"))?;
            println!("Set {key} = {value}");
        }
    }
    Ok(())
}

async fn play(report: &relay_core::init::Report, game_id: &str) -> Result<(), String> {
    let library_root = relay_core::library::library_root();
    let config_dir = report.data_dir.join("launch-configs");
    let running = AtomicBool::new(false);
    let active_pid = AtomicU32::new(0);

    relay_core::emulator::launch::launch_game(
        &report.pool,
        &library_root,
        &config_dir,
        game_id,
        &running,
        &active_pid,
        |status| println!("{status:?}"),
        |log| println!("[{:?}] {}", log.stream, log.line),
    )
    .await
    .map_err(|err| format!("couldn't launch game: {err}"))
}

async fn wifi(action: WifiAction) -> Result<(), String> {
    match action {
        WifiAction::List => {
            let networks = relay_core::system::network::list_wifi_networks("nmcli").await?;
            println!("{} network(s):", networks.len());
            for network in &networks {
                let lock = if network.secured { " (secured)" } else { "" };
                let marker = if network.in_use { "*" } else { " " };
                println!("{marker} {} (signal {}){lock}", network.ssid, network.signal);
            }
        }
        WifiAction::Connect { ssid, password } => {
            relay_core::system::network::connect_to_wifi_network("nmcli", &ssid, password.as_deref())
                .await
                .map_err(|err| err.to_string())?;
            println!("Connected to {ssid}");
        }
    }
    Ok(())
}

async fn bluetooth(action: BluetoothAction) -> Result<(), String> {
    let devices = match action {
        BluetoothAction::Scan => relay_core::system::bluetooth::scan_for_devices("bluetoothctl").await?,
        BluetoothAction::List => relay_core::system::bluetooth::list_paired_devices("bluetoothctl").await?,
        BluetoothAction::Pair { address } => {
            relay_core::system::bluetooth::pair_device("bluetoothctl", &address).await.map_err(|err| err.to_string())?;
            println!("Paired with {address}");
            return Ok(());
        }
    };

    println!("{} device(s):", devices.len());
    for device in &devices {
        let battery = device.battery_percent.map(|b| format!(" ({b}%)")).unwrap_or_default();
        println!("  {} [{}]{battery}", device.name, device.address);
    }
    Ok(())
}

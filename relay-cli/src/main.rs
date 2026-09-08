use std::sync::atomic::AtomicBool;

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
}

#[derive(Subcommand)]
enum ProfilesAction {
    /// List all profiles
    List,
    /// Create a new profile
    Create { name: String },
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
    };

    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn scan(report: &relay_core::init::Report) -> Result<(), String> {
    let roms_root = relay_core::library::roms_path();
    let media_root = relay_core::library::media_path();
    let dats_cache_dir = report.data_dir.join("dats");
    let running = AtomicBool::new(false);

    relay_core::ingestion::pipeline::rescan_from_settings(&report.pool, &roms_root, &media_root, &dats_cache_dir, &running, |status| {
        println!("{status:?}");
    })
    .await
    .map_err(|err| format!("scan failed: {err}"))
}

async fn games(report: &relay_core::init::Report) -> Result<(), String> {
    let games = relay_core::db::games::list(&report.pool).await.map_err(|err| format!("couldn't list games: {err}"))?;

    println!("{} game(s):", games.len());
    for game in &games {
        let enriched = if game.enriched_at.is_some() { "" } else { " (not yet identified)" };
        println!("  {}{}", game.title, enriched);
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

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "relay")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan the ROM library and report what was found
    Scan,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match relay_core::init::ensure_environment().await {
        Ok(_) => {}
        Err(err) => {
            eprintln!("startup failed: {err}");
            std::process::exit(1);
        }
    }

    match cli.command {
        Commands::Scan => {
            // NOTE: calls the raw scan stage directly (per-system folder walk only), not the full
            // DB-backed ingestion::pipeline::rescan (scan+probe+identify+enrich) -- wiring that up
            // (a DB pool, a NoIntroDatLookup, an optional SteamGridDB client from settings) is
            // real design work, tracked separately rather than done inline here.
            let roms_root = relay_core::library::library_root().join("roms");
            let mut total = 0;
            for system in relay_core::systems::ALL {
                let system_folder = roms_root.join(system.id);
                let targets = relay_core::scan::walk_system_folder(&system_folder, system.extensions).await;
                for target in &targets {
                    let title = match target {
                        relay_core::scan::ScanTarget::Single { title, .. } => title,
                        relay_core::scan::ScanTarget::MultiDisc { title, .. } => title,
                    };
                    println!("  [{}] {}", system.id, title);
                }
                total += targets.len();
            }
            println!("Found {total} rom(s)");
        }
    }
}

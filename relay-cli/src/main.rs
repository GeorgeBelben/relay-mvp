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
            match relay_core::scan::scan_library(&relay_core::library::library_root()) {
                Ok(report) => {
                    println!("Found {} rom(s):", report.roms.len());
                    for rom in &report.roms {
                        let name = rom.path.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
                        println!("  [{}] {}", rom.system_id, name);
                    }
                }
                Err(err) => {
                    eprintln!("scan failed: {err}");
                    std::process::exit(1);
                }
            }
        }
    }
}

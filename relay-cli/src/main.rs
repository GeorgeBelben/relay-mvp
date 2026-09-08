use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "relay")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check the data directory and database, creating/migrating as needed
    Init,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match relay_core::init::ensure_environment().await {
        Ok(report) => {
            println!("relay is ready: {:?}", report);
        }
        Err(err) => {
            eprintln!("startup failed: {err}");
            std::process::exit(1);
        }
    }

    match cli.command {
        Commands::Init => {
            // ensure_environment already ran above, unconditionally, for every
            // command -- `relay init` just makes that explicit for the user.
        }
    }
}

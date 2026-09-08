#[tokio::main]
async fn main() {
    match relay_core::init::ensure_environment().await {
        Ok(report) => {
            println!("relay is ready: {:?}", report);
        }
        Err(err) => {
            eprintln!("startup failed: {err}");
            std::process::exit(1);
        }
    }
}

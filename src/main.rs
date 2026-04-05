use tracing::error;

#[tokio::main]
async fn main() {
    if let Err(e) = jail_ai::run_cli().await {
        error!("Error: {}", e);
        std::process::exit(1);
    }
}

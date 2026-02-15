use log::error;

#[tokio::main]
async fn main() {
    if let Err(err) = artifacts::cli::run().await {
        error!("{:#}", err);
        std::process::exit(1);
    }
}

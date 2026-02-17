#[cfg(feature = "logging")]
use log::error;

#[tokio::main]
async fn main() {
    if let Err(err) = artifacts::cli::run().await {
        #[cfg(feature = "logging")]
        error!("{:#}", err);
        #[cfg(not(feature = "logging"))]
        eprintln!("{:#}", err);
        std::process::exit(1);
    }
}

use log::error;

fn main() {
    if let Err(err) = artifacts_cli::cli::run() {
        error!("{:#}", err);
        std::process::exit(1);
    }
}

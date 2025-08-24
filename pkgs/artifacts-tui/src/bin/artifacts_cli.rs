fn main() {
    if let Err(err) = artifacts_tui::cli::run() {
        eprintln!("error: {:#}", err);
        std::process::exit(1);
    }
}

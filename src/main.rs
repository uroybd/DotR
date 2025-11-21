use clap::Parser;
use dotr::cli::{Cli, run_cli};

fn main() {
    let args = Cli::parse();
    if let Err(e) = run_cli(args) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

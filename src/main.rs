use clap::Parser;
use dotr_dear::{
    cli::{Cli, run_cli},
    utils::{LogLevel, cprintln},
};

fn main() {
    let args = Cli::parse();
    if let Err(e) = run_cli(args) {
        cprintln(&e.to_string(), &LogLevel::Error);
        std::process::exit(1);
    }
}

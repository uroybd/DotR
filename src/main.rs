use clap::Parser;
use dotr::{
    cli::{Cli, run_cli},
    utils::{LogLevel, cprintln},
};

fn main() {
    let args = Cli::parse();
    if let Err(e) = run_cli(args) {
        let error = format!("{}", e);
        cprintln(&error, &LogLevel::ERROR);
        std::process::exit(1);
    }
}

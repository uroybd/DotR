use clap::Parser;
use dotr::cli::{Cli, run_cli};

fn main() {
    let args = Cli::parse();
    run_cli(args);
}

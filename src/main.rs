use clap::Parser;
use dotr::cli::{run_cli, Cli};

fn main() {
    let args = Cli::parse();
    run_cli(args);
}

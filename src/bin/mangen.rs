//! Generates man pages for `dotr` and every subcommand into `man/`.
//! Run with `cargo run --bin mangen --features mangen` (or `make man`).

use clap::CommandFactory;
use dotr_dear::cli::Cli;

fn main() -> std::io::Result<()> {
    let out_dir = std::path::Path::new("man");
    std::fs::create_dir_all(out_dir)?;
    clap_mangen::generate_to(Cli::command(), out_dir)?;
    println!("Generated man pages in {}", out_dir.display());
    Ok(())
}

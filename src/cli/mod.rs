use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::config;

pub mod copydots;
pub mod importdots;
pub mod initconfig;

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Command>,
    #[clap(short, long, global = true)]
    pub working_dir: String,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize the application
    Init {},
    Import {
        path: String,
    },
}

const BANNER: &str = r#"
██████╗  ██████╗ ████████╗██████╗ 
██╔══██╗██╔═══██╗╚══██╔══╝██╔══██╗
██║  ██║██║   ██║   ██║   ██████╔╝
██║  ██║██║   ██║   ██║   ██╔══██╗
██████╔╝╚██████╔╝   ██║   ██║  ██║
╚═════╝  ╚═════╝    ╚═╝   ╚═╝  ╚═╝
"#;

pub fn run_cli(args: Cli) {
    let mut working_dir: PathBuf = PathBuf::new(); // if working_dir is empty, set it to current directory
    if args.working_dir.is_empty() {
        working_dir = std::env::current_dir().expect("Failed to get current directory");
    } else {
        working_dir = PathBuf::from(args.working_dir);
        working_dir = working_dir.canonicalize().unwrap();
        if !working_dir.exists() {
            panic!("The specified working directory does not exist");
        }
    }
    // Print working directory
    // Print full working directory path
    match args.command {
        Some(Command::Init {}) => {
            println!("Initializing configuration...");
            initconfig::init_config(&working_dir);
        }
        None => {
            println!("No command provided. Use --help for more information.");
        }
        Some(_) => {
            let mut conf = config::load_config(&working_dir);
            if conf.banner {
                println!("{}", BANNER);
            }
            match args.command {
                Some(Command::Import { path }) => {
                    importdots::import_dots(&path, &mut conf, &working_dir);
                }
                _ => {
                    println!("Unknown command. Use --help for more information.");
                }
            }
        }
    }
}

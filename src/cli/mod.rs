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
    pub working_dir: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize the application
    Init {},
    Import {
        path: String,
    },
    Copy {},
    Update {},
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
    let mut working_dir = std::env::current_dir().expect("Failed to get current directory");
    if let Some(wd) = args.working_dir {
        working_dir = PathBuf::from(wd);
        working_dir = working_dir.canonicalize().unwrap();
    }
    if !working_dir.exists() {
        panic!("The specified working directory does not exist");
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
            let mut conf = config::Config::from_path(&working_dir);
            if conf.banner {
                println!("{}", BANNER);
            }
            match args.command {
                Some(Command::Import { path }) => {
                    importdots::import_dots(&path, &mut conf, &working_dir);
                }
                Some(Command::Copy {}) => {
                    copydots::copy_dots(&conf, &working_dir);
                }
                Some(Command::Update {}) => {
                    importdots::backup_dots(&conf, &working_dir);
                }
                _ => {
                    println!("Unknown command. Use --help for more information.");
                }
            }
        }
    }
}

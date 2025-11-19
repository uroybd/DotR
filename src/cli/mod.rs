use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::config::{self, Config};

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Command>,
    #[clap(short, long, global = true)]
    pub working_dir: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init(InitArgs),
    Import(ImportArgs),
    Deploy(DeployArgs),
    Update(UpdateArgs),
}

#[derive(Debug, Args)]
#[command(name = "init", about = "Intialize dotfiles repository.")]
pub struct InitArgs {}

#[derive(Debug, Args)]
#[command(name = "import", about = "Import dotfile and update configuration.")]
pub struct ImportArgs {
    #[arg(value_name = "IMPORT_PATH")]
    pub path: String,
}

#[derive(Debug, Args)]
#[command(name = "deploy", about = "Deploy dotfiles from repository.")]
pub struct DeployArgs {
    #[arg(num_args(0..), short, long)]
    pub packages: Option<Vec<String>>,
}

#[derive(Debug, Args)]
#[command(name = "update", about = "Update dotfiles to repository.")]
pub struct UpdateArgs {
    #[arg(num_args(0..), short, long)]
    pub packages: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub working_dir: PathBuf,
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
    let ctx = Context {
        working_dir: working_dir.clone(),
    };
    // Print working directory
    // Print full working directory path
    match args.command {
        Some(Command::Init(_)) => {
            println!("Initializing configuration...");
            match Config::init(&working_dir) {
                Ok(_) => {
                    println!("Configuration initialized successfully.");
                }
                Err(e) => {
                    eprintln!("Failed to initialize configuration: {}", e);
                }
            }
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
                Some(Command::Import(args)) => {
                    conf.import_package(&args.path, &working_dir);
                }
                Some(Command::Deploy(args)) => {
                    conf.deploy_packages(&ctx, &args);
                }
                Some(Command::Update(args)) => {
                    conf.backup_packages(&ctx, &args);
                }
                _ => {
                    println!("Unknown command. Use --help for more information.");
                }
            }
        }
    }
}

use std::{collections::HashMap, path::PathBuf};

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
    PrintVars(PrintVarsArgs),
}

#[derive(Debug, Args)]
#[command(name = "init", about = "Intialize dotfiles repository.")]
pub struct InitArgs {}

#[derive(Debug, Args)]
#[command(name = "print-vars", about = "Print all user variables.")]
pub struct PrintVarsArgs {}

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
    pub variables: HashMap<String, String>,
}

impl Context {
    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }
    pub fn new(working_dir: PathBuf) -> Self {
        let mut ctx = Self {
            working_dir,
            variables: HashMap::new(),
        };
        // Add environment variables to context variables
        for (key, value) in std::env::vars() {
            ctx.variables.insert(key, value);
        }
        ctx
    }
    pub fn print_variables(&self) {
        println!("User Variables:");
        if self.variables.is_empty() {
            println!("  (none)");
        } else {
            let mut vars: Vec<_> = self.variables.iter().collect();
            vars.sort_by_key(|(k, _)| k.as_str());
            for (key, value) in vars {
                println!("  {} = {}", key, value);
            }
        }
    }
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
    let mut ctx = Context {
        working_dir: working_dir.clone(),
        variables: HashMap::new(),
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
            ctx.variables = conf.variables.clone();
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
                Some(Command::PrintVars(_)) => {
                    println!("User Variables:");
                    ctx.print_variables();
                }
                _ => {
                    println!("Unknown command. Use --help for more information.");
                }
            }
        }
    }
}

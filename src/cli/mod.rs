use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::{
    config::{self, Config},
    context::Context,
};

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
pub struct PrintVarsArgs {
    #[arg(short, long)]
    pub profile: Option<String>,
}

#[derive(Debug, Args)]
#[command(name = "import", about = "Import dotfile and update configuration.")]
pub struct ImportArgs {
    #[arg(value_name = "IMPORT_PATH")]
    pub path: String,

    #[arg(short, long)]
    pub profile: Option<String>,
}

#[derive(Debug, Args)]
#[command(name = "deploy", about = "Deploy dotfiles from repository.")]
pub struct DeployArgs {
    #[arg(num_args(0..), short, long)]
    pub packages: Option<Vec<String>>,

    #[arg(short, long)]
    pub profile: Option<String>,
}

#[derive(Debug, Args)]
#[command(name = "update", about = "Update dotfiles to repository.")]
pub struct UpdateArgs {
    #[arg(num_args(0..), short, long)]
    pub packages: Option<Vec<String>>,

    #[arg(short, long)]
    pub profile: Option<String>,
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
        // Only canonicalize if the path exists
        if working_dir.exists() {
            working_dir = working_dir.canonicalize().unwrap();
        }
    }

    // For Init command, we allow non-existent directories
    if !working_dir.exists() && !matches!(args.command, Some(Command::Init(_))) {
        panic!("The specified working directory does not exist");
    }

    // Create working directory for Init if it doesn't exist
    if matches!(args.command, Some(Command::Init(_))) && !working_dir.exists() {
        std::fs::create_dir_all(&working_dir).expect("Failed to create working directory");
    }

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
            // Start with environment variables from Context::new()
            let mut ctx = Context::new(&working_dir);
            ctx.extend_variables(conf.variables.clone());

            // Merge config variables, which override environment variables
            match args.command {
                Some(Command::Import(args)) => {
                    let profile_name = args.profile.clone();
                    if let Some(p_name) = profile_name.clone()
                        && let Some(profile) = conf.profiles.get(&p_name)
                    {
                        ctx.set_profile(Some(profile.clone()));
                    }
                    conf.import_package(&args.path, &ctx, &profile_name);
                }
                Some(Command::Deploy(args)) => {
                    let profile_name = args.profile.clone();
                    if let Some(p_name) = profile_name.clone()
                        && let Some(profile) = conf.profiles.get(&p_name)
                    {
                        ctx.set_profile(Some(profile.clone()));
                    }
                    if profile_name.is_some() && ctx.profile.is_none() {
                        eprintln!(
                            "Warning: Profile '{}' not found in configuration.",
                            profile_name.unwrap()
                        );
                        // Exit program
                        std::process::exit(1);
                    }
                    conf.deploy_packages(&ctx, &args);
                }
                Some(Command::Update(args)) => {
                    let profile_name = args.profile.clone();
                    if let Some(p_name) = profile_name.clone()
                        && let Some(profile) = conf.profiles.get(&p_name)
                    {
                        ctx.set_profile(Some(profile.clone()));
                    }
                    if profile_name.is_some() && ctx.profile.is_none() {
                        eprintln!(
                            "Warning: Profile '{}' not found in configuration.",
                            profile_name.unwrap()
                        );
                        // Exit program
                        std::process::exit(1);
                    }
                    conf.backup_packages(&ctx, &args);
                }
                Some(Command::PrintVars(args)) => {
                    let profile_name = args.profile.clone();
                    if let Some(p_name) = profile_name.clone()
                        && let Some(profile) = conf.profiles.get(&p_name)
                    {
                        ctx.set_profile(Some(profile.clone()));
                    }
                    if profile_name.is_some() && ctx.profile.is_none() {
                        eprintln!(
                            "Warning: Profile '{}' not found in configuration.",
                            profile_name.unwrap()
                        );
                        // Exit program
                        std::process::exit(1);
                    }
                    ctx.print_variables();
                }
                _ => {
                    println!("Unknown command. Use --help for more information.");
                }
            }
        }
    }
}

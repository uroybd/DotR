use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::{
    config::{self, Config},
    context::Context,
    profile::Profile,
};

#[derive(Debug, Parser)]
#[command(version)]
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
    Deploy(DeployUpdateArgs),
    Update(DeployUpdateArgs),
    Diff(DeployUpdateArgs),
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
pub struct DeployUpdateArgs {
    #[arg(num_args(0..), short, long)]
    pub packages: Option<Vec<String>>,

    #[arg(short = 'P', long)]
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

pub fn run_cli(args: Cli) -> Result<(), anyhow::Error> {
    let mut working_dir = std::env::current_dir()?;
    if let Some(wd) = args.working_dir {
        working_dir = PathBuf::from(wd);
        // Only canonicalize if the path exists
        if working_dir.exists() {
            working_dir = working_dir.canonicalize()?;
        }
    }

    // For Init command, we allow non-existent directories
    if !working_dir.exists() && !matches!(args.command, Some(Command::Init(_))) {
        anyhow::bail!("The specified working directory does not exist");
    }

    // Create working directory for Init if it doesn't exist
    if matches!(args.command, Some(Command::Init(_))) && !working_dir.exists() {
        std::fs::create_dir_all(&working_dir)?;
    }

    // Print working directory
    // Print full working directory path
    match args.command {
        Some(Command::Init(_)) => {
            println!("Initializing configuration...");
            Config::init(&working_dir)?;
            println!("Configuration initialized successfully.");
        }
        None => {
            println!("No command provided. Use --help for more information.");
        }
        Some(_) => {
            let mut conf = config::Config::from_path(&working_dir)?;
            if conf.banner {
                println!("{}", BANNER);
            }
            // Start with environment variables from Context::new()
            let mut ctx = Context::new(&working_dir)?;
            ctx.extend_variables(conf.variables.clone());
            ctx.get_prompted_variables(&conf.prompts)?;
            let context_vars = ctx.get_context_variables();

            // Merge config variables, which override environment variables
            match args.command {
                Some(Command::Import(args)) => {
                    let (profile_name, profile) =
                        conf.get_profile_details(&args.profile, &context_vars);
                    ctx.set_profile(profile);
                    conf.import_package(&args.path, &ctx, &profile_name)?;
                }
                Some(Command::Deploy(args)) => {
                    let (profile_name, profile) =
                        conf.get_profile_details(&args.profile, &context_vars);
                    validate_profile_exists(&profile_name, &profile)?;
                    ctx.set_profile(profile);
                    conf.deploy_packages(&ctx, &args)?;
                }
                Some(Command::Update(args)) => {
                    let (profile_name, profile) =
                        conf.get_profile_details(&args.profile, &context_vars);
                    validate_profile_exists(&profile_name, &profile)?;
                    ctx.set_profile(profile);
                    conf.backup_packages(&ctx, &args)?;
                }
                Some(Command::Diff(args)) => {
                    let (profile_name, profile) =
                        conf.get_profile_details(&args.profile, &context_vars);
                    validate_profile_exists(&profile_name, &profile)?;
                    ctx.set_profile(profile);
                    conf.diff_packages(&ctx, &args)?;
                }
                Some(Command::PrintVars(args)) => {
                    let (profile_name, profile) =
                        conf.get_profile_details(&args.profile, &context_vars);
                    validate_profile_exists(&profile_name, &profile)?;
                    ctx.set_profile(profile);
                    ctx.print_variables();
                }
                _ => {
                    println!("Unknown command. Use --help for more information.");
                }
            }
        }
    }
    Ok(())
}

fn validate_profile_exists(
    profile_name: &Option<String>,
    profile: &Option<Profile>,
) -> Result<(), anyhow::Error> {
    if profile_name.is_some() && profile.is_none() {
        anyhow::bail!(
            "Profile '{}' not found in configuration",
            profile_name.as_ref().unwrap()
        );
    }
    Ok(())
}

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::{
    config::{self, Config},
    context::Context,
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
    Deploy(DeployArgs),
    Update(UpdateArgs),
    Diff(DiffArgs),
    PrintVars(PrintVarsArgs),
}

#[derive(Debug, Args)]
#[command(name = "init", about = "Intialize dotfiles repository.")]
pub struct InitArgs {}

#[derive(Debug, Args)]
#[command(name = "print-vars", about = "Print all user variables.")]
#[derive(Default)]
pub struct PrintVarsArgs {
    #[arg(short, long)]
    pub profile: Option<String>,
}

impl PrintVarsArgs {
    pub fn new(profile: Option<String>) -> Self {
        Self { profile }
    }
}

#[derive(Debug, Args)]
#[command(name = "import", about = "Import dotfile and update configuration.")]
#[derive(Default)]
pub struct ImportArgs {
    #[arg(value_name = "IMPORT_PATH")]
    pub path: String,

    #[arg(short, long)]
    pub name: Option<String>,

    #[arg(short, long)]
    pub profile: Option<String>,
}

impl ImportArgs {
    pub fn new(path: String, name: Option<String>, profile: Option<String>) -> Self {
        Self {
            path,
            name,
            profile,
        }
    }
}

#[derive(Debug, Args)]
#[command(name = "diff", about = "Show differences between dotfiles.")]
pub struct DiffArgs {
    #[arg(num_args(0..), short, long)]
    pub packages: Option<Vec<String>>,

    #[arg(short = 'P', long)]
    pub profile: Option<String>,

    #[arg(long, default_value_t = false)]
    pub ignore_errors: bool,
}

#[derive(Debug, Args)]
#[command(name = "update", about = "Update dotfiles from deployed versions.")]
#[derive(Default)]
pub struct UpdateArgs {
    #[arg(num_args(0..), short, long)]
    pub packages: Option<Vec<String>>,

    #[arg(short = 'P', long)]
    pub profile: Option<String>,

    #[arg(long, default_value_t = false)]
    pub ignore_errors: bool,

    #[arg(long, default_value_t = false)]
    pub clean: bool,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

impl UpdateArgs {
    pub fn new(
        packages: Option<Vec<String>>,
        profile: Option<String>,
        ignore_errors: bool,
        clean: bool,
        dry_run: bool,
    ) -> Self {
        Self {
            packages,
            profile,
            ignore_errors,
            clean,
            dry_run,
        }
    }
}

#[derive(Debug, Args)]
#[command(name = "deploy", about = "Deploy dotfiles from repository.")]
#[derive(Default)]
pub struct DeployArgs {
    #[arg(num_args(0..), short, long)]
    pub packages: Option<Vec<String>>,

    #[arg(short = 'P', long)]
    pub profile: Option<String>,

    #[arg(long, default_value_t = false)]
    pub ignore_errors: bool,

    #[arg(long, default_value_t = false)]
    pub clean: bool,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

impl DeployArgs {
    pub fn new(
        packages: Option<Vec<String>>,
        profile: Option<String>,
        ignore_errors: bool,
        clean: bool,
        dry_run: bool,
    ) -> Self {
        Self {
            packages,
            profile,
            ignore_errors,
            clean,
            dry_run,
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

pub fn run_cli(args: Cli) -> Result<(), anyhow::Error> {
    let mut working_dir = std::env::current_dir()?;
    if let Some(wd) = args.working_dir {
        working_dir = PathBuf::from(wd);
    }

    if !working_dir.exists() {
        if matches!(args.command, Some(Command::Init(_))) {
            std::fs::create_dir_all(&working_dir)?;
        } else {
            anyhow::bail!("The specified working directory does not exist");
        }
    } else {
        working_dir = working_dir.canonicalize()?;
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

            // Merge config variables, which override environment variables
            match args.command {
                Some(Command::Import(args)) => {
                    conf.set_profile_context(&args.profile, &mut ctx, true)?;
                    conf.import_package(&args, &ctx)?;
                }
                Some(Command::Deploy(args)) => {
                    conf.set_profile_context(&args.profile, &mut ctx, false)?;
                    ctx.get_prompted_variables(&conf, &args.packages)?;
                    conf.deploy_packages(&ctx, &args)?;
                }
                Some(Command::Update(args)) => {
                    conf.set_profile_context(&args.profile, &mut ctx, false)?;
                    ctx.get_prompted_variables(&conf, &args.packages)?;
                    conf.backup_packages(&ctx, &args)?;
                }
                Some(Command::Diff(args)) => {
                    conf.set_profile_context(&args.profile, &mut ctx, false)?;
                    ctx.get_prompted_variables(&conf, &args.packages)?;
                    conf.diff_packages(&ctx, &args)?;
                }
                Some(Command::PrintVars(args)) => {
                    conf.set_profile_context(&args.profile, &mut ctx, false)?;
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

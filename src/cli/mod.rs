use std::path::{Path, PathBuf};

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

#[derive(Debug, Args, Default)]
#[command(name = "print-vars", about = "Print all user variables.")]
pub struct PrintVarsArgs {
    #[arg(short, long)]
    pub profile: Option<String>,
}

#[derive(Debug, Args, Default)]
#[command(name = "import", about = "Import dotfile and update configuration.")]
pub struct ImportArgs {
    #[arg(value_name = "IMPORT_PATH")]
    pub path: String,

    #[arg(short, long)]
    pub name: Option<String>,

    #[arg(short, long)]
    pub profile: Option<String>,
}

#[derive(Debug, Args, Default)]
#[command(name = "diff", about = "Show differences between dotfiles.")]
pub struct DiffArgs {
    #[arg(num_args(0..), short, long)]
    pub packages: Option<Vec<String>>,

    #[arg(short = 'P', long)]
    pub profile: Option<String>,

    #[arg(long, default_value_t = false)]
    pub ignore_errors: bool,
}

#[derive(Debug, Args, Default)]
#[command(name = "update", about = "Update dotfiles from deployed versions.")]
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

#[derive(Debug, Args, Default)]
#[command(name = "deploy", about = "Deploy dotfiles from repository.")]
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
        anyhow::bail!("The specified working directory does not exist");
    }
    working_dir = working_dir.canonicalize()?;

    // Print working directory
    // Print full working directory path
    match args.command {
        Some(Command::Init(_)) => {
            println!("Initializing configuration...");
            Config::init(&working_dir)?;
            println!("Configuration initialized successfully.");
        }
        Some(Command::Import(args)) => {
            let (mut conf, ctx) = init_config(&working_dir, &args.profile, true)?;
            conf.import_package(&args, &ctx)?;
        }
        Some(Command::Deploy(args)) => {
            let (conf, mut ctx) = init_config(&working_dir, &args.profile, false)?;
            ctx.get_prompted_variables(&conf, &args.packages)?;
            conf.deploy_packages(&ctx, &args)?;
        }
        Some(Command::Update(args)) => {
            let (conf, mut ctx) = init_config(&working_dir, &args.profile, false)?;
            ctx.get_prompted_variables(&conf, &args.packages)?;
            conf.backup_packages(&ctx, &args)?;
        }
        Some(Command::Diff(args)) => {
            let (conf, mut ctx) = init_config(&working_dir, &args.profile, false)?;
            ctx.get_prompted_variables(&conf, &args.packages)?;
            conf.diff_packages(&ctx, &args)?;
        }
        Some(Command::PrintVars(args)) => {
            let (_, ctx) = init_config(&working_dir, &args.profile, false)?;
            ctx.print_variables();
        }
        None => {
            println!("No command provided. Use --help for more information.");
        }
    }
    Ok(())
}

fn init_config(
    working_dir: &Path,
    profile: &Option<String>,
    create_if_missing: bool,
) -> anyhow::Result<(Config, Context)> {
    let mut conf = config::Config::from_path(working_dir)?;
    if conf.banner {
        println!("{}", BANNER);
    }
    // Start with environment variables from Context::new()
    let mut ctx = Context::new(working_dir)?;
    ctx.extend_variables(conf.variables.clone());
    conf.set_profile_context(profile, &mut ctx, create_if_missing)?;
    Ok((conf, ctx))
}

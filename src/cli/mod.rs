use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand};

use crate::{
    config::{self, Config},
    context::Context,
};

#[derive(Debug, Parser)]
#[command(name = "dotr", version, about)]
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
    Remove(RemovePackageArgs),
    PrintVars(PrintVarsArgs),
    Packages(PackagesArgs),
    Profiles(ProfilesArgs),
}

#[derive(Debug, Args, Default)]
#[command(name = "remove", about = "Remove a managed package.")]
pub struct RemovePackageArgs {
    #[arg(num_args(0..))]
    pub packages: Option<Vec<String>>,

    #[arg(short, long, default_value_t = false)]
    pub force: bool,

    #[arg(long, default_value_t = false)]
    pub remove_orphans: bool,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    #[arg(short = 'P', long)]
    pub profile: Option<String>,
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

#[derive(Debug, Args)]
#[command(name = "import", about = "Import dotfile and update configuration.")]
#[derive(Default)]
pub struct ImportArgs {
    #[arg(value_name = "IMPORT_PATH")]
    pub path: String,

    #[arg(short, long, default_value_t = false)]
    pub symlink: bool,

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

    #[arg(long)]
    pub clean: Option<bool>,

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

    #[arg(long)]
    pub clean: Option<bool>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    #[arg(long, default_value_t = false)]
    pub skip_actions: bool,

    #[arg(long, default_value_t = false)]
    pub skip_pre_actions: bool,

    #[arg(long, default_value_t = false)]
    pub skip_post_actions: bool,

    #[arg(long, default_value_t = false)]
    pub ignore_dependencies: bool,
}

#[derive(Debug, Args, Default)]
#[command(
    name = "packages",
    about = "Manage packages (list, import, deploy, update, remove, diff)."
)]
pub struct PackagesArgs {
    #[arg(short = 'P', long)]
    pub profile: Option<String>,
    #[clap(subcommand)]
    pub command: Option<PackagesCommand>,
}

#[derive(Debug, Subcommand)]
pub enum PackagesCommand {
    List(PackagesListArgs),
    Import(ImportArgs),
    Deploy(DeployArgs),
    Update(UpdateArgs),
    Remove(RemovePackageArgs),
    Diff(DiffArgs),
}

#[derive(Debug, Args, Default)]
pub struct PackagesListArgs {
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,
}

#[derive(Debug, Args, Default)]
#[command(name = "profiles", about = "Manage profiles (list, add, remove).")]
pub struct ProfilesArgs {
    #[clap(subcommand)]
    pub command: Option<ProfilesCommand>,
}

#[derive(Debug, Subcommand)]
pub enum ProfilesCommand {
    Add(ProfilesAddArgs),
    List(ProfilesListArgs),
    Remove(ProfileRemoveArgs),
}

#[derive(Debug, Args, Default)]
pub struct ProfileRemoveArgs {
    #[arg(value_name = "PROFILE_NAME")]
    pub name: String,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    #[arg(long, default_value_t = false)]
    pub remove_orphans: bool,
}

#[derive(Debug, Args, Default)]
pub struct ProfilesListArgs {
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,
}

#[derive(Debug, Args, Default)]
pub struct ProfilesAddArgs {
    #[arg(value_name = "PROFILE_NAME")]
    pub name: String,

    #[arg(long, default_value_t = false)]
    pub set_as_current: bool,
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
        Some(Command::Remove(args)) => {
            let (mut conf, ctx) = init_config(&working_dir, &args.profile, false)?;
            conf.remove_packages(&args, &ctx)?;
        }
        Some(Command::Packages(args)) => {
            let (mut conf, mut ctx) = init_config(&working_dir, &args.profile, false)?;
            match args.command {
                Some(PackagesCommand::List(args)) => {
                    conf.list_packages(&ctx, &args)?;
                }
                Some(PackagesCommand::Import(import_args)) => {
                    conf.import_package(&import_args, &ctx)?;
                }
                Some(PackagesCommand::Deploy(deploy_args)) => {
                    ctx.get_prompted_variables(&conf, &deploy_args.packages)?;
                    conf.deploy_packages(&ctx, &deploy_args)?;
                }
                Some(PackagesCommand::Update(update_args)) => {
                    ctx.get_prompted_variables(&conf, &update_args.packages)?;
                    conf.backup_packages(&ctx, &update_args)?;
                }
                Some(PackagesCommand::Diff(diff_args)) => {
                    ctx.get_prompted_variables(&conf, &diff_args.packages)?;
                    conf.diff_packages(&ctx, &diff_args)?;
                }
                Some(PackagesCommand::Remove(remove_args)) => {
                    conf.remove_packages(&remove_args, &ctx)?;
                }
                None => {
                    println!("No packages command provided. Use --help for more information.");
                }
            }
        }
        Some(Command::Profiles(args)) => {
            let (mut conf, mut ctx) = init_config(&working_dir, &None, false)?;
            match args.command {
                Some(ProfilesCommand::List(list_args)) => {
                    conf.list_profiles(&list_args)?;
                }
                Some(ProfilesCommand::Add(add_args)) => {
                    conf.add_profile(&add_args, &mut ctx)?;
                }
                Some(ProfilesCommand::Remove(remove_args)) => {
                    conf.remove_profile(&remove_args, &ctx)?;
                }
                None => {
                    println!("No profiles command provided. Use --help for more information.");
                }
            }
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
    let (ctx, profile_created) = Context::new(working_dir, &conf, profile, create_if_missing)?;
    if profile_created {
        conf.update_profiles(&ctx.profile, &ctx)?;
    }
    Ok((conf, ctx))
}

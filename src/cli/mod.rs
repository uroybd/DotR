use std::path::{Path, PathBuf};

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};

use crate::{
    config::{self, Config},
    context::Context,
    utils::{LogLevel, cprintln},
};

#[derive(Debug, Parser)]
#[command(
    name = "dotr",
    version,
    about,
    long_about = "DotR keeps a repository of your dotfiles separate from where they're \
actually used, and manages copying or symlinking them into place.\n\n\
Packages describe individual files or directories with a source and \
destination; profiles let the same repository describe different \
machines (work, home, server) with different packages, variables, and \
destinations. Config files can embed Tera template syntax compiled at \
deploy time, and pre/post shell hooks can run around each deployment."
)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Command>,
    /// Run as if invoked from this directory instead of the current one.
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
    DumpUserVars(DumpUserVarsArgs),
    Packages(PackagesArgs),
    Profiles(ProfilesArgs),
    Completions(CompletionsArgs),
}

/// Shells `dotr completions` can generate a script for, plus the Carapace spec format.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    Elvish,
    #[value(name = "powershell")]
    PowerShell,
    Nushell,
    /// Carapace spec (YAML), not a shell script — see the `carapace` case below.
    Carapace,
}

/// The Carapace spec YAML, kept in sync by hand alongside the `Cli` definition
/// (unlike the other shells, which `clap_complete` generates automatically).
const CARAPACE_SPEC: &str = include_str!("../../completions/carapace/dotr.yaml");

#[derive(Debug, Args)]
#[command(
    name = "completions",
    about = "Print a shell completion script to stdout.",
    long_about = "Prints a completion script for the given shell to stdout. Redirect it \
into your shell's completion directory, e.g.:\n\n\
dotr completions bash > ~/.local/share/bash-completion/completions/dotr\n\n\
dotr completions zsh > ~/.zfunc/_dotr\n\n\
dotr completions fish > ~/.config/fish/completions/dotr.fish\n\n\
dotr completions nushell > ~/.config/nushell/completions/dotr.nu\n\n\
`carapace` is not a shell but a spec file for the Carapace completion \
engine (https://carapace.sh), which adds dynamic completion of real \
package/profile names from config.toml on top of the static commands and \
flags below:\n\n\
dotr completions carapace > ~/.config/carapace/specs/dotr.yaml"
)]
pub struct CompletionsArgs {
    /// Shell (or `carapace` for a Carapace spec) to generate completions for.
    pub shell: CompletionShell,
}

#[derive(Debug, Args, Default)]
#[command(
    name = "remove",
    about = "Remove a managed package.",
    long_about = "Deletes a package's entry from config.toml and its files from \
dotfiles/. Refuses to remove a package that another package or profile still depends \
on unless --force is given."
)]
pub struct RemovePackageArgs {
    /// Packages to remove.
    #[arg(num_args(0..))]
    pub packages: Option<Vec<String>>,

    /// Remove even if another package or profile still depends on it.
    #[arg(short, long, default_value_t = false)]
    pub force: bool,

    /// Also remove dependencies left unreferenced after this removal.
    #[arg(long, default_value_t = false)]
    pub remove_orphans: bool,

    /// Preview what would be removed without making changes.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Profile context used to resolve dependencies.
    #[arg(short = 'P', long)]
    pub profile: Option<String>,
}

#[derive(Debug, Args)]
#[command(
    name = "init",
    about = "Initialize dotfiles repository.",
    long_about = "Creates config.toml, a dotfiles/ directory, and a .gitignore in the \
working directory. Safe to re-run - if config.toml already exists, it's left untouched."
)]
pub struct InitArgs {}

#[derive(Debug, Args, Default)]
#[command(
    name = "print-vars",
    about = "Print all user variables.",
    long_about = "Prints every variable resolved for the given (or default) profile - \
config, environment, package, profile, and prompt-sourced - useful for debugging why a \
template rendered the way it did."
)]
pub struct PrintVarsArgs {
    /// Resolve variables for this profile instead of the default.
    #[arg(short, long)]
    pub profile: Option<String>,
}

#[derive(Debug, Args, Default)]
#[command(
    name = "dump-user-vars",
    about = "Dump user variables to a TOML file.",
    long_about = "Resolves every prompted (user) variable for the given (or default) \
profile - prompting for anything not yet answered, through whichever backend is \
configured (file, keychain, or Bitwarden, via prompt_backend) - and writes the full \
set as TOML to stdout, or to --output if given. Unlike .uservariables.toml, this \
includes keychain- and Bitwarden-backed values too, so it's an escape hatch for \
backup or migrating a value from one backend to another: copy an entry into \
.uservariables.toml and drop (or change) prompt_backend to move it there."
)]
pub struct DumpUserVarsArgs {
    /// Resolve variables for this profile instead of the default.
    #[arg(short, long)]
    pub profile: Option<String>,

    /// Only resolve prompts relevant to these packages. Omit to use the
    /// active profile's packages.
    #[arg(num_args(0..), long)]
    pub packages: Option<Vec<String>>,

    /// Write the dump to this file instead of stdout.
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(Debug, Args)]
#[command(
    name = "import",
    about = "Import dotfile and update configuration.",
    long_about = "Copies a file or directory into the repository and registers it as a \
package in config.toml, with a source and destination. Use --symlink to also deploy it \
immediately as a symlink instead of a copy."
)]
#[derive(Default)]
pub struct ImportArgs {
    /// Path to the file or directory to import.
    #[arg(value_name = "IMPORT_PATH")]
    pub path: String,

    /// Deploy as a symlink instead of a copy, and deploy immediately.
    #[arg(short, long, default_value_t = false)]
    pub symlink: bool,

    /// Override the auto-derived package name.
    #[arg(short, long)]
    pub name: Option<String>,

    /// Add the package to this profile's dependencies instead of default.
    #[arg(short, long)]
    pub profile: Option<String>,
}

#[derive(Debug, Args, Default)]
#[command(
    name = "diff",
    about = "Show differences between dotfiles.",
    long_about = "Shows a colored, line-by-line diff between what's in the repository \
and what's currently deployed, without changing anything."
)]
pub struct DiffArgs {
    /// Diff only these packages. Omit to diff the active profile's packages.
    #[arg(num_args(0..), short, long)]
    pub packages: Option<Vec<String>>,

    /// Use this profile instead of the resolved default.
    #[arg(short = 'P', long)]
    pub profile: Option<String>,

    /// Keep diffing remaining packages if one fails.
    #[arg(long, default_value_t = false)]
    pub ignore_errors: bool,
}

#[derive(Debug, Args, Default)]
#[command(
    name = "update",
    about = "Update dotfiles from deployed versions.",
    long_about = "Copies changes made to deployed files back into the repository, so \
dotfiles/ keeps reflecting what's actually deployed. Templated packages are skipped, \
since the template file is the source of truth."
)]
pub struct UpdateArgs {
    /// Update only these packages. Omit to update the active profile's packages.
    #[arg(num_args(0..), short, long)]
    pub packages: Option<Vec<String>>,

    /// Use this profile instead of the resolved default.
    #[arg(short = 'P', long)]
    pub profile: Option<String>,

    /// Keep updating remaining packages if one fails.
    #[arg(long, default_value_t = false)]
    pub ignore_errors: bool,

    /// Override the clean mode setting for this run.
    #[arg(long)]
    pub clean: Option<bool>,

    /// Preview without changing anything.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Args, Default)]
#[command(
    name = "deploy",
    about = "Deploy dotfiles from repository.",
    long_about = "Copies (or symlinks) every selected package from the repository to \
its destination. Selects the active profile's packages by default, or a specific set \
via --packages."
)]
pub struct DeployArgs {
    /// Deploy only these packages (plus dependencies, unless
    /// --ignore-dependencies). Omit to deploy the active profile's packages.
    #[arg(num_args(0..), short, long)]
    pub packages: Option<Vec<String>>,

    /// Use this profile instead of the resolved default.
    #[arg(short = 'P', long)]
    pub profile: Option<String>,

    /// Keep deploying remaining packages if one fails.
    #[arg(long, default_value_t = false)]
    pub ignore_errors: bool,

    /// Override the clean mode setting for this run.
    #[arg(long)]
    pub clean: Option<bool>,

    /// Preview without changing anything.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Skip both pre- and post-actions.
    #[arg(long, default_value_t = false)]
    pub skip_actions: bool,

    /// Skip only pre-actions.
    #[arg(long, default_value_t = false)]
    pub skip_pre_actions: bool,

    /// Skip only post-actions.
    #[arg(long, default_value_t = false)]
    pub skip_post_actions: bool,

    /// Deploy only the selected packages, without pulling in dependencies.
    #[arg(long, default_value_t = false)]
    pub ignore_dependencies: bool,
}

#[derive(Debug, Args, Default)]
#[command(
    name = "packages",
    about = "Manage packages (list, import, deploy, update, remove, diff).",
    long_about = "Groups package-scoped commands under one namespace. Each subcommand \
behaves the same as its top-level equivalent, plus list for viewing all managed packages."
)]
pub struct PackagesArgs {
    /// Profile context for the packages subcommand.
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
#[command(
    name = "list",
    about = "List all managed packages.",
    long_about = "Lists every package currently tracked in config.toml. Use --verbose \
to also show each package's source, destination, and other fields. Use --plain for a bare, \
newline-separated list of names with no other output, suitable for scripting or shell \
completion."
)]
pub struct PackagesListArgs {
    /// Show detailed information for each package.
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    /// Print only package names, one per line, with no other output.
    #[arg(long, default_value_t = false, conflicts_with = "verbose")]
    pub plain: bool,
}

#[derive(Debug, Args, Default)]
#[command(
    name = "profiles",
    about = "Manage profiles (list, add, remove).",
    long_about = "Profiles describe an environment (work, home, a server) as a set of \
packages, variables, and prompts, so the same repository can deploy differently \
depending on which profile is active."
)]
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
#[command(
    name = "remove",
    about = "Remove a profile.",
    long_about = "Deletes a profile from config.toml. The default profile cannot be \
removed. Use --remove-orphans to also clean up packages that were only referenced by \
this profile."
)]
pub struct ProfileRemoveArgs {
    /// Name of the profile to remove.
    #[arg(value_name = "PROFILE_NAME")]
    pub name: String,

    /// Preview what would be removed without making changes.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Also remove packages that were only referenced by this profile.
    #[arg(long, default_value_t = false)]
    pub remove_orphans: bool,
}

#[derive(Debug, Args, Default)]
#[command(
    name = "list",
    about = "List all profiles.",
    long_about = "Lists every profile defined in config.toml. Use --verbose to also \
show each profile's dependencies and variables. Use --plain for a bare, newline-separated \
list of names with no other output, suitable for scripting or shell completion."
)]
pub struct ProfilesListArgs {
    /// Show detailed information for each profile.
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    /// Print only profile names, one per line, with no other output.
    #[arg(long, default_value_t = false, conflicts_with = "verbose")]
    pub plain: bool,
}

#[derive(Debug, Args, Default)]
#[command(
    name = "add",
    about = "Add a new profile.",
    long_about = "Creates a new, empty profile in config.toml. Use --set-as-current to \
make it the default on this machine by writing DOTR_PROFILE into .uservariables.toml."
)]
pub struct ProfilesAddArgs {
    /// Name for the new profile.
    #[arg(value_name = "PROFILE_NAME")]
    pub name: String,

    /// Save this profile as the default (writes DOTR_PROFILE to .uservariables.toml).
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
            let (mut conf, ctx) = init_config(&working_dir, &args.profile, &None, true, true)?;
            conf.import_package(&args, &ctx)?;
        }
        Some(Command::Deploy(args)) => {
            let (conf, mut ctx) =
                init_config(&working_dir, &args.profile, &args.packages, false, true)?;
            ctx.get_prompted_variables(&conf, &args.packages)?;
            conf.deploy_packages(&ctx, &args)?;
        }
        Some(Command::Update(args)) => {
            let (conf, mut ctx) =
                init_config(&working_dir, &args.profile, &args.packages, false, true)?;
            ctx.get_prompted_variables(&conf, &args.packages)?;
            conf.backup_packages(&ctx, &args)?;
        }
        Some(Command::Diff(args)) => {
            let (conf, mut ctx) =
                init_config(&working_dir, &args.profile, &args.packages, false, true)?;
            ctx.get_prompted_variables(&conf, &args.packages)?;
            conf.diff_packages(&ctx, &args)?;
        }
        Some(Command::PrintVars(args)) => {
            let (_, ctx) = init_config(&working_dir, &args.profile, &None, false, true)?;
            ctx.print_variables();
        }
        Some(Command::DumpUserVars(args)) => {
            let (conf, mut ctx) =
                init_config(&working_dir, &args.profile, &args.packages, false, true)?;
            let resolved = ctx.get_prompted_variables_with_io(
                &conf,
                &args.packages,
                &mut std::io::stdin().lock(),
                &mut std::io::stdout(),
            )?;
            let toml_string = toml::to_string_pretty(&resolved)?;
            match &args.output {
                Some(path) => {
                    std::fs::write(path, &toml_string)?;
                    cprintln(
                        &format!("Wrote {} prompted variable(s) to {}", resolved.len(), path),
                        &LogLevel::Info,
                    );
                }
                None => print!("{}", toml_string),
            }
        }
        Some(Command::Remove(args)) => {
            let (mut conf, ctx) =
                init_config(&working_dir, &args.profile, &args.packages, false, true)?;
            conf.remove_packages(&args, &ctx)?;
        }
        Some(Command::Packages(args)) => {
            let show_banner = !matches!(&args.command, Some(PackagesCommand::List(a)) if a.plain);
            let (mut conf, mut ctx) =
                init_config(&working_dir, &args.profile, &None, false, show_banner)?;
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
            let show_banner = !matches!(&args.command, Some(ProfilesCommand::List(a)) if a.plain);
            let (mut conf, mut ctx) = init_config(&working_dir, &None, &None, false, show_banner)?;
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
        Some(Command::Completions(args)) => {
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            let mut stdout = std::io::stdout();
            match args.shell {
                CompletionShell::Bash => clap_complete::generate(
                    clap_complete::Shell::Bash,
                    &mut cmd,
                    bin_name,
                    &mut stdout,
                ),
                CompletionShell::Zsh => clap_complete::generate(
                    clap_complete::Shell::Zsh,
                    &mut cmd,
                    bin_name,
                    &mut stdout,
                ),
                CompletionShell::Fish => clap_complete::generate(
                    clap_complete::Shell::Fish,
                    &mut cmd,
                    bin_name,
                    &mut stdout,
                ),
                CompletionShell::Elvish => clap_complete::generate(
                    clap_complete::Shell::Elvish,
                    &mut cmd,
                    bin_name,
                    &mut stdout,
                ),
                CompletionShell::PowerShell => clap_complete::generate(
                    clap_complete::Shell::PowerShell,
                    &mut cmd,
                    bin_name,
                    &mut stdout,
                ),
                CompletionShell::Nushell => clap_complete::generate(
                    clap_complete_nushell::Nushell,
                    &mut cmd,
                    bin_name,
                    &mut stdout,
                ),
                CompletionShell::Carapace => print!("{}", CARAPACE_SPEC),
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
    packages: &Option<Vec<String>>,
    create_if_missing: bool,
    show_banner: bool,
) -> anyhow::Result<(Config, Context)> {
    let mut conf = config::Config::from_path(working_dir)?;
    if conf.banner && show_banner {
        println!("{}", BANNER);
    }
    let (ctx, profile_created) =
        Context::new(working_dir, &conf, profile, packages, create_if_missing)?;
    if profile_created {
        conf.update_profiles(&ctx.profile, &ctx)?;
    }
    Ok((conf, ctx))
}

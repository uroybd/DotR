use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use toml::Table;

use crate::{
    cli::{
        DeployArgs, DiffArgs, ImportArgs, PackagesListArgs, ProfileRemoveArgs, ProfilesAddArgs,
        RemovePackageArgs, UpdateArgs,
    },
    context::{Context, USERVARIABLES_FILE},
    out, outln,
    package::{BackupDeployResult, Package, is_templated_str},
    platform::Platform,
    profile::Profile,
    utils::{
        LogLevel, cprintln, get_opt_bool_from_value, get_opt_string_from_value,
        get_table_from_value, is_empty_table,
    },
};

#[cfg(test)]
mod tests;

/// JSON Schema for `config.toml`, referenced via a `#:schema` directive at
/// the top of newly-generated config files (see `Config::init`).
const SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/uroybd/DotR/main/schema/config.schema.json";

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Config {
    pub banner: bool,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub packages: HashMap<String, Package>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub profiles: HashMap<String, Profile>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub platforms: HashMap<String, Platform>,
    #[serde(skip_serializing_if = "is_empty_table")]
    pub variables: Table,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub prompts: HashMap<String, String>,
    pub symlink: bool,
    /// Name of the Bitwarden secure note used when `prompt_backend =
    /// "bitwarden"`. Defaults to `prompt_store::DEFAULT_BITWARDEN_NOTE` when unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitwarden_note: Option<String>,
    /// Repo-wide default backend for every prompt. A profile's own
    /// `prompt_backend` overrides this when that profile is active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_backend: Option<crate::prompt_store::PromptBackendType>,
}

/// What `.gitignore` must list: the machine-local answers file, and the folder symlinked packages are staged in.
pub const GITIGNORE_ENTRIES: [&str; 2] = [USERVARIABLES_FILE, "deployed"];

/// Whether `.uservariables.toml` exists in `cwd` but git would commit it: not ignored, or already tracked (git
/// ignores no tracked file). `false` outside a git repository, or without git, where there is nothing to check.
pub fn uservariables_exposed(cwd: &Path) -> bool {
    if !cwd.join(USERVARIABLES_FILE).exists() {
        return false;
    }
    std::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["check-ignore", "-q", USERVARIABLES_FILE])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.code() == Some(1))
}

#[derive(Clone, Copy)]
pub(crate) enum OpType {
    Backup,
    Deploy,
}

impl Config {
    pub fn from_path(cwd: &Path) -> anyhow::Result<Self> {
        let config_path = cwd.join("config.toml");
        if !config_path.exists() {
            anyhow::bail!("config.toml not found in the current directory");
        }
        let config_content = std::fs::read_to_string(config_path)?;
        let conf_table = config_content.parse::<Table>()?;
        Self::from_table(&conf_table)
    }

    pub fn save(&self, cwd: &Path) -> anyhow::Result<()> {
        let config_content = toml::to_string_pretty(self)?;
        // Every write needs the `#:schema` directive, not just the one from
        // `init()` - otherwise the very next save (import, deploy, profiles
        // add/remove, ...) drops it, since serializing `Config` back to TOML
        // has no way to preserve a leading comment on its own.
        let config_content = format!("#:schema {SCHEMA_URL}\n{config_content}");
        std::fs::write(cwd.join("config.toml"), config_content)?;
        Ok(())
    }

    pub fn from_table(table: &Table) -> anyhow::Result<Self> {
        let mut packages: HashMap<String, Package> = HashMap::new();
        for (key, val) in &get_table_from_value(table.get("packages"), "packages")? {
            let pkg_val = val
                .as_table()
                .ok_or_else(|| anyhow::anyhow!("Package '{key}' must be a table"))?;
            let pkg = Package::from_table(key, pkg_val)
                .map_err(|e| anyhow::anyhow!("Package '{key}': {e}"))?;
            packages.insert(pkg.name.clone(), pkg);
        }

        let platforms = crate::platform::get_platforms_from_table(table.get("platforms"))?;
        let mut profiles: HashMap<String, Profile> = HashMap::new();
        for (key, val) in &get_table_from_value(table.get("profiles"), "profiles")? {
            let prof_val = val
                .as_table()
                .ok_or_else(|| anyhow::anyhow!("Profile '{key}' must be a table"))?;
            let mut profile = Profile::from_table(key, prof_val)
                .map_err(|e| anyhow::anyhow!("Profile '{key}': {e}"))?;
            // Resolved once here, at parse time, rather than on every
            // `Context::new` - a profile's `platform` is fixed for the
            // life of the config, so there's no need to re-look-up its
            // platform's variables on every run.
            if let Some(platform_name) = &profile.platform
                && let Some(platform) = platforms.get(platform_name)
            {
                profile.platform_variables = platform.variables.clone();
            }
            profiles.insert(profile.name.clone(), profile);
        }
        let variables = get_table_from_value(table.get("variables"), "variables")?;
        let prompts = crate::utils::get_string_hashmap_from_value(table.get("prompts"))?;
        let bitwarden_note =
            get_opt_string_from_value(table.get("bitwarden_note"), "bitwarden_note")?;
        let prompt_backend =
            get_opt_string_from_value(table.get("prompt_backend"), "prompt_backend")?
                .as_deref()
                .map(crate::prompt_store::PromptBackendType::parse)
                .transpose()?;
        Ok(Self {
            banner: get_opt_bool_from_value(table.get("banner"), "banner")?.unwrap_or(false),
            symlink: get_opt_bool_from_value(table.get("symlink"), "symlink")?.unwrap_or(false),
            packages,
            profiles,
            variables,
            prompts,
            bitwarden_note,
            prompt_backend,
            platforms,
        })
    }

    pub fn import_package(&mut self, args: &ImportArgs, ctx: &Context) -> anyhow::Result<()> {
        let mut profile = ctx.profile.clone();
        let profile_name = profile.name.clone();
        cprintln(&format!("Importing from {}", args.path), &LogLevel::Info);
        let mut package = Package::from_path(args, &ctx.working_dir)?;
        let pkg_name = package.name.clone();

        let backup_args = crate::cli::UpdateArgs {
            packages: None,
            profile: Some(profile_name.clone()),
            ignore_errors: false,
            clean: Some(false),
            dry_run: false,
            conflict: Some(crate::merge::Resolution::Skip),
            allow_secrets: false,
        };
        package.backup(ctx, &backup_args)?;
        profile.dependencies.push(pkg_name.clone());
        if profile_name != "default" {
            package
                .targets
                .insert(profile_name.clone(), package.dest.clone());
        }
        let should_deploy = args.symlink;
        self.packages.insert(pkg_name.clone(), package);
        self.profiles.insert(profile_name.clone(), profile);
        self.save(&ctx.working_dir)?;
        if should_deploy {
            let pkg = self
                .packages
                .get(&pkg_name)
                .ok_or_else(|| anyhow::anyhow!("Package '{pkg_name}' not found after insertion"))?;
            pkg.deploy(
                ctx,
                &crate::cli::DeployArgs {
                    packages: Some(vec![pkg_name.clone()]),
                    profile: Some(profile_name),
                    ignore_errors: false,
                    clean: Some(false),
                    dry_run: false,
                    skip_actions: false,
                    skip_pre_actions: false,
                    skip_post_actions: false,
                    ignore_dependencies: false,
                },
            )?;
        }
        cprintln(&format!("Package '{pkg_name}' imported"), &LogLevel::Info);
        Ok(())
    }

    pub fn filter_packages(
        &self,
        ctx: &Profile,
        names: &Option<Vec<String>>,
        ignore_dependencies: bool,
    ) -> anyhow::Result<HashMap<String, Package>> {
        let mut packages: HashMap<String, Package> = HashMap::new();
        if let Some(pkg_names) = names {
            for name in pkg_names {
                if let Some(pkg) = self.packages.get(name) {
                    packages.insert(name.clone(), pkg.clone());
                } else {
                    return Err(anyhow::anyhow!("Package '{name}' not found"));
                }
            }
        } else {
            for dep in &ctx.dependencies {
                if let Some(pkg) = self.packages.get(dep) {
                    if !pkg.skip {
                        packages.insert(dep.clone(), pkg.clone());
                    }
                } else {
                    anyhow::bail!("Package '{}' not found for profile '{}'", dep, ctx.name);
                }
            }
        }
        if ignore_dependencies {
            return Ok(packages);
        }
        let mut dependencies: HashMap<String, Package> = HashMap::new();
        for pkg in packages.values() {
            if let Some(deps) = &pkg.dependencies {
                for dep in deps {
                    if let Some(dep_pkg) = self.packages.get(dep) {
                        dependencies.insert(dep.clone(), dep_pkg.clone());
                    } else {
                        anyhow::bail!("Dependency package '{dep}' not found in configuration");
                    }
                }
            }
        }
        packages.extend(dependencies);
        Ok(packages)
    }

    /// Whether any package `update` would touch has a templated deployment path or a templated file (which the
    /// merge renders), and so needs prompted variables resolved before it can run.
    pub fn update_needs_variables(
        &self,
        profile: &Profile,
        names: &Option<Vec<String>>,
        working_dir: &Path,
    ) -> anyhow::Result<bool> {
        for pkg in self.filter_packages(profile, names, false)?.values() {
            if is_templated_str(pkg.raw_dest(profile)) || pkg.has_templated_source(working_dir)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn backup_packages(&self, ctx: &Context, args: &UpdateArgs) -> Result<(), anyhow::Error> {
        cprintln("Backing up packages...", &LogLevel::Info);
        let mut stats: HashMap<BackupDeployResult, u32> = HashMap::new();
        for pkg in self
            .filter_packages(&ctx.profile, &args.packages, false)?
            .values()
        {
            match pkg.backup(ctx, args) {
                Err(e) => {
                    if args.ignore_errors {
                        cprintln(
                            &format!("Error backing up package '{}': {}", pkg.name, e),
                            &LogLevel::Error,
                        );
                        *stats.entry(BackupDeployResult::Failed).or_insert(0) += 1;
                    } else {
                        return Err(e);
                    }
                }
                Ok(res) => {
                    *stats.entry(res).or_insert(0) += 1;
                }
            }
        }
        print_stats(&stats, OpType::Backup);
        Ok(())
    }

    fn get_platform(&self, ctx: &Context) -> Option<&Platform> {
        if let Some(platform_name) = &ctx.profile.platform {
            self.platforms.get(platform_name)
        } else {
            None
        }
    }

    pub fn deploy_packages(&self, ctx: &Context, args: &DeployArgs) -> Result<(), anyhow::Error> {
        cprintln("Deploying packages...", &LogLevel::Info);
        let mut stats: HashMap<BackupDeployResult, u32> = HashMap::new();
        let packages =
            self.filter_packages(&ctx.profile, &args.packages, args.ignore_dependencies)?;
        if packages.is_empty() {
            cprintln("No packages to deploy.", &LogLevel::Info);
            return Ok(());
        }
        self.execute_pre_actions(ctx, args)?;
        for pkg in packages.values() {
            match pkg.deploy(ctx, args) {
                Err(e) => {
                    if args.ignore_errors {
                        cprintln(
                            &format!("Error deploying package '{}': {}", pkg.name, e),
                            &LogLevel::Error,
                        );
                        *stats.entry(BackupDeployResult::Failed).or_insert(0) += 1;
                    } else {
                        return Err(e);
                    }
                }
                Ok(res) => {
                    *stats.entry(res).or_insert(0) += 1;
                }
            }
        }
        self.execute_post_actions(ctx, args)?;
        print_stats(&stats, OpType::Deploy);
        Ok(())
    }

    pub fn execute_pre_actions(
        &self,
        ctx: &Context,
        args: &DeployArgs,
    ) -> Result<(), anyhow::Error> {
        if args.skip_actions || args.skip_pre_actions {
            return Ok(());
        }
        let vars = ctx.profile.get_context_variables(ctx);
        if let Some(platform) = self.get_platform(ctx) {
            for action in &platform.pre_actions {
                crate::utils::execute_action(action, &vars, &ctx.working_dir, args.dry_run)?;
            }
        }
        for action in &ctx.profile.pre_actions {
            crate::utils::execute_action(action, &vars, &ctx.working_dir, args.dry_run)?;
        }
        Ok(())
    }

    pub fn execute_post_actions(
        &self,
        ctx: &Context,
        args: &DeployArgs,
    ) -> Result<(), anyhow::Error> {
        if args.skip_actions || args.skip_post_actions {
            return Ok(());
        }
        let vars = ctx.profile.get_context_variables(ctx);
        for action in &ctx.profile.post_actions {
            crate::utils::execute_action(action, &vars, &ctx.working_dir, args.dry_run)?;
        }
        if let Some(platform) = self.get_platform(ctx) {
            for action in &platform.post_actions {
                crate::utils::execute_action(action, &vars, &ctx.working_dir, args.dry_run)?;
            }
        }
        Ok(())
    }

    pub fn diff_packages(&self, ctx: &Context, args: &DiffArgs) -> Result<(), anyhow::Error> {
        cprintln("Checking differences...", &LogLevel::Info);
        let packages = self.filter_packages(&ctx.profile, &args.packages, false)?;
        if packages.is_empty() {
            cprintln("No packages to diff.", &LogLevel::Info);
            return Ok(());
        }
        for pkg in packages.values() {
            cprintln(&format!("Package: {}", pkg.name), &LogLevel::Info);
            if let Err(e) = pkg.diff(ctx) {
                if args.ignore_errors {
                    cprintln(
                        &format!("Error diffing package '{}': {}", pkg.name, e),
                        &LogLevel::Error,
                    );
                } else {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    pub fn update_profiles(&mut self, profile: &Profile, ctx: &Context) -> anyhow::Result<()> {
        self.profiles
            .entry(profile.name.clone())
            .or_insert_with(|| {
                cprintln(
                    &format!(
                        "Profile '{}' not found in configuration, creating empty profile",
                        profile.name
                    ),
                    &LogLevel::Warning,
                );
                profile.clone()
            });
        self.save(&ctx.working_dir)?;
        Ok(())
    }

    pub fn init(cwd: &Path) -> Result<Self, anyhow::Error> {
        let config_path = cwd.join("config.toml");
        if config_path.exists() {
            cprintln("config.toml exists, skipping", &LogLevel::Warning);
            // Running `init` again is how an existing repository gets its ignore entries back.
            Self::ensure_gitignore(cwd)?;
            return Self::from_path(cwd);
        }
        let default_config = Config::new();
        // `save()` prepends the `#:schema` directive understood by taplo (the
        // LSP behind VS Code's "Even Better TOML", Neovim's nvim-lspconfig
        // taplo preset, and other taplo-based editor integrations) to
        // associate this file with a JSON Schema for validation and
        // autocomplete, without relying on ambiguous filename-based catalog
        // matching.
        default_config.save(cwd)?;
        std::fs::create_dir_all(cwd.join("dotfiles"))?;

        Self::ensure_gitignore(cwd)?;

        cprintln("Repository initialized", &LogLevel::Info);
        Ok(default_config)
    }

    /// Adds to the repository's `.gitignore` each of [`GITIGNORE_ENTRIES`] it does not list yet, keeping everything
    /// else in it (creating it if there is none), and says which were added.
    pub fn ensure_gitignore(cwd: &Path) -> anyhow::Result<Vec<&'static str>> {
        let path = cwd.join(".gitignore");
        let existing = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e.into()),
        };
        let listed: Vec<&str> = existing
            .lines()
            .map(|line| line.trim().trim_start_matches('/').trim_end_matches('/'))
            .collect();
        let missing: Vec<&'static str> = GITIGNORE_ENTRIES
            .into_iter()
            .filter(|entry| !listed.contains(entry))
            .collect();
        if missing.is_empty() {
            return Ok(missing);
        }
        let mut text = existing;
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        for entry in &missing {
            text.push_str(entry);
            text.push('\n');
        }
        std::fs::write(&path, text)?;
        cprintln(
            &format!("Added {} to .gitignore", missing.join(", ")),
            &LogLevel::Info,
        );
        Ok(missing)
    }

    pub fn new() -> Self {
        let mut profiles: HashMap<String, Profile> = HashMap::new();
        profiles.insert("default".to_string(), Profile::new("default"));
        Self {
            banner: true,
            profiles,
            ..Default::default()
        }
    }

    pub fn list_packages(&self, ctx: &Context, args: &PackagesListArgs) -> anyhow::Result<()> {
        let packages = self.filter_packages(&ctx.profile, &None, false)?;
        if args.plain {
            for pkg in packages.values() {
                outln!("{}", pkg.name);
            }
        } else if packages.is_empty() {
            cprintln("No packages found.", &LogLevel::Info);
        } else {
            for pkg in packages.values() {
                outln!("{}", pkg.name);
                if args.verbose {
                    out!(
                        "    Source: {}\n    Destination: {}\n    skipped: {}\n",
                        pkg.src,
                        pkg.dest,
                        pkg.skip
                    );
                    if let Some(deps) = &pkg.dependencies {
                        outln!("    Dependencies: {deps:?}");
                    }
                    if !pkg.targets.is_empty() {
                        outln!("    Targets:");
                        for (target_name, target_dest) in &pkg.targets {
                            outln!("      - {target_name}: {target_dest}");
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn list_profiles(&self, args: &crate::cli::ProfilesListArgs) -> anyhow::Result<()> {
        if args.plain {
            for profile in self.profiles.values() {
                outln!("{}", profile.name);
            }
        } else if self.profiles.is_empty() {
            cprintln("No profiles found.", &LogLevel::Info);
        } else {
            for profile in self.profiles.values() {
                outln!("{}", profile.name);
                if args.verbose {
                    outln!("    Dependencies: {:?}", profile.dependencies);
                    outln!("    Variables: {:?}", profile.variables);
                    if let Some(backend) = profile.prompt_backend {
                        outln!("    Prompt backend: {}", backend.as_str());
                    }
                    if let Some(platform) = &profile.platform {
                        outln!("    Platform: {platform}");
                    }
                    if !profile.prompts.is_empty() {
                        outln!("    Prompts:");
                        for (var, prompt) in &profile.prompts {
                            outln!("      - {var}: {prompt}");
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn add_profile(&mut self, args: &ProfilesAddArgs, ctx: &mut Context) -> anyhow::Result<()> {
        if self.profiles.contains_key(&args.name) {
            anyhow::bail!("Profile '{}' already exists", args.name);
        }
        let profile = Profile::new(&args.name);
        self.profiles.insert(args.name.clone(), profile.clone());
        self.save(&ctx.working_dir)?;
        cprintln(&format!("Profile '{}' added", args.name), &LogLevel::Info);
        if args.set_as_current {
            ctx.save_to_uservariables("DOTR_PROFILE", toml::Value::String(profile.name.clone()))?;
            cprintln(
                &format!("Setting profile '{}' as current", args.name),
                &LogLevel::Info,
            );
        }
        Ok(())
    }

    pub fn get_orphan_packages(&self) -> Vec<String> {
        self.packages
            .keys()
            .filter_map(
                |name| match self.is_package_safe_to_remove(name, &[], &[]) {
                    (true, _, _) => Some(name.clone()),
                    _ => None,
                },
            )
            .collect()
    }

    pub fn remove_packages(
        &mut self,
        args: &RemovePackageArgs,
        ctx: &Context,
    ) -> anyhow::Result<()> {
        let packages = match &args.packages {
            Some(pkgs) => pkgs.clone(),
            None => {
                if args.remove_orphans {
                    vec![]
                } else {
                    anyhow::bail!("No packages specified for removal");
                }
            }
        };
        let ignored_profiles: Vec<String> = vec![ctx.profile.name.clone()];
        let mut dirty = false;
        let mut to_remove = HashMap::new();
        for package_name in &packages {
            if !self.packages.contains_key(package_name) {
                anyhow::bail!("Package '{package_name}' not found in configuration");
            }
            let (is_safe, dependent_profiles, dependent_packages) =
                self.is_package_safe_to_remove(package_name, &ignored_profiles, &packages);
            if !is_safe && !args.force {
                anyhow::bail!(
                    "Package '{package_name}' cannot be removed because it is depended on by profiles: {dependent_profiles:?} and packages: {dependent_packages:?}. Use --force to override."
                );
            }
            to_remove.insert(
                package_name.clone(),
                self.packages
                    .get(package_name)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Package '{package_name}' not found in configuration")
                    })?
                    .clone(),
            );
        }
        if to_remove.is_empty() && !args.remove_orphans {
            cprintln("No packages to remove.", &LogLevel::Info);
            return Ok(());
        }
        for (package_name, pkg) in &to_remove {
            if args.dry_run {
                cprintln(
                    &format!("Package '{package_name}' would be removed (dry run)"),
                    &LogLevel::Info,
                );
                continue;
            }
            match self.remove_package(pkg, ctx) {
                Err(e) => {
                    anyhow::bail!("Error removing package '{package_name}': {e}");
                }
                Ok(()) => {
                    dirty = true;
                    cprintln(
                        &format!("Package '{package_name}' removed"),
                        &LogLevel::Info,
                    );
                }
            }
        }
        if args.remove_orphans {
            let orphan_packages = self.get_orphan_packages();
            for orphan in &orphan_packages {
                if args.dry_run {
                    cprintln(
                        &format!("Orphan package '{orphan}' would be removed (dry run)"),
                        &LogLevel::Info,
                    );
                    continue;
                }
                let pkg = self
                    .packages
                    .get(orphan)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Orphan package '{orphan}' not found in configuration")
                    })?
                    .clone();
                match self.remove_package(&pkg, ctx) {
                    Err(e) => {
                        anyhow::bail!("Error removing orphan package '{orphan}': {e}");
                    }
                    Ok(()) => {
                        dirty = true;
                        cprintln(
                            &format!("Orphan package '{orphan}' removed"),
                            &LogLevel::Info,
                        );
                    }
                }
            }
        }
        if dirty {
            self.save(&ctx.working_dir)?;
        }
        Ok(())
    }

    pub fn remove_package(&mut self, pkg: &Package, ctx: &Context) -> anyhow::Result<()> {
        let src = ctx.working_dir.join(&pkg.src);
        let name = pkg.name.clone();
        self.packages.remove(&pkg.name);
        for profile in self.profiles.values_mut() {
            profile.dependencies.retain(|dep| dep != &name);
        }
        for pkg in self.packages.values_mut() {
            if let Some(deps) = &mut pkg.dependencies {
                deps.retain(|dep| dep != &name);
            }
        }
        if src.exists() {
            if src.is_dir() {
                if src.read_dir()?.next().is_some() {
                    std::fs::remove_dir_all(&src)?;
                } else {
                    std::fs::remove_dir(&src)?;
                }
            } else {
                std::fs::remove_file(&src)?;
            }
        }
        Ok(())
    }

    pub fn is_package_safe_to_remove(
        &self,
        package_name: &str,
        ignored_profiles: &[String],
        ignored_packages: &[String],
    ) -> (bool, Vec<String>, Vec<String>) {
        let mut dependent_profiles: Vec<String> = vec![];
        let mut dependent_packages: Vec<String> = vec![];
        let mut is_safe = true;
        for profile in self.profiles.values() {
            if ignored_profiles.contains(&profile.name) {
                continue;
            }
            if profile.dependencies.contains(&package_name.to_string()) {
                dependent_profiles.push(profile.name.clone());
                is_safe = false;
            }
        }
        for pkg in self.packages.values() {
            if ignored_packages.contains(&pkg.name) {
                continue;
            }
            if let Some(deps) = &pkg.dependencies
                && deps.contains(&package_name.to_string())
            {
                dependent_packages.push(pkg.name.clone());
                is_safe = false;
            }
        }
        (is_safe, dependent_profiles, dependent_packages)
    }

    pub fn remove_profile(
        &mut self,
        args: &ProfileRemoveArgs,
        ctx: &Context,
    ) -> anyhow::Result<()> {
        if !self.profiles.contains_key(&args.name) {
            anyhow::bail!("Profile '{}' not found in configuration", args.name);
        }
        if args.name == "default" {
            anyhow::bail!("Cannot remove the default profile");
        }
        if args.dry_run {
            cprintln(
                &format!("Profile '{}' would be removed (dry run)", args.name),
                &LogLevel::Info,
            );
            return Ok(());
        }
        self.profiles.remove(&args.name);
        self.save(&ctx.working_dir)?;
        cprintln(&format!("Profile '{}' removed", args.name), &LogLevel::Info);
        if args.remove_orphans {
            let orphan_packages = self.get_orphan_packages();
            let mut dirty = false;
            for orphan in &orphan_packages {
                let pkg = self
                    .packages
                    .get(orphan)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Orphan package '{orphan}' not found in configuration")
                    })?
                    .clone();
                match self.remove_package(&pkg, ctx) {
                    Err(e) => {
                        anyhow::bail!("Error removing orphan package '{orphan}': {e}");
                    }
                    Ok(()) => {
                        dirty = true;
                        cprintln(
                            &format!("Orphan package '{orphan}' removed"),
                            &LogLevel::Info,
                        );
                    }
                }
            }
            if dirty {
                self.save(&ctx.working_dir)?;
            }
        }
        Ok(())
    }
}

pub(crate) fn print_stats(stats: &HashMap<BackupDeployResult, u32>, op_type: OpType) {
    let (op_name, op_success_name) = match op_type {
        OpType::Backup => ("Backup", "backed up"),
        OpType::Deploy => ("Deployment", "deployed"),
    };
    let mut summary_parts = vec![];
    if let Some(count) = stats.get(&BackupDeployResult::Success) {
        summary_parts.push(format!("✅ {count} {op_success_name}"));
    }
    if let Some(count) = stats.get(&BackupDeployResult::Skipped) {
        summary_parts.push(format!("🔄 {count} no changes"));
    }
    if let Some(count) = stats.get(&BackupDeployResult::LeftOut) {
        summary_parts.push(format!("⚠️ {count} left out (see the warnings above)"));
    }
    if let Some(count) = stats.get(&BackupDeployResult::Failed) {
        summary_parts.push(format!("❌ {count} failed"));
    }
    if summary_parts.is_empty() {
        cprintln(
            &format!("No packages processed for {op_name}"),
            &LogLevel::Info,
        );
    } else {
        let mut summary_string = summary_parts.join(", ");
        summary_string.push('.');
        cprintln(&format!("{op_name} summary:"), &LogLevel::Info);
        cprintln(&summary_string, &LogLevel::Info);
    }
}

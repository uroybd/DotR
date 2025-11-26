use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use toml::{Table, Value, map::Map};

use crate::{
    cli::{DeployArgs, DiffArgs, ImportArgs, PackagesListArgs, ProfilesAddArgs, UpdateArgs},
    context::Context,
    package::{BackupDeployResult, Package},
    profile::Profile,
    utils::{LogLevel, cprintln},
};

#[cfg(test)]
mod tests;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Config {
    pub banner: bool,
    pub packages: HashMap<String, Package>,
    pub profiles: HashMap<String, Profile>,
    pub variables: Table,
    pub prompts: HashMap<String, String>, // The key of variable, and the value is the prompt message
}

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
        let table = self.to_table();
        let config_content = toml::to_string_pretty(&table)?;
        std::fs::write(cwd.join("config.toml"), config_content)?;
        Ok(())
    }

    pub fn from_table(table: &Table) -> anyhow::Result<Self> {
        let mut packages: HashMap<String, Package> = HashMap::new();
        // Iter on packages value as key value
        let package_confs = table.get("packages").and_then(|v| v.as_table());
        if let Some(pkg_confs) = package_confs {
            for (key, val) in pkg_confs.iter() {
                let pkg_val = val
                    .as_table()
                    .ok_or_else(|| anyhow::anyhow!("Package '{}' must be a table", key))?;
                let pkg = Package::from_table(key, pkg_val)?;
                packages.insert(pkg.name.clone(), pkg);
            }
        }

        let mut profiles: HashMap<String, Profile> = HashMap::new();
        let profile_confs = table.get("profiles").and_then(|v| v.as_table());
        if let Some(prof_confs) = profile_confs {
            for (key, val) in prof_confs.iter() {
                let prof_val = val
                    .as_table()
                    .ok_or_else(|| anyhow::anyhow!("Profile '{}' must be a table", key))?;
                let profile = Profile::from_table(key, prof_val)?;
                profiles.insert(profile.name.clone(), profile);
            }
        }
        let mut variables: Table = Table::new();
        // Add HOME as a default variable
        if let Some(vars) = table.get("variables").and_then(|v| v.as_table()) {
            for (k, v) in vars.iter() {
                variables.insert(k.clone(), v.clone());
            }
        }
        let mut prompts: HashMap<String, String> = HashMap::new();
        if let Some(prompts_table) = table.get("prompts").and_then(|v| v.as_table()) {
            for (k, v) in prompts_table.iter() {
                if let Some(prompt_str) = v.as_str() {
                    prompts.insert(k.clone(), prompt_str.to_string());
                }
            }
        }
        Ok(Self {
            banner: table
                .get("banner")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            packages,
            profiles,
            variables,
            prompts,
        })
    }
    pub fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.insert("banner".to_string(), toml::Value::Boolean(self.banner));
        if !self.variables.is_empty() {
            table.insert(
                "variables".to_string(),
                Value::Table(self.variables.clone()),
            );
        }
        if !self.packages.is_empty() {
            let mut packages_table: Map<String, Value> = Map::new();
            self.packages.iter().for_each(|(name, pkg)| {
                packages_table.insert(name.clone(), Value::Table(pkg.to_table()));
            });
            table.insert("packages".to_string(), packages_table.into());
        }
        if !self.profiles.is_empty() {
            let mut profiles_table: Map<String, Value> = Map::new();
            self.profiles.iter().for_each(|(name, profile)| {
                profiles_table.insert(name.clone(), Value::Table(profile.to_table()));
            });
            table.insert("profiles".to_string(), profiles_table.into());
        }
        if !self.prompts.is_empty() {
            let mut prompts_table: Map<String, Value> = Map::new();
            self.prompts.iter().for_each(|(key, prompt)| {
                prompts_table.insert(key.clone(), Value::String(prompt.clone()));
            });
            table.insert("prompts".to_string(), prompts_table.into());
        }
        table
    }

    pub fn import_package(&mut self, args: &ImportArgs, ctx: &Context) -> anyhow::Result<()> {
        let mut profile = ctx.profile.clone();
        let profile_name = profile.name.clone();
        cprintln(&format!("Importing from {}", args.path), &LogLevel::INFO);
        let mut package = Package::from_path(args, &ctx.working_dir)?;
        let pkg_name = package.name.clone();

        // Create default UpdateArgs for import backup
        let backup_args = crate::cli::UpdateArgs {
            packages: None,
            profile: Some(profile_name.clone()),
            ignore_errors: false,
            clean: false,
            dry_run: false,
        };
        package.backup(ctx, &backup_args)?;
        profile.dependencies.push(pkg_name.clone());
        if profile_name != "default" {
            package
                .targets
                .insert(profile_name.clone(), package.dest.clone());
        }
        self.packages.insert(pkg_name.clone(), package);
        self.profiles.insert(profile_name.clone(), profile);
        self.save(&ctx.working_dir)?;
        cprintln(&format!("Package '{}' imported", pkg_name), &LogLevel::INFO);
        Ok(())
    }

    pub fn filter_packages(
        &self,
        ctx: &Context,
        names: &Option<Vec<String>>,
    ) -> anyhow::Result<HashMap<String, Package>> {
        let mut packages: HashMap<String, Package> = HashMap::new();
        if let Some(pkg_names) = names {
            for name in pkg_names {
                if let Some(pkg) = self.packages.get(name) {
                    packages.insert(name.clone(), pkg.clone());
                } else {
                    return Err(anyhow::anyhow!("Package '{}' not found", name));
                }
            }
        } else {
            for dep in &ctx.profile.dependencies {
                if let Some(pkg) = self.packages.get(dep) {
                    if !pkg.skip {
                        packages.insert(dep.clone(), pkg.clone());
                    }
                } else {
                    anyhow::bail!(
                        "Package '{}' not found for profile '{}'",
                        dep,
                        ctx.profile.name
                    );
                }
            }
        }
        // Now resolve packages dependencies
        let mut dependencies: HashMap<String, Package> = HashMap::new();
        for (_, pkg) in packages.iter() {
            if let Some(deps) = &pkg.dependencies {
                for dep in deps {
                    if let Some(dep_pkg) = self.packages.get(dep) {
                        dependencies.insert(dep.clone(), dep_pkg.clone());
                    } else {
                        anyhow::bail!("Dependency package '{}' not found in configuration", dep);
                    }
                }
            }
        }
        packages.extend(dependencies);
        Ok(packages)
    }

    pub fn backup_packages(&self, ctx: &Context, args: &UpdateArgs) -> Result<(), anyhow::Error> {
        cprintln("Backing up packages...", &LogLevel::INFO);
        let mut stats: HashMap<BackupDeployResult, u32> = HashMap::new();
        for (_, pkg) in self.filter_packages(ctx, &args.packages)?.iter() {
            match pkg.backup(ctx, args) {
                Err(e) => {
                    if args.ignore_errors {
                        cprintln(
                            &format!("Error backing up package '{}': {}", pkg.name, e),
                            &LogLevel::ERROR,
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

    pub fn deploy_packages(&self, ctx: &Context, args: &DeployArgs) -> Result<(), anyhow::Error> {
        cprintln("Deploying packages...", &LogLevel::INFO);
        let mut stats: HashMap<BackupDeployResult, u32> = HashMap::new();
        for (_, pkg) in self.filter_packages(ctx, &args.packages)?.iter() {
            match pkg.deploy(ctx, args) {
                Err(e) => {
                    if args.ignore_errors {
                        cprintln(
                            &format!("Error deploying package '{}': {}", pkg.name, e),
                            &LogLevel::ERROR,
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
        print_stats(&stats, OpType::Deploy);
        Ok(())
    }

    pub fn diff_packages(&self, ctx: &Context, args: &DiffArgs) -> Result<(), anyhow::Error> {
        cprintln("Checking differences...", &LogLevel::INFO);
        for (_, pkg) in self.filter_packages(ctx, &args.packages)?.iter() {
            cprintln(&format!("Package: {}", pkg.name), &LogLevel::INFO);
            if let Err(e) = pkg.diff(ctx) {
                if args.ignore_errors {
                    cprintln(
                        &format!("Error diffing package '{}': {}", pkg.name, e),
                        &LogLevel::ERROR,
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
                    &LogLevel::WARNING,
                );
                profile.clone()
            });
        self.save(&ctx.working_dir)?;
        Ok(())
    }

    pub fn init(cwd: &Path) -> Result<Self, anyhow::Error> {
        // If config.toml already exists, do nothing
        let config_path = cwd.join("config.toml");
        if config_path.exists() {
            cprintln("config.toml exists, skipping", &LogLevel::WARNING);
            return Self::from_path(cwd);
        }
        // Here you would add the logic to create a default config file
        let default_config = Config::new();
        let toml_string = toml::to_string(&default_config)?;
        std::fs::write(config_path, toml_string)?;
        std::fs::create_dir_all(cwd.join("dotfiles"))?;

        // Create .gitignore to ignore .uservariables.toml
        let gitignore_path = cwd.join(".gitignore");
        let gitignore_content = ".uservariables.toml\n";
        std::fs::write(gitignore_path, gitignore_content)?;

        cprintln("Repository initialized", &LogLevel::INFO);
        Ok(default_config)
    }

    pub fn new() -> Self {
        let mut profiles: HashMap<String, Profile> = HashMap::new();
        profiles.insert("default".to_string(), Profile::new("default"));
        Self {
            banner: !cfg!(test),
            packages: HashMap::new(),
            variables: Table::new(),
            profiles,
            prompts: HashMap::new(),
        }
    }

    pub fn list_packages(&self, ctx: &Context, args: &PackagesListArgs) -> anyhow::Result<()> {
        let packages = self.filter_packages(ctx, &None)?;
        if packages.is_empty() {
            cprintln("No packages found.", &LogLevel::INFO);
        } else {
            for (name, pkg) in packages.iter() {
                println!("{} ", name);
                if args.verbose {
                    print!(
                        "    Source: {}\n    Destination: {}\n    skipped: {}\n",
                        pkg.src, pkg.dest, pkg.skip
                    );
                    if let Some(deps) = &pkg.dependencies {
                        println!("    Dependencies: {:?}", deps);
                    }
                    if !pkg.targets.is_empty() {
                        println!("    Targets:");
                        for (target_name, target_dest) in pkg.targets.iter() {
                            println!("      - {}: {}", target_name, target_dest);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn list_profiles(&self, args: &crate::cli::ProfilesListArgs) -> anyhow::Result<()> {
        if self.profiles.is_empty() {
            cprintln("No profiles found.", &LogLevel::INFO);
        } else {
            for (name, profile) in self.profiles.iter() {
                println!("{} ", name,);
                if args.verbose {
                    println!("    Dependencies: {:?}", profile.dependencies);
                    println!("    Variables: {:?}", profile.variables);
                    if !profile.prompts.is_empty() {
                        println!("    Prompts: {:?}", profile.prompts);
                        for (var, prompt) in profile.prompts.iter() {
                            println!("      - {}: {}", var, prompt);
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
        cprintln(&format!("Profile '{}' added", args.name), &LogLevel::INFO);
        if args.set_as_current {
            ctx.save_to_uservariables(
                "DOTR_PROFILE",
                toml::Value::String(profile.name.clone()),
            )?;
            cprintln(
                &format!("Setting profile '{}' as current", args.name),
                &LogLevel::INFO,
            );
        }
        Ok(())
    }
}

pub(crate) fn print_stats(stats: &HashMap<BackupDeployResult, u32>, op_type: OpType) {
    // Print a one-liner summary of stats for each result type, with emojis
    let (op_name, op_success_name) = match op_type {
        OpType::Backup => ("Backup", "backed up"),
        OpType::Deploy => ("Deployment", "deployed"),
    };
    let mut summary_parts = vec![];
    if let Some(count) = stats.get(&BackupDeployResult::Success) {
        summary_parts.push(format!("✅ {} {}", count, op_success_name));
    }
    if let Some(count) = stats.get(&BackupDeployResult::Skipped) {
        summary_parts.push(format!("🔄 {} no changes", count));
    }
    if let Some(count) = stats.get(&BackupDeployResult::Failed) {
        summary_parts.push(format!("❌ {} failed", count));
    }
    if summary_parts.is_empty() {
        cprintln(
            &format!("No packages processed for {}", op_name),
            &LogLevel::INFO,
        );
    } else {
        let mut summary_string = summary_parts.join(", ");
        summary_string.push('.');
        cprintln(
            &format!("{} summary:", op_name).to_string(),
            &LogLevel::INFO,
        );
        cprintln(&summary_string, &LogLevel::INFO);
    }
}

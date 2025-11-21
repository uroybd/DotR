use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use toml::{Table, Value, map::Map};

use crate::{
    cli::{DeployArgs, UpdateArgs},
    context::Context,
    package::Package,
    profile::Profile,
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub banner: bool,
    pub packages: HashMap<String, Package>,
    pub profiles: HashMap<String, Profile>,
    pub variables: Table,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn from_path(cwd: &Path) -> Result<Self, anyhow::Error> {
        let config_path = cwd.join("config.toml");
        if !config_path.exists() {
            anyhow::bail!("config.toml not found in the current directory");
        }
        let config_content = std::fs::read_to_string(config_path)?;
        let conf_table = config_content.parse::<Table>()?;
        Self::from_table(&conf_table)
    }

    pub fn save(&self, cwd: &Path) -> Result<(), anyhow::Error> {
        let table = self.to_table();
        let config_content = table.to_string();
        std::fs::write(cwd.join("config.toml"), config_content)?;
        Ok(())
    }

    pub fn from_table(table: &Table) -> Result<Self, anyhow::Error> {
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
        Ok(Self {
            banner: table
                .get("banner")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            packages,
            profiles,
            variables,
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
        table
    }

    pub fn import_package(
        &mut self,
        path: &str,
        ctx: &Context,
        profile_name: &Option<String>,
    ) -> Result<(), anyhow::Error> {
        println!("Importing dotfiles from path: {}", path);
        let mut package = Package::from_path(path, &ctx.working_dir)?;
        let pkg_name = package.name.clone();
        package.backup(ctx)?;
        if let Some(p_name) = profile_name {
            let profile = self.profiles.entry(p_name.clone()).or_insert_with(|| {
                eprintln!(
                    "Warning: Profile '{}' not found in configuration. Creating new profile.",
                    p_name
                );
                Profile::new(p_name)
            });
            profile.dependencies.push(pkg_name.clone());
            package.skip = true;
            package.targets.insert(p_name.clone(), package.dest.clone());
        }
        self.packages.insert(pkg_name.clone(), package);
        println!("Config: {:?}", self);
        self.save(&ctx.working_dir)?;
        println!("Package '{}' imported successfully.", pkg_name);
        Ok(())
    }

    pub fn backup_packages(&self, ctx: &Context, args: &UpdateArgs) -> Result<(), anyhow::Error> {
        for (_, pkg) in self.filter_packages(ctx, &args.packages)?.iter() {
            pkg.backup(ctx)?;
        }
        Ok(())
    }

    fn filter_packages(
        &self,
        ctx: &Context,
        names: &Option<Vec<String>>,
    ) -> Result<HashMap<String, Package>, anyhow::Error> {
        let mut packages: HashMap<String, Package> = HashMap::new();
        if let Some(pkg_names) = names {
            for name in pkg_names {
                if let Some(pkg) = self.packages.get(name) {
                    packages.insert(name.clone(), pkg.clone());
                } else {
                    eprintln!("Warning: Package '{}' not found in configuration.", name);
                }
            }
        } else if let Some(profile) = &ctx.profile {
            for dep in &profile.dependencies {
                if let Some(pkg) = self.packages.get(dep) {
                    packages.insert(dep.clone(), pkg.clone());
                } else {
                    eprintln!("Warning: Package '{}' not found in configuration.", dep);
                }
            }
        } else {
            // Insert to packages if skip is false
            for (name, pkg) in self.packages.iter() {
                if !pkg.skip {
                    packages.insert(name.clone(), pkg.clone());
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

    pub fn deploy_packages(&self, ctx: &Context, args: &DeployArgs) -> Result<(), anyhow::Error> {
        println!("Copying dotfiles...");
        for (_, pkg) in self.filter_packages(ctx, &args.packages)?.iter() {
            pkg.deploy(ctx)?;
        }
        Ok(())
    }

    pub fn get_profile_details(&self, pname: &Option<String>) -> (Option<String>, Option<Profile>) {
        let profile = if let Some(p_name) = pname {
            self.profiles.get(p_name).cloned()
        } else {
            None
        };
        (pname.clone(), profile)
    }

    pub fn init(cwd: &Path) -> Result<Self, anyhow::Error> {
        // If config.toml already exists, do nothing
        let config_path = cwd.join("config.toml");
        if config_path.exists() {
            println!("config.toml already exists. Initialization skipped.");
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

        println!("Default config.toml created.");
        Ok(default_config)
    }

    pub fn new() -> Self {
        Self {
            banner: true,
            packages: HashMap::new(),
            variables: Table::new(),
            profiles: HashMap::new(),
        }
    }
}

use std::{
    collections::HashMap,
    fs,
    io::{self, Error},
    path::Path,
};

use serde::{Deserialize, Serialize};
use toml::{Table, Value, map::Map};

use crate::{
    cli::{Context, DeployArgs, UpdateArgs},
    package::Package,
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub banner: bool,
    pub packages: HashMap<String, Package>,
    pub variables: Table,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn from_path(cwd: &Path) -> Self {
        let config_path = cwd.join("config.toml");
        if !config_path.exists() {
            eprintln!("Error: config.toml not found in the current directory.");
            std::process::exit(1);
        }
        let config_content =
            std::fs::read_to_string(config_path).expect("Failed to read config.toml");
        let conf_table = config_content
            .parse::<Table>()
            .expect("Failed to parse config.");
        let mut conf = Config::from_table(&conf_table);
        let user_vars = Config::get_uservariables(cwd);
        for (k, v) in user_vars.iter() {
            conf.variables.insert(k.clone(), v.clone());
        }
        conf
    }
    pub fn save(&self, cwd: &Path) {
        // Re-read only the config variables (not user variables) to save
        let mut table = Table::new();
        table.insert("banner".to_string(), toml::Value::Boolean(self.banner));

        // Get user variables to exclude them
        let user_vars = Config::get_uservariables(cwd);

        // Save variables that are NOT from uservariables.toml
        let mut config_vars = Table::new();
        for (k, v) in self.variables.iter() {
            // Only save if it's not a user variable
            if !user_vars.contains_key(k) {
                config_vars.insert(k.clone(), v.clone());
            }
        }

        if !config_vars.is_empty() {
            table.insert("variables".to_string(), Value::Table(config_vars));
        }

        // Save packages
        if !self.packages.is_empty() {
            let mut packages_table: Map<String, Value> = Map::new();
            self.packages.iter().for_each(|(name, pkg)| {
                packages_table.insert(name.clone(), Value::Table(pkg.to_table()));
            });
            table.insert("packages".to_string(), packages_table.into());
        }

        let config_content = table.to_string();
        std::fs::write(cwd.join("config.toml"), config_content)
            .expect("Failed to write config.toml");
    }
    pub fn from_table(table: &Table) -> Self {
        let mut packages: HashMap<String, Package> = HashMap::new();
        // Iter on packages value as key value
        let package_confs = table.get("packages").and_then(|v| v.as_table()); // parse p as table
        if let Some(pkg_confs) = package_confs {
            packages = pkg_confs
                .iter()
                .map(|(key, val)| {
                    let pkg_val = val.as_table().expect("Failed to parse package");
                    let pkg = Package::from_table(key, pkg_val);
                    (pkg.name.clone(), pkg)
                })
                .collect();
        }
        let mut variables: Table = Table::new();
        // Add HOME as a default variable
        if let Some(vars) = table.get("variables").and_then(|v| v.as_table()) {
            for (k, v) in vars.iter() {
                variables.insert(k.clone(), v.clone());
            }
        }
        Self {
            banner: table
                .get("banner")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            packages,
            variables,
        }
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
        table
    }

    pub fn import_package(&mut self, path: &str, ctx: &Context) {
        println!("Importing dotfiles from path: {}", path);
        let package = Package::from_path(path, &ctx.working_dir);
        let pkg_name = package.name.clone();
        package.backup(ctx).expect("Error backing up while import");
        self.packages.insert(pkg_name.clone(), package);
        println!("Config: {:?}", self);
        self.save(&ctx.working_dir);
        println!("Package '{}' imported successfully.", pkg_name);
    }

    pub fn backup_packages(&self, ctx: &Context, args: &UpdateArgs) {
        for (_, pkg) in self.filter_packages(&args.packages).iter() {
            pkg.backup(ctx).expect("Error backing up");
        }
    }

    fn filter_packages(&self, names: &Option<Vec<String>>) -> HashMap<String, Package> {
        match names {
            Some(pkg_names) => pkg_names.iter().fold(HashMap::new(), |mut acc, name| {
                if let Some(pkg) = self.packages.get(name) {
                    acc.insert(name.clone(), pkg.clone());
                    if let Some(deps) = &pkg.dependencies {
                        for dep in deps.iter() {
                            if let Some(dep_pkg) = self.packages.get(dep)
                                && !acc.contains_key(dep)
                            {
                                acc.insert(dep.clone(), dep_pkg.clone());
                            }
                        }
                    }
                } else {
                    eprintln!("Warning: Package '{}' not found in configuration.", name);
                }
                acc
            }),
            None => self.packages.clone(),
        }
    }

    pub fn deploy_packages(&self, ctx: &Context, args: &DeployArgs) {
        println!("Copying dotfiles...");
        for (_, pkg) in self.filter_packages(&args.packages).iter() {
            pkg.deploy(ctx)
        }
    }

    pub fn get_uservariables(cwd: &Path) -> Table {
        let path = cwd.join(".uservariables.toml");
        if path.exists() {
            let content = fs::read_to_string(path).expect("Failed to read .uservariables.toml");
            toml::de::from_str(&content).expect("Failed to parse .uservariables.toml")
        } else {
            Table::new()
        }
    }
    pub fn init(cwd: &Path) -> Result<Self, Error> {
        // If config.toml already exists, do nothing
        let config_path = cwd.join("config.toml");
        if config_path.exists() {
            println!("config.toml already exists. Initialization skipped.");
            return Ok(Self::from_path(cwd));
        }
        // Here you would add the logic to create a default config file
        let default_config = Config::new();
        let toml_string =
            toml::to_string(&default_config).expect("Failed to serialize default config");
        std::fs::write(config_path, toml_string).expect("Failed to write default config.toml");
        std::fs::create_dir_all(cwd.join("dotfiles")).expect("Failed to create dotfiles directory");

        // Create .gitignore to ignore .uservariables.toml
        let gitignore_path = cwd.join(".gitignore");
        let gitignore_content = ".uservariables.toml\n";
        std::fs::write(gitignore_path, gitignore_content).expect("Failed to write .gitignore");

        println!("Default config.toml created.");
        Ok(default_config)
    }

    pub fn new() -> Self {
        Self {
            banner: true,
            packages: HashMap::new(),
            variables: Table::new(),
        }
    }
}

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use toml::Table;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub banner: bool,
    pub packages: HashMap<String, Package>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Package {
    pub src: String,
    pub dest: String,
    pub dependencies: Vec<String>,
}

pub fn load_config(wd: &PathBuf) -> Config {
    let config_path = wd.join("config.toml");
    if !config_path.exists() {
        eprintln!("Error: config.toml not found in the current directory.");
        std::process::exit(1);
    }
    let config_content = std::fs::read_to_string(config_path).expect("Failed to read config.toml");
    let conf_table = config_content
        .parse::<Table>()
        .expect("Failed to parse config.");
    let config = Config::from_table(&conf_table);
    return config;
}
impl Config {
    pub fn save(&self, cwd: &PathBuf) {
        let config_content = self.to_table().to_string();

        std::fs::write(cwd.join("config.toml"), config_content)
            .expect("Failed to write config.toml");
    }
    pub fn from_table(table: &Table) -> Self {
        let mut packages: HashMap<String, Package> = HashMap::new();
        // Iter on packages value as key value
        let package_confs = table.get("packages").and_then(|v| v.as_table()); // parse p as table
        if let Some(pkg_confs) = package_confs {
            pkg_confs.iter().for_each(|(pkg_name, val)| {
                let pkg_val = val.as_table().expect("Failed to parse package");
                let dependencies: Vec<String> = pkg_val
                    .get("dependencies")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.into()))
                            .collect()
                    })
                    .unwrap_or_default();
                let pkg = Package {
                    src: pkg_val
                        .get("src")
                        .expect("Package src is required")
                        .as_str()
                        .unwrap()
                        .to_string(),
                    dest: pkg_val
                        .get("dest")
                        .expect("Package dest is required")
                        .as_str()
                        .unwrap()
                        .to_string(),
                    dependencies: dependencies,
                };
                packages.insert(pkg_name.clone(), pkg);
            });
        }
        Self {
            banner: table
                .get("banner")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            packages: packages,
        }
    }
    pub fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.insert("banner".to_string(), toml::Value::Boolean(self.banner));
        let mut packages_table = Table::new();
        for (pkg_name, pkg) in &self.packages {
            let mut pkg_table = Table::new();
            pkg_table.insert("src".to_string(), toml::Value::String(pkg.src.clone()));
            pkg_table.insert("dest".to_string(), toml::Value::String(pkg.dest.clone()));
            let deps: Vec<toml::Value> = pkg
                .dependencies
                .iter()
                .map(|d| toml::Value::String(d.clone()))
                .collect();
            pkg_table.insert("dependencies".to_string(), toml::Value::Array(deps));
            packages_table.insert(pkg_name.clone(), toml::Value::Table(pkg_table));
        }
        table.insert("packages".to_string(), toml::Value::Table(packages_table));
        table
    }
}

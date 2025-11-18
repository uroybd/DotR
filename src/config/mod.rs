use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use toml::Table;

use crate::utils::resolve_path;

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

impl Package {
    pub fn from_path(path: &str, cwd: &PathBuf) -> Self {
        let resolved_path = resolve_path(path, cwd);
        if !resolved_path.clone().exists() {
            eprintln!("Error: Path '{}' does not exist.", resolved_path.display());
            std::process::exit(1);
        }
        let package_name = get_package_name(path, cwd);
        let dest_path_str = format!("dotfiles/{}", package_name);
        let mut path = path;
        if !path.starts_with('~') {
            path = resolved_path.to_str().unwrap();
        }
        Self {
            src: path.to_string(),
            dest: dest_path_str.clone(),
            dependencies: vec![],
        }
    }

    pub fn package_name(&self, cwd: &PathBuf) -> String {
        get_package_name(&self.src, cwd)
    }

    pub fn backup(&self, cwd: &PathBuf) {
        let dest_path = cwd.join(self.dest.clone());
        let src_path = resolve_path(&self.src, cwd);
        if !dest_path.exists() {
            std::fs::create_dir_all(dest_path.clone())
                .expect("Failed to create destination directory");
        }
        let backup_ext = std::ffi::OsStr::new("dotrbak");
        for entry in walkdir::WalkDir::new(src_path.clone()) {
            let entry = entry.expect("Failed to read directory entry");
            let relative_path = entry
                .path()
                .strip_prefix(src_path.clone())
                .expect("Failed to get relative path");
            let dest_file_path = dest_path.join(relative_path);
            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&dest_file_path).expect("Failed to create directory");
            } else {
                // Copy if the extension is not dotrbak
                if entry.path().extension() != Some(backup_ext) {
                    std::fs::copy(entry.path(), &dest_file_path).expect("Failed to copy file");
                }
            }
        }
    }
}

pub fn get_package_name(pathstr: &str, cwd: &PathBuf) -> String {
    let path = resolve_path(pathstr, cwd);
    let last_component = path
        .components()
        .last()
        .expect("Path has no components")
        .as_os_str()
        .to_str()
        .expect("Failed to convert OsStr to str");
    let mut package_name = last_component.trim_start_matches('.').to_string();

    // Remove any trailing version numbers
    if let Some(pos) = package_name.rfind('-') {
        package_name.truncate(pos);
    }
    return package_name;
}

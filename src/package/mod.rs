use std::{ffi::OsStr, path::Path};

use serde::{Deserialize, Serialize};
use toml::Table;

use crate::{
    cli::Context,
    utils::{BACKUP_EXT, resolve_path},
};

// A package represents a dotfile package with its source, destination, and dependencies.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Package {
    pub name: String,
    pub src: String,
    pub dest: String,
    pub dependencies: Option<Vec<String>>,
}

impl Package {
    // Create a new Package from a given path, used to import dotfiles.
    // The path can be absolute or relative to the current working directory.
    // That path must exist and it will be set to the dest field.
    pub fn from_path(path: &str, cwd: &Path) -> Self {
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
            name: package_name.clone(),
            dest: path.to_string(),
            src: dest_path_str.clone(),
            dependencies: None,
        }
    }

    pub fn from_table(pkg_name: &str, pkg_val: &Table) -> Self {
        let dependencies: Option<Vec<String>> = match pkg_val.get("dependencies") {
            Some(deps) => {
                let d = deps
                    .as_array()
                    .expect("Dependencies should be an array")
                    .iter()
                    .map(|d| d.as_str().unwrap().to_string())
                    .collect();
                Some(d)
            }
            None => None,
        };
        Self {
            name: pkg_name.to_string(),
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
            dependencies,
        }
    }

    pub fn to_table(&self) -> Table {
        let mut pkg_table = Table::new();
        pkg_table.insert("src".to_string(), toml::Value::String(self.src.clone()));
        pkg_table.insert("dest".to_string(), toml::Value::String(self.dest.clone()));
        if let Some(deps) = &self.dependencies {
            let deps_val: Vec<toml::Value> = deps
                .iter()
                .map(|d| toml::Value::String(d.clone()))
                .collect();
            pkg_table.insert("dependencies".to_string(), toml::Value::Array(deps_val));
        }
        pkg_table
    }

    /// Backup the package by copying files from dest to a backup location, recursively.
    pub fn backup(&self, ctx: &Context) -> anyhow::Result<()> {
        let copy_from = resolve_path(&self.dest, &ctx.working_dir);
        let copy_to = ctx.working_dir.join(self.src.clone());
        if copy_from.is_dir() {
            // Recursively copy directory contents, avoiding files ending with BACKUP_EXT
            for entry in walkdir::WalkDir::new(&copy_from) {
                let entry = entry?;
                let relative_path = entry.path().strip_prefix(&copy_from)?;
                let dest_path = copy_to.clone().join(relative_path);
                if entry.path().is_dir() {
                    std::fs::create_dir_all(&dest_path)?;
                } else if entry.path().extension() != Some(OsStr::new(BACKUP_EXT)) {
                    std::fs::create_dir_all(dest_path.parent().unwrap())?;
                    std::fs::copy(entry.path(), &dest_path)?;
                }
            }
        } else {
            std::fs::copy(&copy_from, &copy_to)?;
        }
        println!(
            "[INFO] Backed up file '{}' to '{}'",
            copy_from.display(),
            copy_to.display()
        );
        Ok(())
    }

    /// Deploy the package by copying files from src to dest.
    pub fn deploy(&self, ctx: &Context) {
        let copy_from = resolve_path(&self.src, &ctx.working_dir);
        let copy_to = resolve_path(&self.dest, &ctx.working_dir);
        let backup_path = copy_to.with_extension(BACKUP_EXT);
        // First, create a backup of the existing file/directory at dest
        if copy_to.exists() {
            if copy_to.is_dir() {
                std::fs::remove_dir_all(&backup_path).ok(); // Remove existing backup if any
                std::fs::rename(&copy_to, &backup_path).expect("Failed to create backup directory");
            } else {
                std::fs::rename(&copy_to, &backup_path).expect("Failed to create backup file");
            }
        }
        if copy_from.is_dir() {
            // Recursively copy directory contents
            for entry in walkdir::WalkDir::new(&copy_from) {
                let entry = entry.expect("Failed to read directory entry");
                let relative_path = entry.path().strip_prefix(&copy_from).unwrap();
                let dest_path = copy_to.join(relative_path);
                if entry.path().is_dir() {
                    std::fs::create_dir_all(&dest_path).expect("Failed to create directory");
                } else {
                    std::fs::create_dir_all(dest_path.parent().unwrap())
                        .expect("Failed to create parent directory");
                    std::fs::copy(entry.path(), &dest_path).expect("Failed to copy file");
                }
            }
        } else {
            std::fs::copy(&copy_from, &copy_to).expect("Failed to copy file");
        }
        println!(
            "[INFO] Deployed file '{}' to '{}'",
            copy_from.display(),
            copy_to.display()
        );
    }

    pub fn is_dir(&self) -> bool {
        self.name.starts_with("d_")
    }

    pub fn is_templated(&self, cwd: &Path) -> bool {
        // Check if src exists as a directory or file, if not return true
    }
}

/// Get a package name from a given path string.
/// The package name is derived from the last component of the path,
/// with any leading '.' removed, and any trailing version numbers removed.
/// Additionally, any '-' or '.' characters are replaced with '_'.
/// If the path is a directory, it should be prepended with d_
/// Or, if it's a file, with f_
pub fn get_package_name(pathstr: &str, cwd: &Path) -> String {
    let path = resolve_path(pathstr, cwd);
    let last_component = path
        .file_name()
        .expect("Failed to get file name")
        .to_str()
        .unwrap();
    let mut package_name = last_component.trim_start_matches('.').to_string();

    // Remove any trailing version numbers
    if let Some(pos) = package_name.rfind('-') {
        package_name.truncate(pos);
    }
    // replace any remaining '-' with '_', and '.' with '_'
    let prefix = if path.is_dir() { "d_" } else { "f_" };
    package_name = format!("{}{}", prefix, package_name);
    package_name.replace(['-', '.'], "_")
}

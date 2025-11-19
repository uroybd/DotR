use std::{ffi::OsStr, io::Error, path::Path};

use serde::{Deserialize, Serialize};
use toml::Table;

use crate::{utils::resolve_path, utils::BACKUP_EXT};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Package {
    pub name: String,
    pub src: String,
    pub dest: String,
    pub dependencies: Vec<String>,
}

impl Package {
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
            dependencies: vec![],
        }
    }

    pub fn deploy(&self, cwd: &Path) {
        let src_path = cwd.join(self.src.as_str());
        let dest_path = resolve_path(self.dest.as_str(), cwd);
        let last_dest_segment = dest_path
            .file_name()
            .expect("Failed to get last segment of dest path");
        // println!("SRC: {}, DST: {}", src_path.display(), dest_path.display());
        if dest_path.exists() {
            if dest_path.is_dir() {
                let backup_path = dest_path.with_extension(BACKUP_EXT);
                // Delete previous backup
                if backup_path.exists() {
                    std::fs::remove_dir_all(backup_path.clone())
                        .expect("Error removing previous backup");
                }
                std::fs::rename(dest_path.clone(), backup_path).expect("Failed to backup");

                // Create dest_path again
                std::fs::create_dir_all(dest_path.clone())
                    .expect("Failed to create dest directory");
                let src_path = src_path.join(last_dest_segment);
                for entry in walkdir::WalkDir::new(src_path.clone()) {
                    let entry = entry.expect("Failed to read directory entry");
                    let relative_path = entry
                        .path()
                        .strip_prefix(src_path.clone())
                        .expect("Failed to get relative path");
                    let dest_file_path = dest_path.join(relative_path);
                    // println!(
                    //     "Deploying from {} to {}",
                    //     entry.clone().path().display(),
                    //     dest_file_path.display()
                    // );
                    if entry.file_type().is_dir() {
                        std::fs::create_dir_all(&dest_file_path)
                            .expect("Failed to create directory");
                    } else {
                        std::fs::copy(entry.path(), &dest_file_path).expect("Failed to copy file");
                    }
                }
            } else {
                // create backup extension. e.g. init.lua -> init.lua.dotrbak
                let ext = match dest_path.extension() {
                    Some(e) => format!("{}.{}", e.to_str().unwrap(), BACKUP_EXT),
                    None => BACKUP_EXT.to_string(),
                };
                // println!(
                //     "Backing up existing file to {:?}",
                //     dest_path.with_extension(&ext)
                // );
                let backup_path = dest_path.with_extension(ext);
                std::fs::rename(&dest_path, &backup_path).expect("Failed to backup existing file");
                let src_file_path = src_path.join(last_dest_segment);
                std::fs::copy(&src_file_path, &dest_path).expect("Failed to copy file");
            }
        }
    }

    pub fn from_table(pkg_name: &str, pkg_val: &Table) -> Self {
        let dependencies: Vec<String> = pkg_val
            .get("dependencies")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.into()))
                    .collect()
            })
            .unwrap_or_default();
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
        if !self.dependencies.is_empty() {
            let deps: Vec<toml::Value> = self
                .dependencies
                .iter()
                .map(|d| toml::Value::String(d.clone()))
                .collect();
            pkg_table.insert("dependencies".to_string(), toml::Value::Array(deps));
        }
        pkg_table
    }

    pub fn backup(&self, cwd: &Path) -> Result<(), Error> {
        let src_path = cwd.join(self.src.clone());
        let dest_path = resolve_path(&self.dest, cwd);
        // If the dest path is a file just copy it in:
        if !src_path.exists() {
            std::fs::create_dir_all(src_path.clone())?;
        }
        let dest_path_last_segment = match dest_path.file_name() {
            Some(name) => name,
            None => {
                return Err(Error::other("Failed to get last segment of dest path"));
            }
        };
        if dest_path.is_file() {
            let src_path = src_path.join(dest_path_last_segment);
            std::fs::copy(dest_path, src_path)?;
        } else if dest_path.is_dir() {
            // Get last section of dest_path and create src path by adding it:
            let src_path = src_path.join(dest_path_last_segment);

            if !src_path.exists() {
                std::fs::create_dir_all(src_path.clone())?;
            }
            for entry in walkdir::WalkDir::new(dest_path.clone()) {
                let entry = entry?;
                let relative_path = entry
                    .path()
                    .strip_prefix(dest_path.clone())
                    .expect("Failed to get relative path");
                let src_file_path = src_path.join(relative_path);
                println!("Backing up to {}", src_file_path.display());
                if entry.file_type().is_dir() {
                    std::fs::create_dir_all(&src_file_path).expect("Failed to create directory");
                } else {
                    // Copy if the extension is not dotrbak
                    if entry.path().extension() != Some(OsStr::new(BACKUP_EXT)) {
                        std::fs::copy(entry.path(), &src_file_path).expect("Failed to copy file");
                    }
                }
            }
        }
        Ok(())
    }
}

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
    package_name.replace(['-', '.'], "_")
}

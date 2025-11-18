use std::path::PathBuf;

use crate::config::{Config, Package};

pub fn import_dots(path: &str, conf: &mut Config, cwd: &PathBuf) {
    println!("Importing dotfiles from path: {}", path);
    // Recursively copy the files from src to dest
    let package = Package::from_path(path, cwd);
    let package_name = package.package_name(cwd);
    // Add the new package to the config
    package.backup(cwd);
    conf.packages.insert(package_name.clone(), package);
    // Save the updated config
    conf.save(cwd);
    println!("Package '{}' imported successfully.", package_name);
}

pub fn backup_dots(conf: &Config, cwd: &PathBuf) {
    for (_, pkg) in conf.packages.iter() {
        pkg.backup(cwd);
    }
}

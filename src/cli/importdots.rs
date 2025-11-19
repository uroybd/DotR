use std::path::PathBuf;

use crate::config::{Config, Package};

pub fn import_dots(path: &str, conf: &mut Config, cwd: &PathBuf) {
    println!("Importing dotfiles from path: {}", path);
    let package = Package::from_path(path, cwd);
    let pkg_name = package.name.clone();
    package.backup(cwd);
    conf.packages.insert(pkg_name.clone(), package);
    conf.save(cwd);
    println!("Package '{}' imported successfully.", pkg_name);
}

pub fn backup_dots(conf: &Config, cwd: &PathBuf) {
    for (_, pkg) in conf.packages.iter() {
        pkg.backup(cwd);
    }
}

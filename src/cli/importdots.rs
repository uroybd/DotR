use std::path::Path;

use crate::config::{Config, Package};

pub fn import_dots(path: &str, conf: &mut Config, cwd: &Path) {
    println!("Importing dotfiles from path: {}", path);
    let package = Package::from_path(path, cwd);
    let pkg_name = package.name.clone();
    package.backup(cwd).expect("Error backing up while import");
    conf.packages.insert(pkg_name.clone(), package.clone());
    println!("Config: {:?}", conf);
    conf.save(cwd);
    println!("Package '{}' imported successfully.", pkg_name);
}

pub fn backup_dots(conf: &Config, cwd: &Path) {
    for (_, pkg) in conf.packages.iter() {
        pkg.backup(cwd).expect("Error backing up");
    }
}

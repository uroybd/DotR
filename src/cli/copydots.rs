use std::path::PathBuf;

use crate::config::Config;

pub fn copy_dots(conf: &Config, cwd: &PathBuf) {
    println!("Copying dotfiles...");
    for (_, pkg) in conf.packages.iter() {
        pkg.deploy(cwd)
    }
}

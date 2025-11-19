use std::path::{Path, PathBuf};

pub fn teardown(cwd: &Path) {
    // If NO_CLEANUP is set, skip cleanup
    if std::env::var("NO_CLEANUP").is_ok() {
        return;
    }
    // Clean up created config file after tests
    let config_path = cwd.join("config.toml");
    if config_path.clone().exists() {
        if let Err(e) = std::fs::remove_file(config_path.clone()) {
            eprintln!(
                "Error removing config.toml in {}: {}",
                config_path.clone().display(),
                e
            );
        }
    }
    // Delete the dotfiles directory if it exists
    let dotfiles_dir = cwd.join("dotfiles");
    if dotfiles_dir.clone().exists() {
        if let Err(e) = std::fs::remove_dir_all(dotfiles_dir.clone()) {
            eprintln!(
                "Error removing dotfiles directory in {}: {}",
                dotfiles_dir.clone().display(),
                e
            );
        }
    }
}

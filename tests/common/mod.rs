use std::path::Path;

pub fn teardown(cwd: &Path) {
    // If NO_CLEANUP is set, skip cleanup
    if std::env::var("NO_CLEANUP").is_ok() {
        return;
    }
    // Clean up created config file after tests
    let config_path = cwd.join("config.toml");
    if config_path.exists() {
        let _ = std::fs::remove_file(&config_path);
    }
    // Clean up .gitignore file
    let gitignore_path = cwd.join(".gitignore");
    if gitignore_path.exists() {
        let _ = std::fs::remove_file(&gitignore_path);
    }
    // Delete the dotfiles directory if it exists
    let dotfiles_dir = cwd.join("dotfiles");
    if dotfiles_dir.exists() {
        let _ = std::fs::remove_dir_all(&dotfiles_dir);
    }
    // Clean up any backup directories created during tests
    cleanup_backups(cwd);
}

fn cleanup_backups(cwd: &Path) {
    // Clean up common backup patterns in src directory
    let src_dir = cwd.join("src");
    if !src_dir.exists() {
        return;
    }

    let backup_patterns = vec![".dotrbak", ".bak", ".dotrback", ".testbak"];

    if let Ok(entries) = std::fs::read_dir(&src_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Check if it matches any backup pattern
            for pattern in &backup_patterns {
                if name_str.contains(pattern) {
                    let _ = if path.is_dir() {
                        std::fs::remove_dir_all(&path)
                    } else {
                        std::fs::remove_file(&path)
                    };
                    break;
                }
            }
        }
    }
}

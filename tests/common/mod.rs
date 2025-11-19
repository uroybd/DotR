use std::path::Path;

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

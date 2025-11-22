use std::{fs, path::Path};

#[allow(dead_code)]
pub fn setup(cwd: &Path) {
    // Ensure src directory exists
    let src_dir = cwd.join("src");
    let _ = fs::create_dir_all(&src_dir);

    // Restore all test files
    restore_test_files(cwd);
}

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
    // Restore original test files
    restore_test_files(cwd);
}

fn cleanup_backups(cwd: &Path) {
    // Clean up common backup patterns in src directory
    let src_dir = cwd.join("src");
    if !src_dir.exists() {
        return;
    }

    let backup_patterns = vec![".dotrbak", ".bak", ".dotrback", ".testbak"];

    // Use walkdir to recursively find and remove backup files
    for entry in walkdir::WalkDir::new(&src_dir).into_iter().flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Check if it matches any backup pattern
        for pattern in &backup_patterns {
            if name_str.contains(pattern) {
                let _ = if path.is_dir() {
                    std::fs::remove_dir_all(path)
                } else {
                    std::fs::remove_file(path)
                };
                break;
            }
        }
    }
}

fn restore_test_files(cwd: &Path) {
    let src_dir = cwd.join("src");

    // Restore .bashrc
    let _ = fs::write(
        src_dir.join(".bashrc"),
        "# Bashrc configuration\nexport PATH=\"$HOME/.local/bin:$PATH\"\nalias ls='ls --color=auto'\n",
    );

    // Restore .zshrc
    let _ = fs::write(
        src_dir.join(".zshrc"),
        "# ZSH Configuration\nexport PATH=\"$HOME/bin:$PATH\"\nalias ll='ls -la'\n",
    );

    // Restore .vimrc
    let _ = fs::write(
        src_dir.join(".vimrc"),
        "\" Vim Configuration\nset number\nset expandtab\nset tabstop=4\n",
    );

    // Restore .gitconfig
    let _ = fs::write(
        src_dir.join(".gitconfig"),
        "# Git Configuration\n[user]\n    name = Test User\n    email = test@example.com\n[core]\n    editor = vim\n",
    );

    // Restore nvim/init.lua
    let _ = fs::create_dir_all(src_dir.join("nvim"));
    let _ = fs::write(
        src_dir.join("nvim/init.lua"),
        "-- Neovim configuration\nvim.opt.number = true\nvim.opt.expandtab = true\n",
    );

    // Restore tmux files
    let _ = fs::create_dir_all(src_dir.join("tmux"));
    let _ = fs::write(
        src_dir.join("tmux/tmux.conf"),
        "# Tmux Configuration\nset -g mouse on\nbind-key r source-file ~/.tmux.conf\n",
    );
    let _ = fs::write(
        src_dir.join("tmux/theme.conf"),
        "# Tmux Theme\nset -g status-bg blue\nset -g status-fg white\n",
    );

    // Restore alacritty config
    let _ = fs::create_dir_all(src_dir.join("config/alacritty"));
    let _ = fs::write(
        src_dir.join("config/alacritty/alacritty.yml"),
        "# Alacritty Configuration\nwindow:\n  padding:\n    x: 10\n    y: 10\nfont:\n  size: 12.0\n",
    );
}

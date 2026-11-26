use std::{fs, path::Path};

#[allow(dead_code, reason = "shared by every test file; not each one calls it")]
pub fn setup(cwd: &Path) {
    // Ensure src directory exists
    let src_dir = cwd.join("src");
    fs::create_dir_all(&src_dir).ok();

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
        std::fs::remove_file(&config_path).ok();
    }
    // Clean up .gitignore file
    let gitignore_path = cwd.join(".gitignore");
    if gitignore_path.exists() {
        std::fs::remove_file(&gitignore_path).ok();
    }
    // Delete the dotfiles directory if it exists
    let dotfiles_dir = cwd.join("dotfiles");
    if dotfiles_dir.exists() {
        std::fs::remove_dir_all(&dotfiles_dir).ok();
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
                // Best effort: a backup another test already removed is fine.
                if path.is_dir() {
                    std::fs::remove_dir_all(path).ok();
                } else {
                    std::fs::remove_file(path).ok();
                }
                break;
            }
        }
    }
}

fn restore_test_files(cwd: &Path) {
    let src_dir = cwd.join("src");

    // Helper function to write file, removing symlinks first
    let write_file = |path: &std::path::PathBuf, content: &str| {
        if path.exists() || path.is_symlink() {
            fs::remove_file(path).ok();
        }
        fs::write(path, content).ok();
    };

    // Restore .bashrc
    write_file(
        &src_dir.join(".bashrc"),
        "# Bashrc configuration\nexport PATH=\"$HOME/.local/bin:$PATH\"\nalias ls='ls --color=auto'\n",
    );

    // Restore .zshrc
    write_file(
        &src_dir.join(".zshrc"),
        "# ZSH Configuration\nexport PATH=\"$HOME/bin:$PATH\"\nalias ll='ls -la'\n",
    );

    // Restore .vimrc
    write_file(
        &src_dir.join(".vimrc"),
        "\" Vim Configuration\nset number\nset expandtab\nset tabstop=4\n",
    );

    // Restore .gitconfig
    write_file(
        &src_dir.join(".gitconfig"),
        "# Git Configuration\n[user]\n    name = Test User\n    email = test@example.com\n[core]\n    editor = vim\n",
    );

    // Restore nvim/init.lua
    let nvim_dir = src_dir.join("nvim");
    if nvim_dir.is_symlink() {
        fs::remove_file(&nvim_dir).ok();
    }
    fs::create_dir_all(&nvim_dir).ok();
    write_file(
        &src_dir.join("nvim/init.lua"),
        "-- Neovim configuration\nvim.opt.number = true\nvim.opt.expandtab = true\n",
    );

    // Restore tmux files
    let tmux_dir = src_dir.join("tmux");
    if tmux_dir.is_symlink() {
        fs::remove_file(&tmux_dir).ok();
    }
    fs::create_dir_all(&tmux_dir).ok();
    write_file(
        &src_dir.join("tmux/tmux.conf"),
        "# Tmux Configuration\nset -g mouse on\nbind-key r source-file ~/.tmux.conf\n",
    );
    write_file(
        &src_dir.join("tmux/theme.conf"),
        "# Tmux Theme\nset -g status-bg blue\nset -g status-fg white\n",
    );

    // Restore alacritty config
    let alacritty_dir = src_dir.join("config/alacritty");
    // Remove symlink if exists
    let config_dir = src_dir.join("config");
    if config_dir.join("alacritty").is_symlink() {
        fs::remove_file(config_dir.join("alacritty")).ok();
    }
    fs::create_dir_all(&alacritty_dir).ok();
    write_file(
        &src_dir.join("config/alacritty/alacritty.yml"),
        "# Alacritty Configuration\nwindow:\n  padding:\n    x: 10\n    y: 10\nfont:\n  size: 12.0\n",
    );
}

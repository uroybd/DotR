//! What every integration test file shares: the playground (a repository in `tests/playground`, restored before and
//! after each test, which is why the tests run one at a time) or a fresh temporary one, and the commands run on it.
#![allow(
    dead_code,
    reason = "shared by every test file; not each one calls every helper"
)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use dotr_dear::{
    cli::{Cli, Command, DeployArgs, ImportArgs, InitArgs, UpdateArgs, run_cli},
    config::Config,
    merge::Resolution,
};

pub const PLAYGROUND_DIR: &str = "tests/playground";

/// A repository a test runs dotr on, cleaned up when dropped.
pub struct Playground {
    pub cwd: PathBuf,
    /// A temporary directory of this test's own (removed when dropped), not the shared playground.
    temporary: bool,
}

impl Playground {
    /// The shared playground, restored first: a test never depends on the one before it having cleaned up.
    pub fn new() -> Self {
        let cwd = PathBuf::from(PLAYGROUND_DIR);
        setup(&cwd);
        Self {
            cwd,
            temporary: false,
        }
    }

    /// An empty temporary directory of this test's own.
    pub fn temp() -> Self {
        let cwd = std::env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&cwd).expect("Failed to create temp dir");
        Self {
            cwd,
            temporary: true,
        }
    }

    pub fn get_cli(&self, command: Option<Command>) -> Cli {
        Cli {
            command,
            working_dir: Some(self.cwd.display().to_string()),
        }
    }

    pub fn init(&self) {
        run_cli(self.get_cli(Some(Command::Init(InitArgs {})))).expect("Init failed");
    }

    pub fn get_config(&self) -> Config {
        Config::from_path(&self.cwd).expect("Failed to load config")
    }

    pub fn import(&self, path: &str) {
        self.import_with(path, false);
    }

    pub fn import_with(&self, path: &str, symlink: bool) {
        run_cli(self.get_cli(Some(Command::Import(ImportArgs {
            path: path.to_string(),
            symlink,
            ..Default::default()
        }))))
        .expect("Import failed");
    }

    /// Deploys `packages` (every one of the default profile's with `None`), without cleaning.
    pub fn deploy(&self, packages: Option<Vec<String>>) {
        self.deploy_as(packages, None);
    }

    pub fn deploy_as(&self, packages: Option<Vec<String>>, profile: Option<String>) {
        run_cli(self.get_cli(Some(Command::Deploy(DeployArgs {
            packages,
            profile,
            clean: Some(false),
            ..Default::default()
        }))))
        .expect("Deploy failed");
    }

    /// Backs up `packages` without cleaning, leaving any conflict unsettled (a test never waits on a terminal).
    pub fn update(&self, packages: Option<Vec<String>>) {
        run_cli(self.get_cli(Some(Command::Update(UpdateArgs {
            packages,
            clean: Some(false),
            conflict: Some(Resolution::Skip),
            ..Default::default()
        }))))
        .expect("Update failed");
    }
}

impl Drop for Playground {
    fn drop(&mut self) {
        if self.temporary {
            fs::remove_dir_all(&self.cwd).ok();
        } else {
            teardown(&self.cwd);
        }
    }
}

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
    // Prompted answers, and the folder symlinked packages are staged in
    fs::remove_file(cwd.join(".uservariables.toml")).ok();
    fs::remove_dir_all(cwd.join(dotr_dear::utils::SYMLINK_FOLDER)).ok();
    // A foreign file the unfold_symlink tests put beside their symlinks: not in the restore list, and a test that
    // panicked can leave it (or a dangling symlink) behind. `remove_file` takes both.
    fs::remove_file(cwd.join("src/tmux/foreign.txt")).ok();
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

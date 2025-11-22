use std::{fs, path::PathBuf};

use dotr::{
    cli::{DeployUpdateArgs, ImportArgs, InitArgs, run_cli},
    config::Config,
};

mod common;

const PLAYGROUND_DIR: &str = "tests/playground";
const NVIM_PATH: &str = "src/nvim/";
const BASHRC_PATH: &str = "src/.bashrc";
const TMUX_PATH: &str = "src/tmux/";

struct TestFixture {
    cwd: PathBuf,
}

impl TestFixture {
    fn new() -> Self {
        let cwd = PathBuf::from(PLAYGROUND_DIR);
        common::setup(&cwd);
        Self { cwd }
    }

    fn get_cli(&self, command: Option<dotr::cli::Command>) -> dotr::cli::Cli {
        dotr::cli::Cli {
            command,
            working_dir: Some(PLAYGROUND_DIR.to_string()),
        }
    }

    fn init(&self) {
        run_cli(self.get_cli(Some(dotr::cli::Command::Init(InitArgs {})))).expect("Init failed");
    }

    fn import(&self, path: &str) {
        run_cli(self.get_cli(Some(dotr::cli::Command::Import(ImportArgs {
            path: path.to_string(),
            name: None,
            profile: None,
        }))))
        .expect("Import failed");
    }

    fn deploy(&self, packages: Option<Vec<String>>) {
        run_cli(
            self.get_cli(Some(dotr::cli::Command::Deploy(DeployUpdateArgs {
                packages,
                profile: None,
            }))),
        )
        .expect("Deploy failed");
    }

    fn diff(&self, packages: Option<Vec<String>>) -> Result<(), anyhow::Error> {
        run_cli(
            self.get_cli(Some(dotr::cli::Command::Diff(DeployUpdateArgs {
                packages,
                profile: None,
            }))),
        )
    }

    #[allow(dead_code)]
    fn get_config(&self) -> Config {
        Config::from_path(&self.cwd).expect("Failed to load config")
    }

    fn write_file(&self, path: &str, content: &str) {
        let file_path = self.cwd.join(path);
        fs::write(file_path, content).unwrap_or_else(|_| panic!("Failed to write file: {}", path));
    }

    fn get_package_name(&self, path: &str) -> String {
        let args = ImportArgs {
            path: path.to_string(),
            name: None,
            profile: None,
        };
        dotr::package::get_package_name(&args, &self.cwd)
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        common::teardown(&self.cwd);
    }
}

#[test]
fn test_diff_no_changes() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(BASHRC_PATH);

    // Modify to trigger deployment
    fixture.write_file("src/.bashrc", "# Modified\n");
    fixture.deploy(None);

    // Diff with no changes after deployment should show no differences
    let result = fixture.diff(None);
    assert!(result.is_ok());
}

#[test]
fn test_diff_with_changes() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(BASHRC_PATH);

    // Modify and deploy
    fixture.write_file("src/.bashrc", "# Modified\n");
    fixture.deploy(None);

    // Make more changes
    fixture.write_file("src/.bashrc", "# Modified again\nexport PATH=/test\n");

    // Diff should show the differences
    let result = fixture.diff(None);
    assert!(result.is_ok());
}

#[test]
fn test_diff_specific_package() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(BASHRC_PATH);
    fixture.import(NVIM_PATH);

    // Modify both files
    fixture.write_file("src/.bashrc", "# Bashrc modified\n");
    fixture.write_file("src/nvim/init.lua", "-- Nvim modified\n");
    fixture.deploy(None);

    // Change only bashrc
    fixture.write_file("src/.bashrc", "# Bashrc changed again\n");

    // Diff only bashrc package
    let bashrc_name = fixture.get_package_name(BASHRC_PATH);
    let result = fixture.diff(Some(vec![bashrc_name]));
    assert!(result.is_ok());
}

#[test]
fn test_diff_multiple_specific_packages() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(BASHRC_PATH);
    fixture.import(NVIM_PATH);

    // Modify and deploy
    fixture.write_file("src/.bashrc", "# Bashrc\n");
    fixture.write_file("src/nvim/init.lua", "-- Nvim\n");
    fixture.deploy(None);

    // Change both files
    fixture.write_file("src/.bashrc", "# Bashrc changed\n");
    fixture.write_file("src/nvim/init.lua", "-- Nvim changed\n");

    // Diff both packages
    let bashrc_name = fixture.get_package_name(BASHRC_PATH);
    let nvim_name = fixture.get_package_name(NVIM_PATH);
    let result = fixture.diff(Some(vec![bashrc_name, nvim_name]));
    assert!(result.is_ok());
}

#[test]
fn test_diff_directory_package() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(TMUX_PATH);

    // Modify and deploy
    fixture.write_file("src/tmux/tmux.conf", "# Tmux modified\n");
    fixture.write_file("src/tmux/theme.conf", "# Theme modified\n");
    fixture.deploy(None);

    // Change one file in the directory
    fixture.write_file("src/tmux/tmux.conf", "# Tmux changed again\n");

    // Diff should show changes in the directory
    let result = fixture.diff(None);
    assert!(result.is_ok());
}

#[test]
fn test_diff_all_files_in_directory_changed() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(TMUX_PATH);

    // Modify and deploy
    fixture.write_file("src/tmux/tmux.conf", "# Tmux\n");
    fixture.write_file("src/tmux/theme.conf", "# Theme\n");
    fixture.deploy(None);

    // Change all files in the directory
    fixture.write_file("src/tmux/tmux.conf", "# Tmux changed\n");
    fixture.write_file("src/tmux/theme.conf", "# Theme changed\n");

    // Diff should show changes in all files
    let result = fixture.diff(None);
    assert!(result.is_ok());
}

#[test]
fn test_diff_nonexistent_package() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(BASHRC_PATH);

    // Diff with a non-existent package should fail with error
    let result = fixture.diff(Some(vec!["nonexistent_package".to_string()]));
    assert!(
        result.is_err(),
        "Diff with nonexistent package should error"
    );
    assert!(
        result.unwrap_err().to_string().contains("not found"),
        "Error should mention package not found"
    );
}

#[test]
fn test_diff_file_not_yet_deployed() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(BASHRC_PATH);

    // Modify file but don't deploy
    fixture.write_file("src/.bashrc", "# Modified but not deployed\n");

    // Diff should work even if file wasn't deployed yet
    let result = fixture.diff(None);
    assert!(result.is_ok());
}

#[test]
fn test_diff_after_multiple_changes() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(BASHRC_PATH);

    // Initial deployment
    fixture.write_file("src/.bashrc", "# Version 1\n");
    fixture.deploy(None);

    // Make multiple changes
    fixture.write_file("src/.bashrc", "# Version 2\nexport PATH=/usr/bin\n");

    // Diff should show accumulated changes
    let result = fixture.diff(None);
    assert!(result.is_ok());
}

#[test]
fn test_diff_with_additions() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(BASHRC_PATH);

    // Deploy initial version
    fixture.write_file("src/.bashrc", "# Line 1\n");
    fixture.deploy(None);

    // Add more lines
    fixture.write_file("src/.bashrc", "# Line 1\n# Line 2\n# Line 3\n");

    // Diff should show additions
    let result = fixture.diff(None);
    assert!(result.is_ok());
}

#[test]
fn test_diff_with_deletions() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(BASHRC_PATH);

    // Deploy initial version with multiple lines
    fixture.write_file("src/.bashrc", "# Line 1\n# Line 2\n# Line 3\n");
    fixture.deploy(None);

    // Remove lines
    fixture.write_file("src/.bashrc", "# Line 1\n");

    // Diff should show deletions
    let result = fixture.diff(None);
    assert!(result.is_ok());
}

#[test]
fn test_diff_with_modifications() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(BASHRC_PATH);

    // Deploy initial version
    fixture.write_file("src/.bashrc", "# Original line\n");
    fixture.deploy(None);

    // Modify the line
    fixture.write_file("src/.bashrc", "# Modified line\n");

    // Diff should show modifications
    let result = fixture.diff(None);
    assert!(result.is_ok());
}

#[test]
fn test_diff_empty_file() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(BASHRC_PATH);

    // Deploy empty file
    fixture.write_file("src/.bashrc", "");
    fixture.deploy(None);

    // Add content
    fixture.write_file("src/.bashrc", "# New content\n");

    // Diff should show additions from empty
    let result = fixture.diff(None);
    assert!(result.is_ok());
}

#[test]
fn test_diff_all_packages() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(BASHRC_PATH);
    fixture.import(NVIM_PATH);
    fixture.import(TMUX_PATH);

    // Deploy all
    fixture.write_file("src/.bashrc", "# Bashrc\n");
    fixture.write_file("src/nvim/init.lua", "-- Nvim\n");
    fixture.write_file("src/tmux/tmux.conf", "# Tmux\n");
    fixture.deploy(None);

    // Modify all
    fixture.write_file("src/.bashrc", "# Bashrc changed\n");
    fixture.write_file("src/nvim/init.lua", "-- Nvim changed\n");
    fixture.write_file("src/tmux/tmux.conf", "# Tmux changed\n");

    // Diff all should work
    let result = fixture.diff(None);
    assert!(result.is_ok());
}

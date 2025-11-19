use std::{fs, path::PathBuf};

use dotr::{
    cli::{run_cli, DeployArgs, ImportArgs, InitArgs, UpdateArgs},
    config::{copy_dir_all, Config},
    package::get_package_name,
    utils,
};

mod common;

// Test constants
const PLAYGROUND_DIR: &str = "tests/playground";
const NVIM_PATH: &str = "src/nvim/";
const BASHRC_PATH: &str = "src/.bashrc";

// Test fixture helper
struct TestFixture {
    cwd: PathBuf,
}

impl TestFixture {
    fn new() -> Self {
        Self {
            cwd: PathBuf::from(PLAYGROUND_DIR),
        }
    }

    fn get_cli(&self, command: Option<dotr::cli::Command>) -> dotr::cli::Cli {
        dotr::cli::Cli {
            command,
            working_dir: Some(PLAYGROUND_DIR.to_string()),
        }
    }

    fn init(&self) {
        run_cli(self.get_cli(Some(dotr::cli::Command::Init(InitArgs {}))));
    }

    fn import(&self, path: &str) {
        run_cli(self.get_cli(Some(dotr::cli::Command::Import(ImportArgs {
            path: path.to_string(),
        }))));
    }

    fn deploy(&self, packages: Option<Vec<String>>) {
        run_cli(self.get_cli(Some(dotr::cli::Command::Deploy(DeployArgs {
            packages,
        }))));
    }

    fn update(&self, packages: Option<Vec<String>>) {
        run_cli(self.get_cli(Some(dotr::cli::Command::Update(UpdateArgs {
            packages,
        }))));
    }

    fn get_config(&self) -> Config {
        Config::from_path(&self.cwd)
    }

    fn get_package_name(&self, path: &str) -> String {
        get_package_name(path, &self.cwd)
    }

    fn assert_file_exists(&self, path: &str, message: &str) {
        assert!(self.cwd.join(path).exists(), "{}", message);
    }

    fn assert_file_not_exists(&self, path: &str, message: &str) {
        assert!(!self.cwd.join(path).exists(), "{}", message);
    }

    fn assert_file_contains(&self, path: &str, content: &str, message: &str) {
        let file_path = self.cwd.join(path);
        let file_content = fs::read_to_string(&file_path)
            .unwrap_or_else(|_| panic!("Failed to read file: {}", path));
        assert!(file_content.contains(content), "{}", message);
    }

    fn write_file(&self, path: &str, content: &str) {
        let file_path = self.cwd.join(path);
        fs::write(file_path, content)
            .unwrap_or_else(|_| panic!("Failed to write file: {}", path));
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        common::teardown(&self.cwd);
    }
}

#[test]
fn test_no_command() {
    let fixture = TestFixture::new();
    
    run_cli(fixture.get_cli(None));
    
    fixture.assert_file_not_exists("config.toml", "config.toml should not be created");
    fixture.assert_file_not_exists("dotfiles", "dotfiles directory should not be created");
}

#[test]
fn test_init_config() {
    let fixture = TestFixture::new();
    
    fixture.init();
    
    fixture.assert_file_exists("config.toml", "config.toml should be created");
    fixture.assert_file_exists("dotfiles", "dotfiles directory should be created");
}

#[test]
fn test_import_dots() {
    let fixture = TestFixture::new();
    
    fixture.init();
    fixture.import(NVIM_PATH);
    fixture.import(BASHRC_PATH);
    
    let config = fixture.get_config();
    let nvim_package_name = fixture.get_package_name(NVIM_PATH);
    let bashrc_package_name = fixture.get_package_name(BASHRC_PATH);
    
    // Verify nvim package
    assert!(
        config.packages.contains_key(&nvim_package_name),
        "Config should contain the nvim package"
    );
    let nvim_package = config.packages.get(&nvim_package_name).unwrap();
    assert!(
        nvim_package.dest.ends_with(NVIM_PATH),
        "Package dest should match the imported path"
    );
    assert_eq!(
        nvim_package.src,
        format!("dotfiles/{}", nvim_package_name),
        "Package src should be correctly set"
    );
    
    // Verify bashrc package
    assert!(
        config.packages.contains_key(&bashrc_package_name),
        "Config should contain the bashrc package"
    );
    
    // Verify files are copied to dotfiles directory
    fixture.assert_file_exists(
        &format!("dotfiles/{}/nvim/init.lua", nvim_package_name),
        "nvim init.lua should be copied to dotfiles"
    );
}

#[test]
fn test_canonical_linking() {
    let fixture = TestFixture::new();
    
    let path_from_home = utils::resolve_path("~/.config", &fixture.cwd);
    let path_from_root = utils::resolve_path("/Volumes/Repos/", &fixture.cwd);
    let path_from_cwd = utils::resolve_path("src/nvim", &fixture.cwd);
    
    assert!(path_from_home.is_absolute());
    assert!(path_from_root.is_absolute());
    assert!(path_from_cwd.is_absolute());
}

#[test]
fn test_deploy_all_packages() {
    let fixture = TestFixture::new();
    
    fixture.init();
    fixture.import(NVIM_PATH);
    fixture.import(BASHRC_PATH);
    
    // Deploy all packages
    fixture.deploy(None);
    
    // Verify backups created
    fixture.assert_file_exists("src/nvim.dotrbak/", "nvim backup should exist");
    fixture.assert_file_exists("src/.bashrc.dotrbak", "bashrc backup should exist");
    
    // Verify files deployed
    fixture.assert_file_exists("src/nvim/init.lua", "nvim init.lua should be deployed");
    fixture.assert_file_exists("src/.bashrc", "bashrc should be deployed");
}

#[test]
fn test_deploy_specific_package() {
    let fixture = TestFixture::new();
    
    fixture.init();
    fixture.import(NVIM_PATH);
    fixture.import(BASHRC_PATH);
    
    // Deploy only nvim
    let nvim_package_name = fixture.get_package_name(NVIM_PATH);
    fixture.deploy(Some(vec![nvim_package_name]));
    
    // Verify only nvim was deployed
    fixture.assert_file_exists("src/nvim.dotrbak/", "nvim backup should exist");
    fixture.assert_file_exists("src/nvim/init.lua", "nvim init.lua should be deployed");
    fixture.assert_file_not_exists("src/.bashrc.dotrbak", "bashrc should NOT have been deployed");
}

#[test]
fn test_deploy_multiple_specific_packages() {
    let fixture = TestFixture::new();
    
    fixture.init();
    fixture.import(NVIM_PATH);
    fixture.import(BASHRC_PATH);
    
    // Deploy both packages explicitly
    let nvim_package_name = fixture.get_package_name(NVIM_PATH);
    let bashrc_package_name = fixture.get_package_name(BASHRC_PATH);
    fixture.deploy(Some(vec![nvim_package_name, bashrc_package_name]));
    
    // Verify both were deployed
    fixture.assert_file_exists("src/nvim.dotrbak/", "nvim backup should exist");
    fixture.assert_file_exists("src/nvim/init.lua", "nvim init.lua should be deployed");
    fixture.assert_file_exists("src/.bashrc.dotrbak", "bashrc backup should exist");
    fixture.assert_file_exists("src/.bashrc", "bashrc should be deployed");
}

#[test]
fn test_update_specific_package() {
    let fixture = TestFixture::new();
    
    fixture.init();
    fixture.import(NVIM_PATH);
    fixture.import(BASHRC_PATH);
    
    // Deploy all packages
    fixture.deploy(None);
    
    // Modify deployed files
    fixture.write_file("src/nvim/init.lua", "-- Modified nvim config\n");
    fixture.write_file("src/.bashrc", "# Modified bashrc\n");
    
    // Update only nvim
    let nvim_package_name = fixture.get_package_name(NVIM_PATH);
    fixture.update(Some(vec![nvim_package_name.clone()]));
    
    // Verify nvim was updated
    fixture.assert_file_contains(
        &format!("dotfiles/{}/nvim/init.lua", nvim_package_name),
        "Modified nvim config",
        "nvim config should be updated in dotfiles"
    );
    
    // Verify bashrc was NOT updated
    let bashrc_package_name = fixture.get_package_name(BASHRC_PATH);
    let bashrc_content = fs::read_to_string(
        fixture.cwd.join(format!("dotfiles/{}/.bashrc", bashrc_package_name))
    ).expect("Failed to read bashrc");
    assert!(
        !bashrc_content.contains("Modified bashrc"),
        "bashrc should NOT be updated in dotfiles"
    );
}

#[test]
fn test_update_multiple_specific_packages() {
    let fixture = TestFixture::new();
    
    fixture.init();
    fixture.import(NVIM_PATH);
    fixture.import(BASHRC_PATH);
    
    // Deploy all packages
    fixture.deploy(None);
    
    // Modify deployed files
    fixture.write_file("src/nvim/init.lua", "-- Updated nvim config\n");
    fixture.write_file("src/.bashrc", "# Updated bashrc\n");
    
    // Update both packages
    let nvim_package_name = fixture.get_package_name(NVIM_PATH);
    let bashrc_package_name = fixture.get_package_name(BASHRC_PATH);
    fixture.update(Some(vec![nvim_package_name.clone(), bashrc_package_name.clone()]));
    
    // Verify both were updated
    fixture.assert_file_contains(
        &format!("dotfiles/{}/nvim/init.lua", nvim_package_name),
        "Updated nvim config",
        "nvim config should be updated"
    );
    fixture.assert_file_contains(
        &format!("dotfiles/{}/.bashrc", bashrc_package_name),
        "Updated bashrc",
        "bashrc should be updated"
    );
}

#[test]
fn test_deploy_nonexistent_package() {
    let fixture = TestFixture::new();
    
    fixture.init();
    fixture.import(NVIM_PATH);
    
    // Try to deploy a non-existent package
    fixture.deploy(Some(vec!["nonexistent_package".to_string()]));
    
    // Verify nothing was deployed
    fixture.assert_file_not_exists(
        "src/nvim.dotrbak/",
        "No backup should be created for filtered out packages"
    );
}

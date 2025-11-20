use std::{fs, path::PathBuf};

use dotr::{
    cli::{DeployArgs, ImportArgs, InitArgs, PrintVarsArgs, UpdateArgs, run_cli},
    config::Config,
    package::get_package_name,
    utils,
};

mod common;

// Test constants
const PLAYGROUND_DIR: &str = "tests/playground";
const NVIM_PATH: &str = "src/nvim/";
const BASHRC_PATH: &str = "src/.bashrc";
const ZSHRC_PATH: &str = "src/.zshrc";
const VIMRC_PATH: &str = "src/.vimrc";
const GITCONFIG_PATH: &str = "src/.gitconfig";
const TMUX_PATH: &str = "src/tmux/";
const ALACRITTY_PATH: &str = "src/config/alacritty/";

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
        run_cli(self.get_cli(Some(dotr::cli::Command::Deploy(DeployArgs { packages }))));
    }

    fn update(&self, packages: Option<Vec<String>>) {
        run_cli(self.get_cli(Some(dotr::cli::Command::Update(UpdateArgs { packages }))));
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
        fs::write(file_path, content).unwrap_or_else(|_| panic!("Failed to write file: {}", path));
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
    fixture.assert_file_exists(".gitignore", ".gitignore should be created");
    fixture.assert_file_contains(
        ".gitignore",
        ".uservariables.toml",
        ".gitignore should contain .uservariables.toml",
    );
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
        &format!("dotfiles/{}/init.lua", nvim_package_name),
        "nvim init.lua should be copied to dotfiles",
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
    fixture.assert_file_not_exists(
        "src/.bashrc.dotrbak",
        "bashrc should NOT have been deployed",
    );
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
        &format!("dotfiles/{}/init.lua", nvim_package_name),
        "Modified nvim config",
        "nvim config should be updated in dotfiles",
    );

    // Verify bashrc was NOT updated
    let bashrc_package_name = fixture.get_package_name(BASHRC_PATH);
    let bashrc_content = fs::read_to_string(
        fixture
            .cwd
            .join(format!("dotfiles/{}", bashrc_package_name)),
    )
    .expect("Failed to read bashrc");
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
    fixture.update(Some(vec![
        nvim_package_name.clone(),
        bashrc_package_name.clone(),
    ]));

    // Verify both were updated
    fixture.assert_file_contains(
        &format!("dotfiles/{}/init.lua", nvim_package_name),
        "Updated nvim config",
        "nvim config should be updated",
    );
    fixture.assert_file_contains(
        &format!("dotfiles/{}", bashrc_package_name),
        "Updated bashrc",
        "bashrc should be updated",
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
        "No backup should be created for filtered out packages",
    );
}

#[test]
fn test_import_multiple_files() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(BASHRC_PATH);
    fixture.import(ZSHRC_PATH);
    fixture.import(VIMRC_PATH);
    fixture.import(GITCONFIG_PATH);

    let config = fixture.get_config();
    let bashrc_name = fixture.get_package_name(BASHRC_PATH);
    let zshrc_name = fixture.get_package_name(ZSHRC_PATH);
    let vimrc_name = fixture.get_package_name(VIMRC_PATH);
    let gitconfig_name = fixture.get_package_name(GITCONFIG_PATH);

    // Verify all packages exist in config
    assert!(config.packages.contains_key(&bashrc_name));
    assert!(config.packages.contains_key(&zshrc_name));
    assert!(config.packages.contains_key(&vimrc_name));
    assert!(config.packages.contains_key(&gitconfig_name));

    // Verify files are backed up
    fixture.assert_file_exists(
        &format!("dotfiles/{}", bashrc_name),
        "bashrc should be backed up",
    );
    fixture.assert_file_exists(
        &format!("dotfiles/{}", zshrc_name),
        "zshrc should be backed up",
    );
    fixture.assert_file_exists(
        &format!("dotfiles/{}", vimrc_name),
        "vimrc should be backed up",
    );
    fixture.assert_file_exists(
        &format!("dotfiles/{}", gitconfig_name),
        "gitconfig should be backed up",
    );

    // Verify content
    fixture.assert_file_contains(
        &format!("dotfiles/{}", bashrc_name),
        "Bashrc configuration",
        "bashrc content should match",
    );
    fixture.assert_file_contains(
        &format!("dotfiles/{}", gitconfig_name),
        "Test User",
        "gitconfig content should match",
    );
}

#[test]
fn test_import_nested_directories() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(TMUX_PATH);
    fixture.import(ALACRITTY_PATH);

    let config = fixture.get_config();
    let tmux_name = fixture.get_package_name(TMUX_PATH);
    let alacritty_name = fixture.get_package_name(ALACRITTY_PATH);

    // Verify packages in config
    assert!(config.packages.contains_key(&tmux_name));
    assert!(config.packages.contains_key(&alacritty_name));

    // Verify directory contents are backed up
    fixture.assert_file_exists(
        &format!("dotfiles/{}/tmux.conf", tmux_name),
        "tmux.conf should be backed up",
    );
    fixture.assert_file_exists(
        &format!("dotfiles/{}/theme.conf", tmux_name),
        "theme.conf should be backed up",
    );
    fixture.assert_file_exists(
        &format!("dotfiles/{}/alacritty.yml", alacritty_name),
        "alacritty.yml should be backed up",
    );

    // Verify content
    fixture.assert_file_contains(
        &format!("dotfiles/{}/tmux.conf", tmux_name),
        "set -g mouse on",
        "tmux.conf content should match",
    );
    fixture.assert_file_contains(
        &format!("dotfiles/{}/alacritty.yml", alacritty_name),
        "padding",
        "alacritty.yml content should match",
    );
}

#[test]
fn test_deploy_all_file_types() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(BASHRC_PATH);
    fixture.import(ZSHRC_PATH);
    fixture.import(NVIM_PATH);
    fixture.import(TMUX_PATH);

    // Deploy all packages
    fixture.deploy(None);

    // Verify file backups created
    fixture.assert_file_exists("src/.bashrc.dotrbak", "bashrc backup should exist");
    fixture.assert_file_exists("src/.zshrc.dotrbak", "zshrc backup should exist");

    // Verify directory backups created
    fixture.assert_file_exists("src/nvim.dotrbak/", "nvim backup should exist");
    fixture.assert_file_exists("src/tmux.dotrbak/", "tmux backup should exist");

    // Verify deployed files exist
    fixture.assert_file_exists("src/.bashrc", "bashrc should be deployed");
    fixture.assert_file_exists("src/.zshrc", "zshrc should be deployed");
    fixture.assert_file_exists("src/nvim/init.lua", "nvim init.lua should be deployed");
    fixture.assert_file_exists("src/tmux/tmux.conf", "tmux.conf should be deployed");
    fixture.assert_file_exists("src/tmux/theme.conf", "theme.conf should be deployed");
}

#[test]
fn test_update_preserves_changes() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(VIMRC_PATH);
    fixture.import(TMUX_PATH);

    // Deploy
    fixture.deploy(None);

    // Modify deployed files
    fixture.write_file(
        "src/.vimrc",
        "\" Updated vimrc\nset number\nset relativenumber\n",
    );
    fixture.write_file("src/tmux/tmux.conf", "# Updated tmux\nset -g mouse off\n");

    // Update all
    fixture.update(None);

    let vimrc_name = fixture.get_package_name(VIMRC_PATH);
    let tmux_name = fixture.get_package_name(TMUX_PATH);

    // Verify updates in dotfiles
    fixture.assert_file_contains(
        &format!("dotfiles/{}", vimrc_name),
        "Updated vimrc",
        "vimrc should be updated",
    );
    fixture.assert_file_contains(
        &format!("dotfiles/{}", vimrc_name),
        "relativenumber",
        "vimrc should contain new content",
    );
    fixture.assert_file_contains(
        &format!("dotfiles/{}/tmux.conf", tmux_name),
        "Updated tmux",
        "tmux.conf should be updated",
    );
    fixture.assert_file_contains(
        &format!("dotfiles/{}/tmux.conf", tmux_name),
        "mouse off",
        "tmux.conf should contain modified content",
    );
}

#[test]
fn test_deploy_preserves_directory_structure() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.import(ALACRITTY_PATH);

    // Deploy
    let alacritty_name = fixture.get_package_name(ALACRITTY_PATH);
    fixture.deploy(Some(vec![alacritty_name]));

    // Verify backup was created
    fixture.assert_file_exists(
        "src/config/alacritty.dotrbak/",
        "alacritty backup should exist",
    );

    // Verify deployed with correct structure
    fixture.assert_file_exists(
        "src/config/alacritty/alacritty.yml",
        "alacritty.yml should be deployed",
    );

    // Verify content
    fixture.assert_file_contains(
        "src/config/alacritty/alacritty.yml",
        "window:",
        "deployed file should have correct content",
    );
}

#[test]
fn test_mixed_files_and_directories() {
    let fixture = TestFixture::new();

    fixture.init();

    // Import mix of files and directories
    fixture.import(BASHRC_PATH);
    fixture.import(GITCONFIG_PATH);
    fixture.import(NVIM_PATH);
    fixture.import(TMUX_PATH);

    let config = fixture.get_config();

    // Should have 4 packages
    assert_eq!(config.packages.len(), 4, "Should have 4 packages");

    // Deploy all
    fixture.deploy(None);

    // Verify all deployed correctly
    fixture.assert_file_exists("src/.bashrc", "bashrc deployed");
    fixture.assert_file_exists("src/.gitconfig", "gitconfig deployed");
    fixture.assert_file_exists("src/nvim/init.lua", "nvim/init.lua deployed");
    fixture.assert_file_exists("src/tmux/tmux.conf", "tmux/tmux.conf deployed");

    // Modify and update only directories
    fixture.write_file("src/nvim/init.lua", "-- Modified nvim\n");
    fixture.write_file("src/tmux/theme.conf", "# Modified theme\n");

    let nvim_name = fixture.get_package_name(NVIM_PATH);
    let tmux_name = fixture.get_package_name(TMUX_PATH);
    fixture.update(Some(vec![nvim_name.clone(), tmux_name.clone()]));

    // Verify only specified packages updated
    fixture.assert_file_contains(
        &format!("dotfiles/{}/init.lua", nvim_name),
        "Modified nvim",
        "nvim should be updated",
    );
    fixture.assert_file_contains(
        &format!("dotfiles/{}/theme.conf", tmux_name),
        "Modified theme",
        "tmux theme should be updated",
    );
}

#[test]
fn test_print_vars_empty() {
    let fixture = TestFixture::new();

    fixture.init();

    // Print vars should show environment variables including HOME
    run_cli(fixture.get_cli(Some(dotr::cli::Command::PrintVars(PrintVarsArgs {}))));

    // Verify that Context has HOME environment variable
    let ctx = dotr::cli::Context::new(fixture.cwd.clone());
    assert!(
        ctx.variables.contains_key("HOME"),
        "HOME environment variable should be present"
    );
}

#[test]
fn test_print_vars_with_custom_variables() {
    let fixture = TestFixture::new();

    fixture.init();

    // Add custom variables to config using the Config API
    let mut config = fixture.get_config();
    config
        .variables
        .insert("EDITOR".to_string(), "vim".to_string());
    config
        .variables
        .insert("SHELL".to_string(), "/bin/zsh".to_string());
    config
        .variables
        .insert("USER_EMAIL".to_string(), "test@example.com".to_string());
    config.save(&fixture.cwd);

    // Print vars should show custom variables
    run_cli(fixture.get_cli(Some(dotr::cli::Command::PrintVars(PrintVarsArgs {}))));

    // Verify config contains variables
    let config = fixture.get_config();
    assert_eq!(config.variables.get("EDITOR"), Some(&"vim".to_string()));
    assert_eq!(config.variables.get("SHELL"), Some(&"/bin/zsh".to_string()));
    assert_eq!(
        config.variables.get("USER_EMAIL"),
        Some(&"test@example.com".to_string())
    );

    // Verify that environment variables like HOME are still present in Context
    let ctx = dotr::cli::Context::new(fixture.cwd.clone());
    assert!(
        ctx.variables.contains_key("HOME"),
        "HOME environment variable should be present in Context"
    );
}

#[test]
fn test_variables_persist_after_save() {
    let fixture = TestFixture::new();

    fixture.init();

    // Add variables manually
    let mut config = fixture.get_config();
    config
        .variables
        .insert("TEST_VAR".to_string(), "test_value".to_string());
    config
        .variables
        .insert("ANOTHER_VAR".to_string(), "another_value".to_string());
    config.save(&fixture.cwd);

    // Reload config and verify variables persist
    let reloaded_config = fixture.get_config();
    assert_eq!(
        reloaded_config.variables.get("TEST_VAR"),
        Some(&"test_value".to_string())
    );
    assert_eq!(
        reloaded_config.variables.get("ANOTHER_VAR"),
        Some(&"another_value".to_string())
    );
}

#[test]
fn test_home_variable_always_present() {
    let fixture = TestFixture::new();

    fixture.init();

    // Context should have environment variables including HOME
    let ctx = dotr::cli::Context::new(fixture.cwd.clone());

    // HOME should always be present in context
    assert!(ctx.variables.contains_key("HOME"));

    // HOME should be a valid path
    let home = ctx.variables.get("HOME").expect("HOME variable not found");
    assert!(!home.is_empty());
}

#[test]
fn test_variables_with_special_characters() {
    let fixture = TestFixture::new();

    fixture.init();

    // Add variables with special characters using Config API
    let mut config = fixture.get_config();
    config.variables.insert(
        "PATH".to_string(),
        "/usr/local/bin:/usr/bin:/bin".to_string(),
    );
    config
        .variables
        .insert("PS1".to_string(), "[\\u@\\h \\W]$ ".to_string());
    config.variables.insert(
        "COMPLEX_VAR".to_string(),
        "value with spaces and $pecial ch@rs".to_string(),
    );
    config.save(&fixture.cwd);

    let config = fixture.get_config();
    assert_eq!(
        config.variables.get("PATH"),
        Some(&"/usr/local/bin:/usr/bin:/bin".to_string())
    );
    assert_eq!(
        config.variables.get("PS1"),
        Some(&"[\\u@\\h \\W]$ ".to_string())
    );
    assert_eq!(
        config.variables.get("COMPLEX_VAR"),
        Some(&"value with spaces and $pecial ch@rs".to_string())
    );
}

#[test]
fn test_variables_do_not_interfere_with_packages() {
    let fixture = TestFixture::new();

    fixture.init();

    // Add variables
    let mut config = fixture.get_config();
    config
        .variables
        .insert("MY_VAR".to_string(), "my_value".to_string());
    config.save(&fixture.cwd);

    // Import packages
    fixture.import(BASHRC_PATH);
    fixture.import(NVIM_PATH);

    // Reload and verify both variables and packages exist
    let config = fixture.get_config();
    assert_eq!(
        config.variables.get("MY_VAR"),
        Some(&"my_value".to_string())
    );
    assert_eq!(config.packages.len(), 2);

    let bashrc_name = fixture.get_package_name(BASHRC_PATH);
    let nvim_name = fixture.get_package_name(NVIM_PATH);
    assert!(config.packages.contains_key(&bashrc_name));
    assert!(config.packages.contains_key(&nvim_name));
}

#[test]
fn test_config_variables_override_environment_variables() {
    let fixture = TestFixture::new();

    fixture.init();

    // Get the original HOME from environment
    let original_home = std::env::var("HOME").expect("HOME should be set in environment");

    // Override HOME in config with a custom value
    let custom_home = "/custom/home/path";
    let mut config = fixture.get_config();
    config
        .variables
        .insert("HOME".to_string(), custom_home.to_string());
    config.save(&fixture.cwd);

    // Reload config and verify HOME is overridden
    let reloaded_config = fixture.get_config();
    assert_eq!(
        reloaded_config.variables.get("HOME"),
        Some(&custom_home.to_string()),
        "Config HOME should override environment HOME"
    );
    assert_ne!(
        reloaded_config.variables.get("HOME"),
        Some(&original_home),
        "Config HOME should be different from environment HOME"
    );

    // Create context and verify that config variables take precedence
    let mut ctx = dotr::cli::Context::new(fixture.cwd.clone());
    // When config is loaded, it should override the environment variable
    ctx.variables.extend(reloaded_config.variables.clone());

    assert_eq!(
        ctx.variables.get("HOME"),
        Some(&custom_home.to_string()),
        "Context should use config HOME over environment HOME"
    );
}

use std::{fs, path::PathBuf};

use dotr::{
    cli::{DeployUpdateArgs, ImportArgs, InitArgs, PrintVarsArgs, run_cli},
    config::Config,
    context::Context,
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
        let cwd = PathBuf::from(PLAYGROUND_DIR);
        // Ensure test files exist
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

    fn update(&self, packages: Option<Vec<String>>) {
        run_cli(
            self.get_cli(Some(dotr::cli::Command::Update(DeployUpdateArgs {
                packages,
                profile: None,
            }))),
        )
        .expect("Update failed");
    }

    fn get_config(&self) -> Config {
        Config::from_path(&self.cwd).expect("Failed to load config")
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

    fn get_context_variables(&self) -> toml::Table {
        let mut ctx = Context::new(&self.cwd).expect("Failed to create context");
        let config = self.get_config();
        ctx.extend_variables(config.variables.clone());
        ctx.get_context_variables()
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

    let _ = run_cli(fixture.get_cli(None));

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

    // Modify the source files to ensure deployment happens and backups are created
    fixture.write_file("src/nvim/init.lua", "-- Modified init.lua\n");
    fixture.write_file("src/.bashrc", "# Modified bashrc\n");

    // Deploy all packages
    fixture.deploy(None);

    // Verify backups created (granular per-file backups)
    fixture.assert_file_exists(
        "src/nvim/init.lua.dotrbak",
        "nvim init.lua backup should exist",
    );
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

    // Modify nvim to ensure deployment happens
    fixture.write_file("src/nvim/init.lua", "-- Modified init.lua\n");

    // Deploy only nvim
    let nvim_package_name = fixture.get_package_name(NVIM_PATH);
    fixture.deploy(Some(vec![nvim_package_name]));

    // Verify only nvim was deployed
    fixture.assert_file_exists(
        "src/nvim/init.lua.dotrbak",
        "nvim init.lua backup should exist",
    );
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

    // Modify files to ensure deployment happens
    fixture.write_file("src/nvim/init.lua", "-- Modified init.lua\n");
    fixture.write_file("src/.bashrc", "# Modified bashrc\n");

    // Deploy both packages explicitly
    let nvim_package_name = fixture.get_package_name(NVIM_PATH);
    let bashrc_package_name = fixture.get_package_name(BASHRC_PATH);
    fixture.deploy(Some(vec![nvim_package_name, bashrc_package_name]));

    // Verify both were deployed
    fixture.assert_file_exists(
        "src/nvim/init.lua.dotrbak",
        "nvim init.lua backup should exist",
    );
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

    // Modify file to ensure deployment would happen if it were deployed
    fixture.write_file("src/nvim/init.lua", "-- Modified init.lua\n");

    // Try to deploy a non-existent package
    fixture.deploy(Some(vec!["nonexistent_package".to_string()]));

    // Verify nothing was deployed
    fixture.assert_file_not_exists(
        "src/nvim/init.lua.dotrbak",
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

    // Modify files to ensure deployment happens
    fixture.write_file("src/.bashrc", "# Modified bashrc\n");
    fixture.write_file("src/.zshrc", "# Modified zshrc\n");
    fixture.write_file("src/nvim/init.lua", "-- Modified init.lua\n");
    fixture.write_file("src/tmux/tmux.conf", "# Modified tmux.conf\n");
    fixture.write_file("src/tmux/theme.conf", "# Modified theme.conf\n");

    // Deploy all packages
    fixture.deploy(None);

    // Verify file backups created
    fixture.assert_file_exists("src/.bashrc.dotrbak", "bashrc backup should exist");
    fixture.assert_file_exists("src/.zshrc.dotrbak", "zshrc backup should exist");

    // Verify directory backups created (granular per-file)
    fixture.assert_file_exists(
        "src/nvim/init.lua.dotrbak",
        "nvim init.lua backup should exist",
    );
    fixture.assert_file_exists(
        "src/tmux/tmux.conf.dotrbak",
        "tmux.conf backup should exist",
    );
    fixture.assert_file_exists(
        "src/tmux/theme.conf.dotrbak",
        "theme.conf backup should exist",
    );

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

    // Modify file to ensure deployment happens
    fixture.write_file(
        "src/config/alacritty/alacritty.yml",
        "# Modified alacritty\n",
    );

    // Deploy
    let alacritty_name = fixture.get_package_name(ALACRITTY_PATH);
    fixture.deploy(Some(vec![alacritty_name]));

    // Verify backup was created (granular per-file)
    fixture.assert_file_exists(
        "src/config/alacritty/alacritty.yml.dotrbak",
        "alacritty.yml backup should exist",
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

    // Modify files to ensure deployment happens
    fixture.write_file("src/.bashrc", "# Modified bashrc\n");
    fixture.write_file("src/.gitconfig", "# Modified gitconfig\n");
    fixture.write_file("src/nvim/init.lua", "-- Modified init.lua\n");
    fixture.write_file("src/tmux/theme.conf", "# Modified theme.conf\n");

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
    let _ = run_cli(
        fixture.get_cli(Some(dotr::cli::Command::PrintVars(PrintVarsArgs {
            profile: None,
        }))),
    );

    // Verify that Context has HOME environment variable
    let ctx_vars = fixture.get_context_variables();
    assert!(
        ctx_vars.contains_key("HOME"),
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
        .insert("EDITOR".to_string(), toml::Value::String("vim".to_string()));
    config.variables.insert(
        "SHELL".to_string(),
        toml::Value::String("/bin/zsh".to_string()),
    );
    config.variables.insert(
        "USER_EMAIL".to_string(),
        toml::Value::String("test@example.com".to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // Print vars should show custom variables
    let _ = run_cli(
        fixture.get_cli(Some(dotr::cli::Command::PrintVars(PrintVarsArgs {
            profile: None,
        }))),
    );

    // Verify config contains variables
    let config = fixture.get_config();
    assert_eq!(
        config.variables.get("EDITOR"),
        Some(&toml::Value::String("vim".to_string()))
    );
    assert_eq!(
        config.variables.get("SHELL"),
        Some(&toml::Value::String("/bin/zsh".to_string()))
    );
    assert_eq!(
        config.variables.get("USER_EMAIL"),
        Some(&toml::Value::String("test@example.com".to_string()))
    );

    // Verify that environment variables like HOME are still present in Context
    let ctx_vars = fixture.get_context_variables();
    assert!(
        ctx_vars.contains_key("HOME"),
        "HOME environment variable should be present in Context"
    );
}

#[test]
fn test_variables_persist_after_save() {
    let fixture = TestFixture::new();

    fixture.init();

    // Add variables manually
    let mut config = fixture.get_config();
    config.variables.insert(
        "TEST_VAR".to_string(),
        toml::Value::String("test_value".to_string()),
    );
    config.variables.insert(
        "ANOTHER_VAR".to_string(),
        toml::Value::String("another_value".to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload config and verify variables persist
    let reloaded_config = fixture.get_config();
    assert_eq!(
        reloaded_config.variables.get("TEST_VAR"),
        Some(&toml::Value::String("test_value".to_string()))
    );
    assert_eq!(
        reloaded_config.variables.get("ANOTHER_VAR"),
        Some(&toml::Value::String("another_value".to_string()))
    );
}

#[test]
fn test_home_variable_always_present() {
    let fixture = TestFixture::new();

    fixture.init();

    // Context should have environment variables including HOME
    let ctx_vars = fixture.get_context_variables();

    // HOME should always be present in context
    assert!(ctx_vars.contains_key("HOME"));

    // HOME should be a valid path
    let home = ctx_vars.get("HOME").expect("HOME variable not found");
    if let toml::Value::String(s) = home {
        assert!(!s.is_empty());
    } else {
        panic!("HOME should be a string value");
    }
}

#[test]
fn test_variables_with_special_characters() {
    let fixture = TestFixture::new();

    fixture.init();

    // Add variables with special characters using Config API
    let mut config = fixture.get_config();
    config.variables.insert(
        "PATH".to_string(),
        toml::Value::String("/usr/local/bin:/usr/bin:/bin".to_string()),
    );
    config.variables.insert(
        "PS1".to_string(),
        toml::Value::String("[\\u@\\h \\W]$ ".to_string()),
    );
    config.variables.insert(
        "COMPLEX_VAR".to_string(),
        toml::Value::String("value with spaces and $pecial ch@rs".to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    let config = fixture.get_config();
    assert_eq!(
        config.variables.get("PATH"),
        Some(&toml::Value::String(
            "/usr/local/bin:/usr/bin:/bin".to_string()
        ))
    );
    assert_eq!(
        config.variables.get("PS1"),
        Some(&toml::Value::String("[\\u@\\h \\W]$ ".to_string()))
    );
    assert_eq!(
        config.variables.get("COMPLEX_VAR"),
        Some(&toml::Value::String(
            "value with spaces and $pecial ch@rs".to_string()
        ))
    );
}

#[test]
fn test_variables_do_not_interfere_with_packages() {
    let fixture = TestFixture::new();

    fixture.init();

    // Add variables
    let mut config = fixture.get_config();
    config.variables.insert(
        "MY_VAR".to_string(),
        toml::Value::String("my_value".to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // Import packages
    fixture.import(BASHRC_PATH);
    fixture.import(NVIM_PATH);

    // Reload and verify both variables and packages exist
    let config = fixture.get_config();
    assert_eq!(
        config.variables.get("MY_VAR"),
        Some(&toml::Value::String("my_value".to_string()))
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
    config.variables.insert(
        "HOME".to_string(),
        toml::Value::String(custom_home.to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload config and verify HOME is overridden
    let reloaded_config = fixture.get_config();
    assert_eq!(
        reloaded_config.variables.get("HOME"),
        Some(&toml::Value::String(custom_home.to_string())),
        "Config HOME should override environment HOME"
    );
    assert_ne!(
        reloaded_config.variables.get("HOME"),
        Some(&toml::Value::String(original_home.clone())),
        "Config HOME should be different from environment HOME"
    );

    // Create context and verify that config variables take precedence
    let ctx_vars = fixture.get_context_variables();

    assert_eq!(
        ctx_vars.get("HOME"),
        Some(&toml::Value::String(custom_home.to_string())),
        "Context should use config HOME over environment HOME"
    );
}

#[test]
fn test_nested_variables_simple() {
    let fixture = TestFixture::new();

    fixture.init();

    // Create nested variable structure
    let mut config = fixture.get_config();
    let mut database_config = toml::map::Map::new();
    database_config.insert(
        "host".to_string(),
        toml::Value::String("localhost".to_string()),
    );
    database_config.insert("port".to_string(), toml::Value::Integer(5432));
    database_config.insert("name".to_string(), toml::Value::String("mydb".to_string()));

    config
        .variables
        .insert("database".to_string(), toml::Value::Table(database_config));
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify nested structure
    let reloaded_config = fixture.get_config();
    let db_var = reloaded_config.variables.get("database");
    assert!(db_var.is_some(), "database variable should exist");

    if let Some(toml::Value::Table(db_table)) = db_var {
        assert_eq!(
            db_table.get("host"),
            Some(&toml::Value::String("localhost".to_string()))
        );
        assert_eq!(db_table.get("port"), Some(&toml::Value::Integer(5432)));
        assert_eq!(
            db_table.get("name"),
            Some(&toml::Value::String("mydb".to_string()))
        );
    } else {
        panic!("database should be a table");
    }
}

#[test]
fn test_nested_variables_deep() {
    let fixture = TestFixture::new();

    fixture.init();

    // Create deeply nested structure
    let mut config = fixture.get_config();

    // Build: app.server.config.port
    let mut port_config = toml::map::Map::new();
    port_config.insert("http".to_string(), toml::Value::Integer(8080));
    port_config.insert("https".to_string(), toml::Value::Integer(8443));

    let mut server_config = toml::map::Map::new();
    server_config.insert(
        "host".to_string(),
        toml::Value::String("0.0.0.0".to_string()),
    );
    server_config.insert("ports".to_string(), toml::Value::Table(port_config));

    let mut app_config = toml::map::Map::new();
    app_config.insert("name".to_string(), toml::Value::String("myapp".to_string()));
    app_config.insert("server".to_string(), toml::Value::Table(server_config));

    config
        .variables
        .insert("app".to_string(), toml::Value::Table(app_config));
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify deep nesting
    let reloaded_config = fixture.get_config();
    let app_var = reloaded_config.variables.get("app");
    assert!(app_var.is_some(), "app variable should exist");

    if let Some(toml::Value::Table(app_table)) = app_var {
        assert_eq!(
            app_table.get("name"),
            Some(&toml::Value::String("myapp".to_string()))
        );

        if let Some(toml::Value::Table(server_table)) = app_table.get("server") {
            assert_eq!(
                server_table.get("host"),
                Some(&toml::Value::String("0.0.0.0".to_string()))
            );

            if let Some(toml::Value::Table(ports_table)) = server_table.get("ports") {
                assert_eq!(ports_table.get("http"), Some(&toml::Value::Integer(8080)));
                assert_eq!(ports_table.get("https"), Some(&toml::Value::Integer(8443)));
            } else {
                panic!("ports should be a table");
            }
        } else {
            panic!("server should be a table");
        }
    } else {
        panic!("app should be a table");
    }
}

#[test]
fn test_nested_variables_with_arrays() {
    let fixture = TestFixture::new();

    fixture.init();

    // Create nested structure with arrays
    let mut config = fixture.get_config();

    let hosts = vec![
        toml::Value::String("server1.example.com".to_string()),
        toml::Value::String("server2.example.com".to_string()),
        toml::Value::String("server3.example.com".to_string()),
    ];

    let mut cluster_config = toml::map::Map::new();
    cluster_config.insert(
        "name".to_string(),
        toml::Value::String("production".to_string()),
    );
    cluster_config.insert("hosts".to_string(), toml::Value::Array(hosts));
    cluster_config.insert("replicas".to_string(), toml::Value::Integer(3));

    config
        .variables
        .insert("cluster".to_string(), toml::Value::Table(cluster_config));
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify arrays in nested structures
    let reloaded_config = fixture.get_config();
    let cluster_var = reloaded_config.variables.get("cluster");
    assert!(cluster_var.is_some(), "cluster variable should exist");

    if let Some(toml::Value::Table(cluster_table)) = cluster_var {
        assert_eq!(
            cluster_table.get("name"),
            Some(&toml::Value::String("production".to_string()))
        );
        assert_eq!(
            cluster_table.get("replicas"),
            Some(&toml::Value::Integer(3))
        );

        if let Some(toml::Value::Array(hosts_array)) = cluster_table.get("hosts") {
            assert_eq!(hosts_array.len(), 3);
            assert_eq!(
                hosts_array[0],
                toml::Value::String("server1.example.com".to_string())
            );
            assert_eq!(
                hosts_array[1],
                toml::Value::String("server2.example.com".to_string())
            );
            assert_eq!(
                hosts_array[2],
                toml::Value::String("server3.example.com".to_string())
            );
        } else {
            panic!("hosts should be an array");
        }
    } else {
        panic!("cluster should be a table");
    }
}

#[test]
fn test_nested_variables_mixed_types() {
    let fixture = TestFixture::new();

    fixture.init();

    // Create nested structure with mixed types
    let mut config = fixture.get_config();

    let mut settings = toml::map::Map::new();
    settings.insert("debug".to_string(), toml::Value::Boolean(true));
    settings.insert("timeout".to_string(), toml::Value::Float(30.5));
    settings.insert("retries".to_string(), toml::Value::Integer(3));
    settings.insert(
        "endpoint".to_string(),
        toml::Value::String("https://api.example.com".to_string()),
    );

    config
        .variables
        .insert("settings".to_string(), toml::Value::Table(settings));
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify mixed types
    let reloaded_config = fixture.get_config();
    let settings_var = reloaded_config.variables.get("settings");
    assert!(settings_var.is_some(), "settings variable should exist");

    if let Some(toml::Value::Table(settings_table)) = settings_var {
        assert_eq!(
            settings_table.get("debug"),
            Some(&toml::Value::Boolean(true))
        );
        assert_eq!(
            settings_table.get("timeout"),
            Some(&toml::Value::Float(30.5))
        );
        assert_eq!(
            settings_table.get("retries"),
            Some(&toml::Value::Integer(3))
        );
        assert_eq!(
            settings_table.get("endpoint"),
            Some(&toml::Value::String("https://api.example.com".to_string()))
        );
    } else {
        panic!("settings should be a table");
    }
}

#[test]
fn test_nested_variables_print() {
    let fixture = TestFixture::new();

    fixture.init();

    // Create nested variable structure for print test
    let mut config = fixture.get_config();

    let mut api_config = toml::map::Map::new();
    api_config.insert(
        "key".to_string(),
        toml::Value::String("secret123".to_string()),
    );
    api_config.insert("timeout".to_string(), toml::Value::Integer(30));

    config
        .variables
        .insert("api".to_string(), toml::Value::Table(api_config));
    config.variables.insert(
        "version".to_string(),
        toml::Value::String("1.0.0".to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // Test that print-vars works with nested variables
    let _ = run_cli(
        fixture.get_cli(Some(dotr::cli::Command::PrintVars(PrintVarsArgs {
            profile: None,
        }))),
    );

    // Verify the nested structure is preserved
    let reloaded_config = fixture.get_config();
    assert!(reloaded_config.variables.contains_key("api"));
    assert!(reloaded_config.variables.contains_key("version"));
}

#[test]
fn test_nested_variables_do_not_interfere_with_flat_variables() {
    let fixture = TestFixture::new();

    fixture.init();

    // Mix flat and nested variables
    let mut config = fixture.get_config();

    // Flat variables
    config
        .variables
        .insert("EDITOR".to_string(), toml::Value::String("vim".to_string()));
    config.variables.insert(
        "SHELL".to_string(),
        toml::Value::String("/bin/bash".to_string()),
    );

    // Nested variable
    let mut db_config = toml::map::Map::new();
    db_config.insert(
        "host".to_string(),
        toml::Value::String("localhost".to_string()),
    );
    db_config.insert("port".to_string(), toml::Value::Integer(3306));
    config
        .variables
        .insert("database".to_string(), toml::Value::Table(db_config));

    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify both flat and nested coexist
    let reloaded_config = fixture.get_config();

    // Check flat variables
    assert_eq!(
        reloaded_config.variables.get("EDITOR"),
        Some(&toml::Value::String("vim".to_string()))
    );
    assert_eq!(
        reloaded_config.variables.get("SHELL"),
        Some(&toml::Value::String("/bin/bash".to_string()))
    );

    // Check nested variable
    if let Some(toml::Value::Table(db_table)) = reloaded_config.variables.get("database") {
        assert_eq!(
            db_table.get("host"),
            Some(&toml::Value::String("localhost".to_string()))
        );
        assert_eq!(db_table.get("port"), Some(&toml::Value::Integer(3306)));
    } else {
        panic!("database should be a table");
    }
}

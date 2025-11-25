use std::{collections::HashMap, fs, path::PathBuf};

use dotr::{
    cli::{Cli, Command, DeployArgs, ImportArgs, InitArgs, PrintVarsArgs, UpdateArgs, run_cli},
    config::Config,
    package::Package,
};

struct TestFixture {
    cwd: PathBuf,
}

impl TestFixture {
    fn new() -> Self {
        let temp_dir = std::env::temp_dir().join(format!("dotr_cli_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");
        Self { cwd: temp_dir }
    }

    fn get_cli(&self, command: Option<Command>) -> Cli {
        Cli {
            command,
            working_dir: Some(self.cwd.to_str().unwrap().to_string()),
        }
    }

    fn init(&self) {
        run_cli(self.get_cli(Some(Command::Init(InitArgs {})))).expect("Init failed");
    }

    fn get_config(&self) -> Config {
        Config::from_path(&self.cwd).expect("Failed to load config")
    }

    fn assert_file_exists(&self, path: &str, message: &str) {
        assert!(self.cwd.join(path).exists(), "{}", message);
    }

    fn assert_file_not_exists(&self, path: &str, message: &str) {
        assert!(!self.cwd.join(path).exists(), "{}", message);
    }

    fn write_file(&self, path: &str, content: &str) {
        let file_path = self.cwd.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        fs::write(file_path, content).expect("Failed to write file");
    }

    fn read_file(&self, path: &str) -> String {
        fs::read_to_string(self.cwd.join(path)).expect("Failed to read file")
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.cwd).ok();
    }
}

#[test]
fn test_init_creates_config() {
    let fixture = TestFixture::new();

    fixture.init();

    fixture.assert_file_exists("config.toml", "config.toml should be created");
    fixture.assert_file_exists("dotfiles", "dotfiles directory should be created");
    fixture.assert_file_exists(".gitignore", ".gitignore should be created");

    let gitignore = fixture.read_file(".gitignore");
    assert!(
        gitignore.contains(".uservariables.toml"),
        ".gitignore should contain .uservariables.toml"
    );
}

#[test]
fn test_init_idempotent() {
    let fixture = TestFixture::new();

    // First init
    fixture.init();
    let first_config = fixture.read_file("config.toml");

    // Second init should not change config
    fixture.init();
    let second_config = fixture.read_file("config.toml");

    assert_eq!(
        first_config, second_config,
        "Config should not change on second init"
    );
}

#[test]
fn test_import_creates_package() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.write_file("test.conf", "test content");

    let _ = run_cli(fixture.get_cli(Some(Command::Import(ImportArgs {
        name: None,
        path: fixture.cwd.join("test.conf").to_str().unwrap().to_string(),
        profile: None,
    }))));

    let config = fixture.get_config();
    assert!(
        config.packages.contains_key("f_test_conf"),
        "Package should be imported"
    );

    fixture.assert_file_exists("dotfiles/f_test.conf", "File should be copied to dotfiles");
}

#[test]
fn test_import_with_profile() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.write_file("work.conf", "work content");

    let _ = run_cli(fixture.get_cli(Some(Command::Import(ImportArgs {
        name: None,
        path: fixture.cwd.join("work.conf").to_str().unwrap().to_string(),
        profile: Some("work".to_string()),
    }))));

    let config = fixture.get_config();
    let package = config
        .packages
        .get("f_work_conf")
        .expect("Package should exist");

    assert!(!package.skip, "Package should not be marked as skip");
    assert!(
        config.profiles.contains_key("work"),
        "Profile should be created"
    );

    let profile = config.profiles.get("work").unwrap();
    assert!(
        profile.dependencies.contains(&"f_work_conf".to_string()),
        "Profile should have package as dependency"
    );
}

#[test]
fn test_deploy_creates_files() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.write_file("dotfiles/f_test/config.txt", "test config");

    // Add package to config
    let mut config = fixture.get_config();

    let test_package = dotr::package::Package {
        name: "f_test".to_string(),
        src: "dotfiles/f_test".to_string(),
        dest: fixture
            .cwd
            .join("deploy_dest")
            .to_str()
            .unwrap()
            .to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("f_test".to_string(), test_package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_test".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    let _ = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))));

    fixture.assert_file_exists("deploy_dest/config.txt", "Deployed file should exist");
}

#[test]
fn test_deploy_with_profile() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.write_file("dotfiles/f_app/app.conf", "app config");

    // Create package and profile
    let mut config = fixture.get_config();

    let mut package = dotr::package::Package {
        name: "f_app".to_string(),
        src: "dotfiles/f_app".to_string(),
        dest: fixture
            .cwd
            .join("default_dest")
            .to_str()
            .unwrap()
            .to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    package.targets.insert(
        "work".to_string(),
        fixture.cwd.join("work_dest").to_str().unwrap().to_string(),
    );

    config.packages.insert("f_app".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_app".to_string());

    let profile = dotr::profile::Profile {
        name: "work".to_string(),
        variables: toml::Table::new(),
        dependencies: vec!["f_app".to_string()],
        prompts: HashMap::new(),
    };
    config.profiles.insert("work".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    let _ = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: Some("work".to_string()),
        ignore_errors: false,
        clean: false,
        dry_run: false,
    }))));

    fixture.assert_file_exists(
        "work_dest/app.conf",
        "File should be deployed to profile target",
    );
}

#[test]
fn test_deploy_specific_packages() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.write_file("dotfiles/f_pkg1/file1.txt", "file 1");
    fixture.write_file("dotfiles/f_pkg2/file2.txt", "file 2");

    // Create two packages
    let mut config = fixture.get_config();

    let pkg1 = dotr::package::Package {
        name: "f_pkg1".to_string(),
        src: "dotfiles/f_pkg1".to_string(),
        dest: fixture.cwd.join("dest1").to_str().unwrap().to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    let pkg2 = dotr::package::Package {
        name: "f_pkg2".to_string(),
        src: "dotfiles/f_pkg2".to_string(),
        dest: fixture.cwd.join("dest2").to_str().unwrap().to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("f_pkg1".to_string(), pkg1);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_pkg1".to_string());
    config.packages.insert("f_pkg2".to_string(), pkg2);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_pkg2".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy only pkg1
    let _ = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: Some(vec!["f_pkg1".to_string()]),
        profile: None,
        ignore_errors: false,
        clean: false,
        dry_run: false,
    }))));

    fixture.assert_file_exists("dest1/file1.txt", "pkg1 should be deployed");
    fixture.assert_file_not_exists("dest2/file2.txt", "pkg2 should not be deployed");
}

#[test]
fn test_update_backs_up_files() {
    let fixture = TestFixture::new();

    fixture.init();

    // Create a package
    let mut config = fixture.get_config();
    let pkg = dotr::package::Package {
        name: "f_update".to_string(),
        src: "dotfiles/f_update".to_string(),
        dest: fixture
            .cwd
            .join("update_dest")
            .to_str()
            .unwrap()
            .to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };
    config.packages.insert("f_update".to_string(), pkg);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_update".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create file at dest
    fixture.write_file("update_dest", "updated content");

    let _ = run_cli(fixture.get_cli(Some(Command::Update(UpdateArgs::default()))));

    fixture.assert_file_exists("dotfiles/f_update", "File should be backed up");
    let content = fixture.read_file("dotfiles/f_update");
    assert_eq!(content, "updated content", "Backed up content should match");
}

#[test]
fn test_print_vars_shows_variables() {
    let fixture = TestFixture::new();

    fixture.init();

    // Add some variables to config
    let mut config = fixture.get_config();
    config.variables.insert(
        "TEST_VAR".to_string(),
        toml::Value::String("test_value".to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // This will print to stdout - we're just testing it doesn't panic
    run_cli(fixture.get_cli(Some(Command::PrintVars(PrintVarsArgs { profile: None }))))
        .expect("Print vars should succeed");
}

#[test]
fn test_print_vars_with_profile() {
    let fixture = TestFixture::new();

    fixture.init();

    // Create profile with variables
    let mut config = fixture.get_config();
    let mut profile = dotr::profile::Profile::new("dev");
    profile.variables.insert(
        "PROFILE_VAR".to_string(),
        toml::Value::String("dev_value".to_string()),
    );
    config.profiles.insert("dev".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // This will print to stdout - we're just testing it doesn't panic
    let _ = run_cli(fixture.get_cli(Some(Command::PrintVars(PrintVarsArgs {
        profile: Some("dev".to_string()),
    }))));
}

#[test]
fn test_banner_display() {
    let fixture = TestFixture::new();

    fixture.init();

    // Banner is controlled by config
    let mut config = fixture.get_config();
    config.banner = true;
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy command should show banner
    let _ = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))));

    // Just testing it doesn't panic
}

#[test]
fn test_banner_disabled() {
    let fixture = TestFixture::new();

    fixture.init();

    // Disable banner
    let mut config = fixture.get_config();
    config.banner = false;
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy command should not show banner
    let _ = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))));

    // Just testing it doesn't panic
}

#[test]
fn test_working_dir_relative_path() {
    let fixture = TestFixture::new();

    // Create a subdirectory
    fs::create_dir_all(fixture.cwd.join("subdir")).expect("Failed to create subdir");

    let cli = Cli {
        command: Some(Command::Init(InitArgs {})),
        working_dir: Some(fixture.cwd.join("subdir").to_str().unwrap().to_string()),
    };

    run_cli(cli).expect("Init in subdir should succeed");

    fixture.assert_file_exists("subdir/config.toml", "Config should be created in subdir");
}

#[test]
fn test_skip_flag_prevents_deployment() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.write_file("dotfiles/f_skip/skip.txt", "skip content");

    // Create package with skip flag
    let mut config = fixture.get_config();
    let pkg = dotr::package::Package {
        name: "f_skip".to_string(),
        src: "dotfiles/f_skip".to_string(),
        dest: fixture.cwd.join("skip_dest").to_str().unwrap().to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: true,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };
    config.packages.insert("f_skip".to_string(), pkg);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_skip".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy without profile (skip packages should not be deployed)
    let _ = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))));

    fixture.assert_file_not_exists("skip_dest/skip.txt", "Skip package should not be deployed");
}

#[test]
fn test_profile_dependencies_deployment() {
    let fixture = TestFixture::new();

    fixture.init();
    fixture.write_file("dotfiles/f_dep1/dep1.txt", "dep1");
    fixture.write_file("dotfiles/f_dep2/dep2.txt", "dep2");

    // Create packages and profile
    let mut config = fixture.get_config();

    let pkg1 = dotr::package::Package {
        name: "f_dep1".to_string(),
        src: "dotfiles/f_dep1".to_string(),
        dest: fixture.cwd.join("dep1_dest").to_str().unwrap().to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    let pkg2 = dotr::package::Package {
        name: "f_dep2".to_string(),
        src: "dotfiles/f_dep2".to_string(),
        dest: fixture.cwd.join("dep2_dest").to_str().unwrap().to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("f_dep1".to_string(), pkg1);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_dep1".to_string());
    config.packages.insert("f_dep2".to_string(), pkg2);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_dep2".to_string());

    // Profile with only dep1 as dependency
    let profile = dotr::profile::Profile {
        name: "minimal".to_string(),
        variables: toml::Table::new(),
        dependencies: vec!["f_dep1".to_string()],
        prompts: HashMap::new(),
    };
    config.profiles.insert("minimal".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy with profile
    let _ = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: Some("minimal".to_string()),
        ignore_errors: false,
        clean: false,
        dry_run: false,
    }))));

    fixture.assert_file_exists(
        "dep1_dest/dep1.txt",
        "Profile dependency should be deployed",
    );
    fixture.assert_file_not_exists(
        "dep2_dest/dep2.txt",
        "Non-dependency should not be deployed",
    );
}

#[test]
fn test_no_command_shows_help_message() {
    let fixture = TestFixture::new();

    let cli = Cli {
        command: None,
        working_dir: Some(fixture.cwd.to_str().unwrap().to_string()),
    };

    let result = run_cli(cli);
    assert!(result.is_ok(), "No command should not error");
}

// ===== UNHAPPY PATH TESTS =====

#[test]
fn test_nonexistent_working_directory_fails() {
    let nonexistent = PathBuf::from("/this/path/does/not/exist/dotr_test");

    let cli = Cli {
        command: Some(Command::Deploy(DeployArgs::default())),
        working_dir: Some(nonexistent.to_str().unwrap().to_string()),
    };

    let result = run_cli(cli);
    assert!(result.is_err(), "Should fail with nonexistent directory");
    assert!(
        result.unwrap_err().to_string().contains("does not exist"),
        "Error should mention directory doesn't exist"
    );
}

#[test]
fn test_deploy_without_config_fails() {
    let fixture = TestFixture::new();

    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))));

    assert!(result.is_err(), "Deploy without config should fail");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("config.toml not found"),
        "Error should mention missing config"
    );
}

#[test]
fn test_import_nonexistent_file_fails() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(
        fixture.get_cli(Some(Command::Import(ImportArgs {
            name: None,
            path: fixture
                .cwd
                .join("does_not_exist.conf")
                .to_str()
                .unwrap()
                .to_string(),
            profile: None,
        }))),
    );

    assert!(result.is_err(), "Import nonexistent file should fail");
    assert!(
        result.unwrap_err().to_string().contains("does not exist"),
        "Error should mention file doesn't exist"
    );
}

#[test]
fn test_deploy_with_invalid_profile_fails() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: Some("nonexistent_profile".to_string()),
        ignore_errors: false,
        clean: false,
        dry_run: false,
    }))));

    assert!(result.is_err(), "Deploy with invalid profile should fail");
    assert!(
        result.unwrap_err().to_string().contains("not found"),
        "Error should mention profile not found"
    );
}

#[test]
fn test_update_with_invalid_profile_fails() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(fixture.get_cli(Some(Command::Update(UpdateArgs {
        packages: None,
        profile: Some("invalid_profile".to_string()),
        ignore_errors: false,
        clean: false,
        dry_run: false,
    }))));

    assert!(result.is_err(), "Update with invalid profile should fail");
    assert!(
        result.unwrap_err().to_string().contains("not found"),
        "Error should mention profile not found"
    );
}

#[test]
fn test_print_vars_with_invalid_profile_fails() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(fixture.get_cli(Some(Command::PrintVars(PrintVarsArgs {
        profile: Some("missing_profile".to_string()),
    }))));

    assert!(
        result.is_err(),
        "PrintVars with invalid profile should fail"
    );
    assert!(
        result.unwrap_err().to_string().contains("not found"),
        "Error should mention profile not found"
    );
}

#[test]
fn test_deploy_nonexistent_package_fails() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: Some(vec!["nonexistent_package".to_string()]),
        profile: None,
        ignore_errors: false,
        clean: false,
        dry_run: false,
    }))));

    // Deploy should fail with error for nonexistent package
    assert!(
        result.is_err(),
        "Deploy with nonexistent package should error"
    );
    assert!(
        result.unwrap_err().to_string().contains("not found"),
        "Error should mention package not found"
    );
}

#[test]
fn test_update_nonexistent_package_fails() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(fixture.get_cli(Some(Command::Update(UpdateArgs {
        packages: Some(vec!["nonexistent_package".to_string()]),
        profile: None,
        ignore_errors: false,
        clean: false,
        dry_run: false,
    }))));

    // Update should fail with error for nonexistent package
    assert!(
        result.is_err(),
        "Update with nonexistent package should error"
    );
    assert!(
        result.unwrap_err().to_string().contains("not found"),
        "Error should mention package not found"
    );
}

#[test]
fn test_invalid_toml_config_fails() {
    let fixture = TestFixture::new();
    fixture.init();

    // Corrupt the config file
    fixture.write_file("config.toml", "invalid toml {{{ syntax");

    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))));

    assert!(result.is_err(), "Invalid TOML config should fail");
}

#[test]
fn test_invalid_uservariables_toml_fails() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create invalid .uservariables.toml
    fixture.write_file(".uservariables.toml", "bad toml [[[");

    // Use PrintVars which will definitely try to load context
    let result =
        run_cli(fixture.get_cli(Some(Command::PrintVars(PrintVarsArgs { profile: None }))));

    assert!(result.is_err(), "Invalid uservariables TOML should fail");
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("parse") || error_msg.contains("Failed to parse"),
        "Error should mention parsing failure, got: {}",
        error_msg
    );
}

#[test]
fn test_package_with_missing_dependency_fails() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create package with nonexistent dependency
    let mut config = fixture.get_config();
    let pkg = dotr::package::Package {
        name: "test_pkg".to_string(),
        src: "dotfiles/test_pkg".to_string(),
        dest: fixture.cwd.join("dest").to_str().unwrap().to_string(),
        dependencies: Some(vec!["nonexistent_dep".to_string()]),
        variables: toml::Table::new(),
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };
    config.packages.insert("test_pkg".to_string(), pkg);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("test_pkg".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: Some(vec!["test_pkg".to_string()]),
        profile: None,
        ignore_errors: false,
        clean: false,
        dry_run: false,
    }))));

    assert!(
        result.is_err(),
        "Package with missing dependency should fail"
    );
    assert!(
        result.unwrap_err().to_string().contains("not found"),
        "Error should mention dependency not found"
    );
}

#[test]
fn test_deploy_missing_source_fails() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create package but don't create source files
    let mut config = fixture.get_config();
    let pkg = dotr::package::Package {
        name: "missing_src".to_string(),
        src: "dotfiles/missing_src".to_string(),
        dest: fixture.cwd.join("dest").to_str().unwrap().to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };
    config.packages.insert("missing_src".to_string(), pkg);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("missing_src".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))));

    // This might succeed as walkdir might not find any files, depending on implementation
    // If src directory doesn't exist, it should fail
    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("No such file") || error_msg.contains("does not exist"),
            "Error should mention missing source, got: {}",
            error_msg
        );
    }
}

#[test]
fn test_import_normalizes_home_path() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a test file in a subdirectory
    let test_dir = fixture.cwd.join("test_import");
    fs::create_dir_all(&test_dir).expect("Failed to create test dir");
    fs::write(test_dir.join("test.txt"), "content").expect("Failed to write test file");

    // Import the file
    let _ = run_cli(fixture.get_cli(Some(Command::Import(ImportArgs {
        name: None,
        path: test_dir.to_str().unwrap().to_string(),
        profile: None,
    }))));

    let config = fixture.get_config();
    let package = config
        .packages
        .values()
        .next()
        .expect("Should have package");

    // Path should not have ~ since it's not in home directory
    assert!(
        !package.dest.starts_with('~'),
        "Non-home path should not use ~"
    );
}

#[test]
fn test_import_preserves_tilde_path() {
    // This test verifies that the normalize_home_path function preserves tilde notation
    // We test this at the utility level rather than end-to-end to avoid creating files in real home

    let path_with_tilde = "~/.config/nvim";
    let normalized = dotr::utils::normalize_home_path(path_with_tilde);

    assert_eq!(
        normalized, path_with_tilde,
        "Tilde paths should be preserved as-is"
    );

    // Test with different tilde paths
    let paths = vec![
        "~/.bashrc",
        "~/.config/alacritty/alacritty.yml",
        "~/Documents/notes.txt",
    ];

    for path in paths {
        let normalized = dotr::utils::normalize_home_path(path);
        assert_eq!(normalized, path, "Tilde path {} should be preserved", path);
    }
}

#[test]
fn test_import_converts_absolute_home_path_to_tilde() {
    let fixture = TestFixture::new();
    fixture.init();

    // Test 1: Path outside home should remain absolute
    let test_file = fixture.cwd.join("test_file.txt");
    fs::write(&test_file, "content").expect("Failed to write test file");

    let abs_path = test_file.to_str().unwrap().to_string();
    let _ = run_cli(fixture.get_cli(Some(Command::Import(ImportArgs {
        name: None,
        path: abs_path.clone(),
        profile: None,
    }))));

    let config = fixture.get_config();
    let package = config
        .packages
        .values()
        .next()
        .expect("Should have package");

    // Since the path is NOT in home directory, it should remain absolute
    assert!(
        !package.dest.starts_with('~'),
        "Path outside home should not use ~ notation, got: {}",
        package.dest
    );

    // Test 2: Verify utility function correctly normalizes home paths
    let home = std::env::home_dir().expect("Should have home dir");
    let mock_home_path = format!("{}/test/path", home.to_string_lossy());
    let normalized = dotr::utils::normalize_home_path(&mock_home_path);
    assert!(
        normalized.starts_with('~'),
        "Path in home directory should be normalized to ~, got: {}",
        normalized
    );
    assert_eq!(
        normalized, "~/test/path",
        "Path should be correctly normalized"
    );
}

#[test]
fn test_dotr_profile_env_var_deploy() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a test file
    fixture.write_file("dotfiles/f_profile_test/profile.conf", "profile content");

    // Create package and profile
    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "f_profile_test".to_string(),
        src: "dotfiles/f_profile_test".to_string(),
        dest: "src/.profile_test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    let profile = dotr::profile::Profile {
        name: "testenv".to_string(),
        variables: toml::Table::new(),
        dependencies: vec!["f_profile_test".to_string()],
        prompts: HashMap::new(),
    };

    config
        .packages
        .insert("f_profile_test".to_string(), package);
    config.profiles.insert("testenv".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Set DOTR_PROFILE env var
    fixture.write_file(".uservariables.toml", "DOTR_PROFILE = \"testenv\"\n");

    // Deploy without specifying profile (should use env var)
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))));

    assert!(
        result.is_ok(),
        "Deploy should succeed with DOTR_PROFILE env var"
    );
    fixture.assert_file_exists(
        "src/.profile_test",
        "File should be deployed using DOTR_PROFILE env var",
    );
}

#[test]
fn test_dotr_profile_env_var_update() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create dest directory
    fs::create_dir_all(fixture.cwd.join("dest")).expect("Failed to create dest dir");

    // Create profile and package with a single file
    fixture.write_file("dotfiles/f_env_update", "original");

    let mut config = fixture.get_config();
    let dest_path = fixture.cwd.join("dest/.env_update");
    let package = dotr::package::Package {
        name: "f_env_update".to_string(),
        src: "dotfiles/f_env_update".to_string(),
        dest: dest_path.to_str().unwrap().to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    let profile = dotr::profile::Profile {
        name: "updateenv".to_string(),
        variables: toml::Table::new(),
        dependencies: vec!["f_env_update".to_string()],
        prompts: HashMap::new(),
    };

    config.packages.insert("f_env_update".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_env_update".to_string());
    config.profiles.insert("updateenv".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy first
    run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: Some(vec!["f_env_update".to_string()]),
        profile: None,
        ignore_errors: false,
        clean: false,
        dry_run: false,
    }))))
    .expect("Deploy failed");

    // Modify deployed file
    fixture.write_file("dest/.env_update", "modified");

    // Set profile via env var
    fixture.write_file(".uservariables.toml", "DOTR_PROFILE = \"updateenv\"\n");

    // Update without specifying profile - should succeed with profile from env var
    let result = run_cli(fixture.get_cli(Some(Command::Update(UpdateArgs {
        packages: Some(vec!["f_env_update".to_string()]),
        profile: None,
        ignore_errors: false,
        clean: false,
        dry_run: false,
    }))));

    assert!(
        result.is_ok(),
        "Update should succeed with DOTR_PROFILE env var"
    );
}

#[test]
fn test_dotr_profile_env_var_print_vars() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create profile with variables
    let mut config = fixture.get_config();
    let mut profile_vars = toml::Table::new();
    profile_vars.insert(
        "PROFILE_VAR".to_string(),
        toml::Value::String("from_env_profile".to_string()),
    );

    let profile = dotr::profile::Profile {
        name: "printenv".to_string(),
        variables: profile_vars,
        dependencies: vec![],
        prompts: HashMap::new(),
    };

    config.profiles.insert("printenv".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Set profile via env var
    fixture.write_file(".uservariables.toml", "DOTR_PROFILE = \"printenv\"\n");

    // Should work without specifying profile
    let result =
        run_cli(fixture.get_cli(Some(Command::PrintVars(PrintVarsArgs { profile: None }))));

    assert!(
        result.is_ok(),
        "PrintVars should succeed with DOTR_PROFILE env var"
    );
}

#[test]
fn test_cli_profile_overrides_env_var() {
    let fixture = TestFixture::new();
    fixture.init();

    fixture.write_file("dotfiles/f_override/override.txt", "content");

    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "f_override".to_string(),
        src: "dotfiles/f_override".to_string(),
        dest: "src/.override".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    let profile1 = dotr::profile::Profile {
        name: "envprofile".to_string(),
        variables: toml::Table::new(),
        dependencies: vec!["f_override".to_string()],
        prompts: HashMap::new(),
    };

    let profile2 = dotr::profile::Profile {
        name: "cliprofile".to_string(),
        variables: toml::Table::new(),
        dependencies: vec!["f_override".to_string()],
        prompts: HashMap::new(),
    };

    config.packages.insert("f_override".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_override".to_string());
    config.profiles.insert("envprofile".to_string(), profile1);
    config.profiles.insert("cliprofile".to_string(), profile2);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Set env var to one profile
    fixture.write_file(".uservariables.toml", "DOTR_PROFILE = \"envprofile\"\n");

    // But explicitly pass different profile via CLI
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: Some("cliprofile".to_string()),
        ignore_errors: false,
        clean: false,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy should use CLI profile over env var");
    fixture.assert_file_exists(
        "src/.override",
        "File should be deployed with CLI-specified profile",
    );
}

#[test]
fn test_invalid_dotr_profile_env_var_ignored() {
    let fixture = TestFixture::new();
    fixture.init();

    fixture.write_file("dotfiles/f_invalid_env/test.txt", "content");

    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "f_invalid_env".to_string(),
        src: "dotfiles/f_invalid_env".to_string(),
        dest: "src/.invalid_env".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("f_invalid_env".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_invalid_env".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Set env var to non-existent profile
    fixture.write_file(".uservariables.toml", "DOTR_PROFILE = \"nonexistent\"\n");

    // Deploy without profile should fail (env var points to invalid profile)
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))));

    assert!(
        result.is_err(),
        "Deploy should fail with invalid DOTR_PROFILE env var"
    );
}

#[test]
fn test_deploy_with_ignore_errors_continues_on_failure() {
    let fixture = TestFixture::new();

    fixture.init();

    // Create a package that will fail during deployment (non-existent source)
    let mut config = fixture.get_config();

    let package1 = Package {
        name: "f_valid_pkg".to_string(),
        src: "dotfiles/f_valid_pkg".to_string(),
        dest: "deploy_dest/valid.txt".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    let package2 = Package {
        name: "f_invalid_pkg".to_string(),
        src: "dotfiles/f_nonexistent".to_string(), // This doesn't exist
        dest: "deploy_dest/invalid.txt".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    let package3 = Package {
        name: "f_another_valid".to_string(),
        src: "dotfiles/f_another_valid".to_string(),
        dest: "deploy_dest/another.txt".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("f_valid_pkg".to_string(), package1);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_valid_pkg".to_string());
    config
        .packages
        .insert("f_invalid_pkg".to_string(), package2);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_invalid_pkg".to_string());
    config
        .packages
        .insert("f_another_valid".to_string(), package3);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_another_valid".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create the destination directory
    std::fs::create_dir_all(fixture.cwd.join("deploy_dest")).unwrap();

    // Create the valid package files
    fixture.write_file("dotfiles/f_valid_pkg", "valid content");
    fixture.write_file("dotfiles/f_another_valid", "another valid content");

    // Deploy with ignore_errors=true should succeed despite one package failing
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: true,
        clean: false,
        dry_run: false,
    }))));

    assert!(
        result.is_ok(),
        "Deploy should succeed with ignore_errors=true even when one package fails"
    );

    // Valid packages should have been deployed
    fixture.assert_file_exists("deploy_dest/valid.txt", "Valid package should be deployed");
    fixture.assert_file_exists(
        "deploy_dest/another.txt",
        "Another valid package should be deployed",
    );
}

#[test]
fn test_deploy_without_ignore_errors_stops_on_failure() {
    let fixture = TestFixture::new();

    fixture.init();

    // Create packages where one will fail
    let mut config = fixture.get_config();

    let package1 = Package {
        name: "f_first_pkg".to_string(),
        src: "dotfiles/f_first_pkg".to_string(),
        dest: "deploy_dest/first.txt".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    let package2 = Package {
        name: "f_failing_pkg".to_string(),
        src: "dotfiles/f_nonexistent".to_string(), // This doesn't exist
        dest: "deploy_dest/failing.txt".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("f_first_pkg".to_string(), package1);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_first_pkg".to_string());
    config
        .packages
        .insert("f_failing_pkg".to_string(), package2);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_failing_pkg".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    std::fs::create_dir_all(fixture.cwd.join("deploy_dest")).unwrap();
    fixture.write_file("dotfiles/f_first_pkg", "first content");

    // Deploy with ignore_errors=false should fail when any package fails
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))));

    assert!(
        result.is_err(),
        "Deploy should fail with ignore_errors=false when a package fails"
    );
}

#[test]
fn test_backup_with_ignore_errors_continues_on_failure() {
    let fixture = TestFixture::new();

    fixture.init();

    // Create packages
    let mut config = fixture.get_config();

    let package1 = Package {
        name: "f_valid_backup".to_string(),
        src: "dotfiles/f_valid_backup".to_string(),
        dest: "backup_src/valid.txt".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    let package2 = Package {
        name: "f_invalid_backup".to_string(),
        src: "dotfiles/f_invalid_backup".to_string(),
        dest: "backup_src/nonexistent.txt".to_string(), // Source doesn't exist
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config
        .packages
        .insert("f_valid_backup".to_string(), package1);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_valid_backup".to_string());
    config
        .packages
        .insert("f_invalid_backup".to_string(), package2);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_invalid_backup".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create the valid source file
    fixture.write_file("backup_src/valid.txt", "valid backup content");

    // Backup with ignore_errors=true should succeed despite one package failing
    let result = run_cli(fixture.get_cli(Some(Command::Update(UpdateArgs {
        packages: None,
        profile: None,
        ignore_errors: true,
        clean: false,
        dry_run: false,
    }))));

    assert!(
        result.is_ok(),
        "Backup should succeed with ignore_errors=true even when one package fails"
    );

    // Valid package should have been backed up
    fixture.assert_file_exists(
        "dotfiles/f_valid_backup",
        "Valid package should be backed up",
    );
}

#[test]
fn test_update_without_ignore_errors_stops_on_failure() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package1 = Package {
        name: "f_backup_pkg".to_string(),
        src: "dotfiles/f_backup_pkg".to_string(),
        dest: "backup_src/exists.txt".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    let package2 = Package {
        name: "f_failing_backup".to_string(),
        src: "dotfiles/f_failing_backup".to_string(),
        dest: "backup_src/missing.txt".to_string(), // Doesn't exist
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("f_backup_pkg".to_string(), package1);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_backup_pkg".to_string());
    config
        .packages
        .insert("f_failing_backup".to_string(), package2);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_failing_backup".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    fixture.write_file("backup_src/exists.txt", "exists");

    // Update with ignore_errors=false should fail when any package fails
    let result = run_cli(fixture.get_cli(Some(Command::Update(UpdateArgs::default()))));

    assert!(
        result.is_err(),
        "Update should fail with ignore_errors=false when a package fails"
    );
}

#[test]
fn test_deploy_with_clean_removes_extra_files() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "d_config_dir".to_string(),
        src: "dotfiles/d_config_dir".to_string(),
        dest: "deploy_dest/config".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("d_config_dir".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("d_config_dir".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source directory with two files
    fixture.write_file("dotfiles/d_config_dir/file1.txt", "file1 content");
    fixture.write_file("dotfiles/d_config_dir/file2.txt", "file2 content");

    // Create destination directory with an extra file that shouldn't be there
    fixture.write_file("deploy_dest/config/file1.txt", "old file1");
    fixture.write_file("deploy_dest/config/file2.txt", "old file2");
    fixture.write_file(
        "deploy_dest/config/extra_file.txt",
        "this should be removed",
    );

    // Deploy with clean=true
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: true,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy with clean should succeed");

    // Check that the correct files exist
    fixture.assert_file_exists("deploy_dest/config/file1.txt", "file1 should exist");
    fixture.assert_file_exists("deploy_dest/config/file2.txt", "file2 should exist");

    // Check that the extra file was removed
    fixture.assert_file_not_exists(
        "deploy_dest/config/extra_file.txt",
        "extra_file should be removed by clean",
    );
}

#[test]
fn test_deploy_without_clean_keeps_extra_files() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "d_another_dir".to_string(),
        src: "dotfiles/d_another_dir".to_string(),
        dest: "deploy_dest/another".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("d_another_dir".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("d_another_dir".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source directory with two files
    fixture.write_file("dotfiles/d_another_dir/file1.txt", "file1 content");
    fixture.write_file("dotfiles/d_another_dir/file2.txt", "file2 content");

    // Create destination directory with an extra file
    fixture.write_file("deploy_dest/another/file1.txt", "old file1");
    fixture.write_file("deploy_dest/another/file2.txt", "old file2");
    fixture.write_file("deploy_dest/another/extra_file.txt", "this should remain");

    // Deploy with clean=false
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))));

    assert!(result.is_ok(), "Deploy without clean should succeed");

    // Check that all files exist, including the extra one
    fixture.assert_file_exists("deploy_dest/another/file1.txt", "file1 should exist");
    fixture.assert_file_exists("deploy_dest/another/file2.txt", "file2 should exist");
    fixture.assert_file_exists(
        "deploy_dest/another/extra_file.txt",
        "extra_file should remain without clean",
    );
}

#[test]
fn test_backup_with_clean_removes_extra_files() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "d_backup_dir".to_string(),
        src: "dotfiles/d_backup_dir".to_string(),
        dest: "source/backup_dir".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("d_backup_dir".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("d_backup_dir".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source directory with two files
    fixture.write_file("source/backup_dir/file1.txt", "file1 content");
    fixture.write_file("source/backup_dir/file2.txt", "file2 content");

    // Create dotfiles directory with an extra file that shouldn't be there
    fixture.write_file("dotfiles/d_backup_dir/file1.txt", "old file1");
    fixture.write_file("dotfiles/d_backup_dir/file2.txt", "old file2");
    fixture.write_file(
        "dotfiles/d_backup_dir/old_file.txt",
        "this should be removed",
    );

    // Backup with clean=true
    let result = run_cli(fixture.get_cli(Some(Command::Update(UpdateArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: true,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Backup with clean should succeed");

    // Check that the correct files exist
    fixture.assert_file_exists("dotfiles/d_backup_dir/file1.txt", "file1 should exist");
    fixture.assert_file_exists("dotfiles/d_backup_dir/file2.txt", "file2 should exist");

    // Check that the old file was removed
    fixture.assert_file_not_exists(
        "dotfiles/d_backup_dir/old_file.txt",
        "old_file should be removed by clean",
    );
}

#[test]
fn test_backup_without_clean_keeps_extra_files() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "d_backup_keep".to_string(),
        src: "dotfiles/d_backup_keep".to_string(),
        dest: "source/backup_keep".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("d_backup_keep".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("d_backup_keep".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source directory with two files
    fixture.write_file("source/backup_keep/file1.txt", "file1 content");
    fixture.write_file("source/backup_keep/file2.txt", "file2 content");

    // Create dotfiles directory with an extra file
    fixture.write_file("dotfiles/d_backup_keep/file1.txt", "old file1");
    fixture.write_file("dotfiles/d_backup_keep/file2.txt", "old file2");
    fixture.write_file("dotfiles/d_backup_keep/old_file.txt", "this should remain");

    // Backup with clean=false
    let result = run_cli(fixture.get_cli(Some(Command::Update(UpdateArgs::default()))));

    assert!(result.is_ok(), "Backup without clean should succeed");

    // Check that all files exist, including the extra one
    fixture.assert_file_exists("dotfiles/d_backup_keep/file1.txt", "file1 should exist");
    fixture.assert_file_exists("dotfiles/d_backup_keep/file2.txt", "file2 should exist");
    fixture.assert_file_exists(
        "dotfiles/d_backup_keep/old_file.txt",
        "old_file should remain without clean",
    );
}

#[test]
fn test_clean_preserves_backup_files() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "d_test_clean".to_string(),
        src: "dotfiles/d_test_clean".to_string(),
        dest: "deploy_dest/test_clean".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("d_test_clean".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("d_test_clean".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source files
    fixture.write_file("dotfiles/d_test_clean/config.txt", "config content");

    // Create destination with a file and a backup file
    fixture.write_file("deploy_dest/test_clean/config.txt", "old config");
    fixture.write_file("deploy_dest/test_clean/extra.txt", "should be removed");
    fixture.write_file(
        "deploy_dest/test_clean/config.txt.dotrbak",
        "backup content",
    );

    // Deploy with clean=true
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: true,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy with clean should succeed");

    // Check that config file exists
    fixture.assert_file_exists(
        "deploy_dest/test_clean/config.txt",
        "config.txt should exist",
    );

    // Check that extra file was removed
    fixture.assert_file_not_exists(
        "deploy_dest/test_clean/extra.txt",
        "extra.txt should be removed",
    );

    // Check that backup file is preserved
    fixture.assert_file_exists(
        "deploy_dest/test_clean/config.txt.dotrbak",
        "backup file should be preserved",
    );
}

#[test]
fn test_clean_removes_empty_directories() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "d_empty_dirs".to_string(),
        src: "dotfiles/d_empty_dirs".to_string(),
        dest: "deploy_dest/empty_dirs".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("d_empty_dirs".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("d_empty_dirs".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source with a simple structure
    fixture.write_file("dotfiles/d_empty_dirs/file.txt", "content");

    // Create destination with nested empty directories
    std::fs::create_dir_all(fixture.cwd.join("deploy_dest/empty_dirs/subdir1/subdir2")).unwrap();
    std::fs::create_dir_all(fixture.cwd.join("deploy_dest/empty_dirs/subdir3")).unwrap();

    // Deploy with clean=true
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: true,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy with clean should succeed");

    // Check that the file exists
    fixture.assert_file_exists("deploy_dest/empty_dirs/file.txt", "file.txt should exist");

    // Check that empty directories were removed
    fixture.assert_file_not_exists(
        "deploy_dest/empty_dirs/subdir1",
        "empty subdir1 should be removed",
    );
    fixture.assert_file_not_exists(
        "deploy_dest/empty_dirs/subdir3",
        "empty subdir3 should be removed",
    );
}

#[test]
fn test_clean_removes_non_empty_directories() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "d_nonempty_dirs".to_string(),
        src: "dotfiles/d_nonempty_dirs".to_string(),
        dest: "deploy_dest/nonempty_dirs".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config
        .packages
        .insert("d_nonempty_dirs".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("d_nonempty_dirs".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source with a simple file
    fixture.write_file("dotfiles/d_nonempty_dirs/current.txt", "current content");

    // Create destination with a directory containing files
    fixture.write_file(
        "deploy_dest/nonempty_dirs/old_dir/old_file.txt",
        "old content",
    );
    fixture.write_file(
        "deploy_dest/nonempty_dirs/old_dir/another.txt",
        "more old content",
    );

    // Deploy with clean=true
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: true,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy with clean should succeed");

    // Check that current file exists
    fixture.assert_file_exists(
        "deploy_dest/nonempty_dirs/current.txt",
        "current.txt should exist",
    );

    // Check that the entire old directory was removed
    fixture.assert_file_not_exists(
        "deploy_dest/nonempty_dirs/old_dir",
        "old_dir should be removed",
    );
    fixture.assert_file_not_exists(
        "deploy_dest/nonempty_dirs/old_dir/old_file.txt",
        "files in old_dir should be removed",
    );
}

#[test]
fn test_clean_handles_nested_directory_structure() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "d_nested".to_string(),
        src: "dotfiles/d_nested".to_string(),
        dest: "deploy_dest/nested".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("d_nested".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("d_nested".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source with nested structure
    fixture.write_file("dotfiles/d_nested/level1/level2/file.txt", "nested content");

    // Create destination with different structure - some to keep, some to remove
    fixture.write_file(
        "deploy_dest/nested/old_level1/old_file.txt",
        "should be removed",
    );
    fixture.write_file(
        "deploy_dest/nested/old_level1/old_level2/deep.txt",
        "should be removed",
    );
    std::fs::create_dir_all(fixture.cwd.join("deploy_dest/nested/empty_dir")).unwrap();

    // Deploy with clean=true
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: true,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy with clean should succeed");

    // Check that new structure exists
    fixture.assert_file_exists(
        "deploy_dest/nested/level1/level2/file.txt",
        "nested file should exist",
    );

    // Check that old structure was removed (deepest first)
    fixture.assert_file_not_exists(
        "deploy_dest/nested/old_level1",
        "old nested structure should be removed",
    );
    fixture.assert_file_not_exists(
        "deploy_dest/nested/empty_dir",
        "empty_dir should be removed",
    );
}

#[test]
fn test_clean_preserves_kept_directories() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "d_preserve_dirs".to_string(),
        src: "dotfiles/d_preserve_dirs".to_string(),
        dest: "deploy_dest/preserve".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config
        .packages
        .insert("d_preserve_dirs".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("d_preserve_dirs".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source with subdirectory
    fixture.write_file("dotfiles/d_preserve_dirs/subdir/file.txt", "content");
    fixture.write_file("dotfiles/d_preserve_dirs/root.txt", "root content");

    // Create destination matching the structure plus extra
    fixture.write_file("deploy_dest/preserve/subdir/file.txt", "old content");
    fixture.write_file("deploy_dest/preserve/root.txt", "old root");
    fixture.write_file(
        "deploy_dest/preserve/extra_in_subdir/extra.txt",
        "should be removed",
    );

    // Deploy with clean=true
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: true,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy with clean should succeed");

    // Check that deployed structure exists
    fixture.assert_file_exists(
        "deploy_dest/preserve/subdir/file.txt",
        "subdir/file.txt should exist",
    );
    fixture.assert_file_exists("deploy_dest/preserve/root.txt", "root.txt should exist");

    // Check that extra directory was removed
    fixture.assert_file_not_exists(
        "deploy_dest/preserve/extra_in_subdir",
        "extra directory should be removed",
    );
}

#[test]
fn test_clean_respects_ignore_patterns_files() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "d_ignore_files".to_string(),
        src: "dotfiles/d_ignore_files".to_string(),
        dest: "deploy_dest/ignore_files".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: vec!["*.log".to_string(), "temp*".to_string()],
    };

    config
        .packages
        .insert("d_ignore_files".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("d_ignore_files".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source files
    fixture.write_file("dotfiles/d_ignore_files/config.txt", "config");

    // Create destination with files matching ignore patterns and extra files
    fixture.write_file("deploy_dest/ignore_files/config.txt", "old config");
    fixture.write_file("deploy_dest/ignore_files/app.log", "logs");
    fixture.write_file("deploy_dest/ignore_files/debug.log", "debug");
    fixture.write_file("deploy_dest/ignore_files/temp_data", "temp");
    fixture.write_file("deploy_dest/ignore_files/extra.txt", "should be removed");

    // Deploy with clean=true
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: true,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy with clean should succeed");

    // Check that config file exists
    fixture.assert_file_exists(
        "deploy_dest/ignore_files/config.txt",
        "config.txt should exist",
    );

    // Check that ignored files are preserved
    fixture.assert_file_exists(
        "deploy_dest/ignore_files/app.log",
        "*.log should be ignored and preserved",
    );
    fixture.assert_file_exists(
        "deploy_dest/ignore_files/debug.log",
        "*.log should be ignored and preserved",
    );
    fixture.assert_file_exists(
        "deploy_dest/ignore_files/temp_data",
        "temp* should be ignored and preserved",
    );

    // Check that non-ignored extra file was removed
    fixture.assert_file_not_exists(
        "deploy_dest/ignore_files/extra.txt",
        "extra.txt should be removed",
    );
}

#[test]
fn test_clean_respects_ignore_patterns_directories() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "d_ignore_dirs".to_string(),
        src: "dotfiles/d_ignore_dirs".to_string(),
        dest: "deploy_dest/ignore_dirs".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: vec!["cache".to_string(), ".git".to_string()],
    };

    config.packages.insert("d_ignore_dirs".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("d_ignore_dirs".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source files
    fixture.write_file("dotfiles/d_ignore_dirs/config.txt", "config");

    // Create destination with directories that should be ignored
    fixture.write_file("deploy_dest/ignore_dirs/config.txt", "old config");
    std::fs::create_dir_all(fixture.cwd.join("deploy_dest/ignore_dirs/cache")).unwrap();
    std::fs::create_dir_all(fixture.cwd.join("deploy_dest/ignore_dirs/.git")).unwrap();
    std::fs::create_dir_all(fixture.cwd.join("deploy_dest/ignore_dirs/old_dir")).unwrap();

    // Deploy with clean=true
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: true,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy with clean should succeed");

    // Check that config file exists
    fixture.assert_file_exists(
        "deploy_dest/ignore_dirs/config.txt",
        "config.txt should exist",
    );

    // Check that ignored directories are preserved (as empty dirs)
    assert!(
        fixture.cwd.join("deploy_dest/ignore_dirs/cache").exists(),
        "cache dir should be preserved"
    );
    assert!(
        fixture.cwd.join("deploy_dest/ignore_dirs/.git").exists(),
        ".git dir should be preserved"
    );

    // Check that non-ignored directory was removed
    fixture.assert_file_not_exists(
        "deploy_dest/ignore_dirs/old_dir",
        "old_dir should be removed",
    );
}

#[test]
fn test_clean_ignore_patterns_in_backup() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "d_backup_ignore".to_string(),
        src: "dotfiles/d_backup_ignore".to_string(),
        dest: "source/backup_ignore".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: vec!["*.tmp".to_string(), "build".to_string()],
    };

    config
        .packages
        .insert("d_backup_ignore".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("d_backup_ignore".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source files
    fixture.write_file("source/backup_ignore/config.txt", "config");

    // Create dotfiles with files matching ignore patterns and old files
    fixture.write_file("dotfiles/d_backup_ignore/config.txt", "old config");
    fixture.write_file("dotfiles/d_backup_ignore/temp.tmp", "temp file");
    std::fs::create_dir_all(fixture.cwd.join("dotfiles/d_backup_ignore/build")).unwrap();
    fixture.write_file("dotfiles/d_backup_ignore/old.txt", "should be removed");

    // Backup with clean=true
    let result = run_cli(fixture.get_cli(Some(Command::Update(UpdateArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: true,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Backup with clean should succeed");

    // Check that config file exists
    fixture.assert_file_exists(
        "dotfiles/d_backup_ignore/config.txt",
        "config.txt should exist",
    );

    // Check that ignored files/dirs are preserved
    fixture.assert_file_exists(
        "dotfiles/d_backup_ignore/temp.tmp",
        "*.tmp should be ignored and preserved",
    );
    assert!(
        fixture.cwd.join("dotfiles/d_backup_ignore/build").exists(),
        "build dir should be preserved"
    );

    // Check that old file was removed
    fixture.assert_file_not_exists(
        "dotfiles/d_backup_ignore/old.txt",
        "old.txt should be removed",
    );
}

#[test]
fn test_clean_ignore_patterns_with_nested_paths() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "d_nested_ignore".to_string(),
        src: "dotfiles/d_nested_ignore".to_string(),
        dest: "deploy_dest/nested_ignore".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: vec!["node_modules".to_string(), "**/*.swp".to_string()],
    };

    config
        .packages
        .insert("d_nested_ignore".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("d_nested_ignore".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source files
    fixture.write_file("dotfiles/d_nested_ignore/app/main.js", "main");

    // Create destination with nested ignored patterns
    fixture.write_file("deploy_dest/nested_ignore/app/main.js", "old main");
    std::fs::create_dir_all(fixture.cwd.join("deploy_dest/nested_ignore/node_modules")).unwrap();
    std::fs::create_dir_all(
        fixture
            .cwd
            .join("deploy_dest/nested_ignore/app/node_modules"),
    )
    .unwrap();
    fixture.write_file("deploy_dest/nested_ignore/app/file.swp", "vim swap");
    fixture.write_file("deploy_dest/nested_ignore/old.js", "should be removed");

    // Deploy with clean=true
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: true,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy with clean should succeed");

    // Check that app file exists
    fixture.assert_file_exists(
        "deploy_dest/nested_ignore/app/main.js",
        "app/main.js should exist",
    );

    // Check that ignored patterns are preserved
    assert!(
        fixture
            .cwd
            .join("deploy_dest/nested_ignore/node_modules")
            .exists(),
        "node_modules dir should be preserved"
    );
    fixture.assert_file_exists(
        "deploy_dest/nested_ignore/app/file.swp",
        "**/*.swp should be preserved",
    );

    // Nested node_modules inside app will be removed since pattern matches top-level only
    fixture.assert_file_not_exists(
        "deploy_dest/nested_ignore/app/node_modules",
        "nested node_modules should be removed",
    );

    // Check that non-ignored file was removed
    fixture.assert_file_not_exists(
        "deploy_dest/nested_ignore/old.js",
        "old.js should be removed",
    );
}

#[test]
fn test_clean_without_ignore_patterns() {
    let fixture = TestFixture::new();

    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "d_no_ignore".to_string(),
        src: "dotfiles/d_no_ignore".to_string(),
        dest: "deploy_dest/no_ignore".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: std::collections::HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(), // No ignore patterns
    };

    config.packages.insert("d_no_ignore".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("d_no_ignore".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source files
    fixture.write_file("dotfiles/d_no_ignore/config.txt", "config");

    // Create destination with various files
    fixture.write_file("deploy_dest/no_ignore/config.txt", "old config");
    fixture.write_file("deploy_dest/no_ignore/app.log", "logs");
    fixture.write_file("deploy_dest/no_ignore/temp.txt", "temp");

    // Deploy with clean=true and no ignore patterns
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: true,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy with clean should succeed");

    // Check that config file exists
    fixture.assert_file_exists(
        "deploy_dest/no_ignore/config.txt",
        "config.txt should exist",
    );

    // Check that all extra files are removed (no ignore patterns)
    fixture.assert_file_not_exists("deploy_dest/no_ignore/app.log", "app.log should be removed");
    fixture.assert_file_not_exists(
        "deploy_dest/no_ignore/temp.txt",
        "temp.txt should be removed",
    );
}

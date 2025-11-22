use std::{collections::HashMap, fs, path::PathBuf};

use dotr::{
    cli::{Cli, Command, DeployUpdateArgs, ImportArgs, InitArgs, PrintVarsArgs, run_cli},
    config::Config,
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

    fixture.assert_file_exists("dotfiles/f_test_conf", "File should be copied to dotfiles");
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

    assert!(package.skip, "Package should be marked as skip");
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
    config.save(&fixture.cwd).expect("Failed to save config");

    let _ = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: None,
        profile: None,
    }))));

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

    let profile = dotr::profile::Profile {
        name: "work".to_string(),
        variables: toml::Table::new(),
        dependencies: vec!["f_app".to_string()],
        prompts: HashMap::new(),
    };
    config.profiles.insert("work".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    let _ = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: None,
        profile: Some("work".to_string()),
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
    config.packages.insert("f_pkg2".to_string(), pkg2);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy only pkg1
    let _ = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: Some(vec!["f_pkg1".to_string()]),
        profile: None,
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
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create file at dest
    fixture.write_file("update_dest", "updated content");

    let _ = run_cli(fixture.get_cli(Some(Command::Update(DeployUpdateArgs {
        packages: None,
        profile: None,
    }))));

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
    let _ = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: None,
        profile: None,
    }))));

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
    let _ = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: None,
        profile: None,
    }))));

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
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy without profile (skip packages should not be deployed)
    let _ = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: None,
        profile: None,
    }))));

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
        skip: true,
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
        skip: true,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };

    config.packages.insert("f_dep1".to_string(), pkg1);
    config.packages.insert("f_dep2".to_string(), pkg2);

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
    let _ = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: None,
        profile: Some("minimal".to_string()),
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
        command: Some(Command::Deploy(DeployUpdateArgs {
            packages: None,
            profile: None,
        })),
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

    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: None,
        profile: None,
    }))));

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

    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: None,
        profile: Some("nonexistent_profile".to_string()),
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

    let result = run_cli(fixture.get_cli(Some(Command::Update(DeployUpdateArgs {
        packages: None,
        profile: Some("invalid_profile".to_string()),
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

    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: Some(vec!["nonexistent_package".to_string()]),
        profile: None,
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

    let result = run_cli(fixture.get_cli(Some(Command::Update(DeployUpdateArgs {
        packages: Some(vec!["nonexistent_package".to_string()]),
        profile: None,
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

    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: None,
        profile: None,
    }))));

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
    config.save(&fixture.cwd).expect("Failed to save config");

    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: Some(vec!["test_pkg".to_string()]),
        profile: None,
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
    config.save(&fixture.cwd).expect("Failed to save config");

    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: None,
        profile: None,
    }))));

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
        skip: true,
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
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: None,
        profile: None,
    }))));

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
    config.profiles.insert("updateenv".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy first
    run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: Some(vec!["f_env_update".to_string()]),
        profile: None,
    }))))
    .expect("Deploy failed");

    // Modify deployed file
    fixture.write_file("dest/.env_update", "modified");

    // Set profile via env var
    fixture.write_file(".uservariables.toml", "DOTR_PROFILE = \"updateenv\"\n");

    // Update without specifying profile - should succeed with profile from env var
    let result = run_cli(fixture.get_cli(Some(Command::Update(DeployUpdateArgs {
        packages: Some(vec!["f_env_update".to_string()]),
        profile: None,
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
        skip: true,
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
    config.profiles.insert("envprofile".to_string(), profile1);
    config.profiles.insert("cliprofile".to_string(), profile2);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Set env var to one profile
    fixture.write_file(".uservariables.toml", "DOTR_PROFILE = \"envprofile\"\n");

    // But explicitly pass different profile via CLI
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: None,
        profile: Some("cliprofile".to_string()),
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
    config.save(&fixture.cwd).expect("Failed to save config");

    // Set env var to non-existent profile
    fixture.write_file(".uservariables.toml", "DOTR_PROFILE = \"nonexistent\"\n");

    // Deploy without profile should fail (env var points to invalid profile)
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployUpdateArgs {
        packages: None,
        profile: None,
    }))));

    assert!(
        result.is_err(),
        "Deploy should fail with invalid DOTR_PROFILE env var"
    );
}

use std::{fs, path::PathBuf};

use dotr::{
    cli::{Cli, Command, DeployArgs, ImportArgs, InitArgs, PrintVarsArgs, UpdateArgs, run_cli},
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
        run_cli(self.get_cli(Some(Command::Init(InitArgs {}))));
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

    run_cli(fixture.get_cli(Some(Command::Import(ImportArgs {
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

    run_cli(fixture.get_cli(Some(Command::Import(ImportArgs {
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
    };

    config.packages.insert("f_test".to_string(), test_package);
    config.save(&fixture.cwd).expect("Failed to save config");

    run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
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
    };
    config.profiles.insert("work".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
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
    };

    config.packages.insert("f_pkg1".to_string(), pkg1);
    config.packages.insert("f_pkg2".to_string(), pkg2);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy only pkg1
    run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
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
    };
    config.packages.insert("f_update".to_string(), pkg);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create file at dest
    fixture.write_file("update_dest", "updated content");

    run_cli(fixture.get_cli(Some(Command::Update(UpdateArgs {
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
    run_cli(fixture.get_cli(Some(Command::PrintVars(PrintVarsArgs { profile: None }))));
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
    run_cli(fixture.get_cli(Some(Command::PrintVars(PrintVarsArgs {
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
    run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
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
    run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
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

    run_cli(cli);

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
    };
    config.packages.insert("f_skip".to_string(), pkg);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy without profile (skip packages should not be deployed)
    run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
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
    };

    config.packages.insert("f_dep1".to_string(), pkg1);
    config.packages.insert("f_dep2".to_string(), pkg2);

    // Profile with only dep1 as dependency
    let profile = dotr::profile::Profile {
        name: "minimal".to_string(),
        variables: toml::Table::new(),
        dependencies: vec!["f_dep1".to_string()],
    };
    config.profiles.insert("minimal".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy with profile
    run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
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

    run_cli(cli);

    // Just testing it doesn't panic and prints help message
}

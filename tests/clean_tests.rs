use std::{fs, path::PathBuf};

use dotr_dear::{
    cli::{Cli, Command, DeployArgs, UpdateArgs, run_cli},
    config::Config,
    package::Package,
};

struct TestFixture {
    cwd: PathBuf,
}

impl TestFixture {
    fn new() -> Self {
        let temp_dir =
            std::env::temp_dir().join(format!("dotr_clean_test_{}", uuid::Uuid::new_v4()));
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
        run_cli(self.get_cli(Some(Command::Init(dotr_dear::cli::InitArgs {}))))
            .expect("Init failed");
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
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.cwd).ok();
    }
}

#[test]
fn test_deploy_clean_by_default() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "test_pkg".to_string(),
        src: "dotfiles/test_pkg".to_string(),
        dest: "deploy_dest/test".to_string(),
        ..Default::default()
    };

    config.packages.insert("test_pkg".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("test_pkg".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source files
    fixture.write_file("dotfiles/test_pkg/file1.txt", "file1");
    fixture.write_file("dotfiles/test_pkg/file2.txt", "file2");

    // Create destination with extra file
    fixture.write_file("deploy_dest/test/file1.txt", "old");
    fixture.write_file("deploy_dest/test/extra.txt", "should be removed");

    // Deploy without specifying clean flag (should clean by default)
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: None,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy should succeed");

    fixture.assert_file_exists("deploy_dest/test/file1.txt", "file1 should exist");
    fixture.assert_file_exists("deploy_dest/test/file2.txt", "file2 should exist");
    fixture.assert_file_not_exists(
        "deploy_dest/test/extra.txt",
        "extra file should be removed by default clean",
    );
}

#[test]
fn test_update_clean_by_default() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "backup_pkg".to_string(),
        src: "dotfiles/backup_pkg".to_string(),
        dest: "source/backup".to_string(),
        ..Default::default()
    };

    config.packages.insert("backup_pkg".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("backup_pkg".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source files
    fixture.write_file("source/backup/file1.txt", "file1");
    fixture.write_file("source/backup/file2.txt", "file2");

    // Create dotfiles with extra file
    fixture.write_file("dotfiles/backup_pkg/file1.txt", "old");
    fixture.write_file("dotfiles/backup_pkg/extra.txt", "should be removed");

    // Update without specifying clean flag (should clean by default)
    let result = run_cli(fixture.get_cli(Some(Command::Update(UpdateArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: None,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Update should succeed");

    fixture.assert_file_exists("dotfiles/backup_pkg/file1.txt", "file1 should exist");
    fixture.assert_file_exists("dotfiles/backup_pkg/file2.txt", "file2 should exist");
    fixture.assert_file_not_exists(
        "dotfiles/backup_pkg/extra.txt",
        "extra file should be removed by default clean",
    );
}

#[test]
fn test_deploy_explicit_no_clean_overrides_default() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "no_clean_pkg".to_string(),
        src: "dotfiles/no_clean_pkg".to_string(),
        dest: "deploy_dest/no_clean".to_string(),
        ..Default::default()
    };

    config.packages.insert("no_clean_pkg".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("no_clean_pkg".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source files
    fixture.write_file("dotfiles/no_clean_pkg/file1.txt", "file1");
    fixture.write_file("dotfiles/no_clean_pkg/file2.txt", "file2");

    // Create destination with extra file
    fixture.write_file("deploy_dest/no_clean/file1.txt", "old");
    fixture.write_file("deploy_dest/no_clean/extra.txt", "should remain");

    // Deploy with explicit clean=false
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: Some(false),
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy should succeed");

    fixture.assert_file_exists("deploy_dest/no_clean/file1.txt", "file1 should exist");
    fixture.assert_file_exists("deploy_dest/no_clean/file2.txt", "file2 should exist");
    fixture.assert_file_exists(
        "deploy_dest/no_clean/extra.txt",
        "extra file should remain when clean=false",
    );
}

#[test]
fn test_package_clean_false_overrides_default() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "pkg_no_clean".to_string(),
        src: "dotfiles/pkg_no_clean".to_string(),
        dest: "deploy_dest/pkg_no_clean".to_string(),
        clean: false, // Package explicitly says no clean
        ..Default::default()
    };

    config.packages.insert("pkg_no_clean".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("pkg_no_clean".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source files
    fixture.write_file("dotfiles/pkg_no_clean/file1.txt", "file1");
    fixture.write_file("dotfiles/pkg_no_clean/file2.txt", "file2");

    // Create destination with extra file
    fixture.write_file("deploy_dest/pkg_no_clean/file1.txt", "old");
    fixture.write_file("deploy_dest/pkg_no_clean/extra.txt", "should remain");

    // Deploy without clean flag (should use package's clean=false setting)
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: None,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy should succeed");

    fixture.assert_file_exists("deploy_dest/pkg_no_clean/file1.txt", "file1 should exist");
    fixture.assert_file_exists("deploy_dest/pkg_no_clean/file2.txt", "file2 should exist");
    fixture.assert_file_exists(
        "deploy_dest/pkg_no_clean/extra.txt",
        "extra file should remain when package clean=false",
    );
}

#[test]
fn test_cli_clean_arg_overrides_package_setting() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "override_pkg".to_string(),
        src: "dotfiles/override_pkg".to_string(),
        dest: "deploy_dest/override".to_string(),
        clean: false, // Package says no clean
        ..Default::default()
    };

    config.packages.insert("override_pkg".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("override_pkg".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source files
    fixture.write_file("dotfiles/override_pkg/file1.txt", "file1");
    fixture.write_file("dotfiles/override_pkg/file2.txt", "file2");

    // Create destination with extra file
    fixture.write_file("deploy_dest/override/file1.txt", "old");
    fixture.write_file("deploy_dest/override/extra.txt", "should be removed");

    // Deploy with explicit clean=true (should override package's clean=false)
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: Some(true),
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy should succeed");

    fixture.assert_file_exists("deploy_dest/override/file1.txt", "file1 should exist");
    fixture.assert_file_exists("deploy_dest/override/file2.txt", "file2 should exist");
    fixture.assert_file_not_exists(
        "deploy_dest/override/extra.txt",
        "extra file should be removed when CLI arg overrides package setting",
    );
}

#[test]
fn test_package_clean_true_explicit() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "explicit_clean".to_string(),
        src: "dotfiles/explicit_clean".to_string(),
        dest: "deploy_dest/explicit_clean".to_string(),
        clean: true, // Explicitly set to true
        ..Default::default()
    };

    config
        .packages
        .insert("explicit_clean".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("explicit_clean".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source files
    fixture.write_file("dotfiles/explicit_clean/file1.txt", "file1");

    // Create destination with extra file
    fixture.write_file("deploy_dest/explicit_clean/extra.txt", "should be removed");

    // Deploy without clean flag
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: None,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Deploy should succeed");

    fixture.assert_file_exists("deploy_dest/explicit_clean/file1.txt", "file1 should exist");
    fixture.assert_file_not_exists(
        "deploy_dest/explicit_clean/extra.txt",
        "extra file should be removed when package clean=true",
    );
}

#[test]
fn test_update_with_package_clean_false() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "update_no_clean".to_string(),
        src: "dotfiles/update_no_clean".to_string(),
        dest: "source/update_no_clean".to_string(),
        clean: false,
        ..Default::default()
    };

    config
        .packages
        .insert("update_no_clean".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("update_no_clean".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source files
    fixture.write_file("source/update_no_clean/file1.txt", "file1");
    fixture.write_file("source/update_no_clean/file2.txt", "file2");

    // Create dotfiles with extra file
    fixture.write_file("dotfiles/update_no_clean/file1.txt", "old");
    fixture.write_file("dotfiles/update_no_clean/extra.txt", "should remain");

    // Update without clean flag
    let result = run_cli(fixture.get_cli(Some(Command::Update(UpdateArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: None,
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Update should succeed");

    fixture.assert_file_exists("dotfiles/update_no_clean/file1.txt", "file1 should exist");
    fixture.assert_file_exists("dotfiles/update_no_clean/file2.txt", "file2 should exist");
    fixture.assert_file_exists(
        "dotfiles/update_no_clean/extra.txt",
        "extra file should remain when package clean=false",
    );
}

#[test]
fn test_update_cli_override_package_clean() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();

    let package = Package {
        name: "update_override".to_string(),
        src: "dotfiles/update_override".to_string(),
        dest: "source/update_override".to_string(),
        clean: false,
        ..Default::default()
    };

    config
        .packages
        .insert("update_override".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("update_override".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create source files
    fixture.write_file("source/update_override/file1.txt", "file1");

    // Create dotfiles with extra file
    fixture.write_file("dotfiles/update_override/extra.txt", "should be removed");

    // Update with explicit clean=true
    let result = run_cli(fixture.get_cli(Some(Command::Update(UpdateArgs {
        packages: None,
        profile: None,
        ignore_errors: false,
        clean: Some(true),
        dry_run: false,
    }))));

    assert!(result.is_ok(), "Update should succeed");

    fixture.assert_file_exists("dotfiles/update_override/file1.txt", "file1 should exist");
    fixture.assert_file_not_exists(
        "dotfiles/update_override/extra.txt",
        "extra file should be removed when CLI overrides package setting",
    );
}

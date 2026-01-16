use std::fs;

use dotr_dear::{
    cli::{
        Cli, Command, DeployArgs, DiffArgs, ImportArgs, PackagesArgs, PackagesCommand,
        RemovePackageArgs, UpdateArgs, run_cli,
    },
    config::Config,
};

mod common;

const PLAYGROUND_DIR: &str = "tests/playground";

struct TestFixture {
    cwd: std::path::PathBuf,
}

impl TestFixture {
    fn new() -> Self {
        Self {
            cwd: std::path::PathBuf::from(PLAYGROUND_DIR),
        }
    }

    fn get_cli(&self, command: Option<Command>) -> Cli {
        Cli {
            command,
            working_dir: Some(PLAYGROUND_DIR.to_string()),
        }
    }

    fn init(&self) {
        run_cli(self.get_cli(Some(Command::Init(dotr_dear::cli::InitArgs {}))))
            .expect("Init failed");
    }

    fn get_config(&self) -> Config {
        Config::from_path(&self.cwd).expect("Failed to load config")
    }

    fn create_test_file(&self, name: &str, content: &str) {
        fs::create_dir_all(self.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
        fs::write(self.cwd.join(format!("dotfiles/{}", name)), content)
            .expect("Failed to create test file");
    }

    fn assert_file_exists(&self, path: &str) {
        assert!(self.cwd.join(path).exists(), "File should exist: {}", path);
    }

    fn assert_file_not_exists(&self, path: &str) {
        assert!(
            !self.cwd.join(path).exists(),
            "File should not exist: {}",
            path
        );
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        common::teardown(&self.cwd);
    }
}

#[test]
fn test_packages_import_subcommand() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a test file to import
    fs::create_dir_all(fixture.cwd.join("src")).expect("Failed to create src dir");
    fs::write(fixture.cwd.join("src/.test_import"), "test content\n")
        .expect("Failed to create test file");

    // Import using packages import subcommand
    run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: Some(PackagesCommand::Import(ImportArgs {
            path: "src/.test_import".to_string(),
            symlink: false,
            name: None,
            profile: None,
        })),
    }))))
    .expect("Packages import failed");

    // Verify package was imported
    let config = fixture.get_config();
    assert!(
        config.packages.contains_key("f_test_import"),
        "Package should be imported"
    );

    // Verify file was backed up
    fixture.assert_file_exists("dotfiles/f_test_import");
}

#[test]
fn test_packages_deploy_subcommand() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a test package
    fixture.create_test_file("f_deploy_test", "deploy test\n");

    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_deploy_test".to_string(),
        src: "dotfiles/f_deploy_test".to_string(),
        dest: "src/.deploy_test".to_string(),
        ..Default::default()
    };
    config.packages.insert("f_deploy_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy using packages deploy subcommand
    run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: Some(PackagesCommand::Deploy(DeployArgs {
            packages: Some(vec!["f_deploy_test".to_string()]),
            profile: None,
            ignore_errors: false,
            clean: Some(false),
            dry_run: false,
        })),
    }))))
    .expect("Packages deploy failed");

    // Verify file was deployed
    fixture.assert_file_exists("src/.deploy_test");
}

#[test]
fn test_packages_update_subcommand() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create and deploy a test package
    fixture.create_test_file("f_update_test", "original content\n");

    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_update_test".to_string(),
        src: "dotfiles/f_update_test".to_string(),
        dest: "src/.update_test".to_string(),
        ..Default::default()
    };
    config.packages.insert("f_update_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy first
    run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: Some(vec!["f_update_test".to_string()]),
        profile: None,
        ignore_errors: false,
        clean: Some(false),
        dry_run: false,
    }))))
    .expect("Deploy failed");

    // Modify the deployed file
    fs::write(fixture.cwd.join("src/.update_test"), "modified content\n")
        .expect("Failed to modify file");

    // Update using packages update subcommand
    run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: Some(PackagesCommand::Update(UpdateArgs {
            packages: Some(vec!["f_update_test".to_string()]),
            profile: None,
            ignore_errors: false,
            clean: Some(false),
            dry_run: false,
        })),
    }))))
    .expect("Packages update failed");

    // Verify the backup was updated
    let content = fs::read_to_string(fixture.cwd.join("dotfiles/f_update_test"))
        .expect("Failed to read backup");
    assert_eq!(content, "modified content\n");
}

#[test]
fn test_packages_diff_subcommand() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create and deploy a test package
    fixture.create_test_file("f_diff_test", "original content\n");

    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_diff_test".to_string(),
        src: "dotfiles/f_diff_test".to_string(),
        dest: "src/.diff_test".to_string(),
        ..Default::default()
    };
    config.packages.insert("f_diff_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy first
    run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: Some(vec!["f_diff_test".to_string()]),
        profile: None,
        ignore_errors: false,
        clean: Some(false),
        dry_run: false,
    }))))
    .expect("Deploy failed");

    // Modify the deployed file
    fs::write(fixture.cwd.join("src/.diff_test"), "modified content\n")
        .expect("Failed to modify file");

    // Diff using packages diff subcommand - should not fail
    run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: Some(PackagesCommand::Diff(DiffArgs {
            packages: Some(vec!["f_diff_test".to_string()]),
            profile: None,
            ignore_errors: false,
        })),
    }))))
    .expect("Packages diff failed");
}

#[test]
fn test_packages_remove_subcommand() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a test package
    fixture.create_test_file("f_remove_test", "remove test\n");

    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_remove_test".to_string(),
        src: "dotfiles/f_remove_test".to_string(),
        dest: "src/.remove_test".to_string(),
        ..Default::default()
    };
    config.packages.insert("f_remove_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Remove using packages remove subcommand
    run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: Some(PackagesCommand::Remove(RemovePackageArgs {
            packages: Some(vec!["f_remove_test".to_string()]),
            force: false,
            remove_orphans: false,
            dry_run: false,
            profile: None,
        })),
    }))))
    .expect("Packages remove failed");

    // Verify package was removed from config
    let config = fixture.get_config();
    assert!(
        !config.packages.contains_key("f_remove_test"),
        "Package should be removed from config"
    );

    // Verify source file was removed
    fixture.assert_file_not_exists("dotfiles/f_remove_test");
}

#[test]
fn test_packages_import_with_profile() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a profile
    let mut config = fixture.get_config();
    let profile = dotr_dear::profile::Profile::new("test_profile");
    config.profiles.insert("test_profile".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create a test file to import
    fs::create_dir_all(fixture.cwd.join("src")).expect("Failed to create src dir");
    fs::write(
        fixture.cwd.join("src/.test_profile_import"),
        "profile test\n",
    )
    .expect("Failed to create test file");

    // Import with profile using packages import subcommand
    run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: Some("test_profile".to_string()),
        command: Some(PackagesCommand::Import(ImportArgs {
            path: "src/.test_profile_import".to_string(),
            symlink: false,
            name: None,
            profile: Some("test_profile".to_string()),
        })),
    }))))
    .expect("Packages import with profile failed");

    // Verify package was added to profile
    let config = fixture.get_config();
    let profile = config.profiles.get("test_profile").unwrap();
    assert!(
        profile
            .dependencies
            .contains(&"f_test_profile_import".to_string()),
        "Package should be added to profile dependencies"
    );
}

#[test]
fn test_packages_deploy_with_profile() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a profile
    let mut config = fixture.get_config();
    let mut profile = dotr_dear::profile::Profile::new("deploy_profile");
    profile.dependencies.push("f_profile_deploy".to_string());
    config
        .profiles
        .insert("deploy_profile".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create a test package
    fixture.create_test_file("f_profile_deploy", "profile deploy\n");

    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_profile_deploy".to_string(),
        src: "dotfiles/f_profile_deploy".to_string(),
        dest: "src/.profile_deploy".to_string(),
        ..Default::default()
    };
    config
        .packages
        .insert("f_profile_deploy".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy with profile using packages deploy subcommand
    run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: Some("deploy_profile".to_string()),
        command: Some(PackagesCommand::Deploy(DeployArgs {
            packages: None, // Deploy all packages in profile
            profile: Some("deploy_profile".to_string()),
            ignore_errors: false,
            clean: Some(false),
            dry_run: false,
        })),
    }))))
    .expect("Packages deploy with profile failed");

    // Verify file was deployed
    fixture.assert_file_exists("src/.profile_deploy");
}

#[test]
fn test_packages_deploy_dry_run() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a test package
    fixture.create_test_file("f_dryrun_test", "dry run test\n");

    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_dryrun_test".to_string(),
        src: "dotfiles/f_dryrun_test".to_string(),
        dest: "src/.dryrun_test".to_string(),
        ..Default::default()
    };
    config.packages.insert("f_dryrun_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy with dry-run using packages deploy subcommand
    run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: Some(PackagesCommand::Deploy(DeployArgs {
            packages: Some(vec!["f_dryrun_test".to_string()]),
            profile: None,
            ignore_errors: false,
            clean: Some(false),
            dry_run: true,
        })),
    }))))
    .expect("Packages deploy dry-run failed");

    // Verify file was NOT actually deployed
    fixture.assert_file_not_exists("src/.dryrun_test");
}

#[test]
fn test_packages_update_dry_run() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create and deploy a test package
    fixture.create_test_file("f_update_dryrun", "original\n");

    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_update_dryrun".to_string(),
        src: "dotfiles/f_update_dryrun".to_string(),
        dest: "src/.update_dryrun".to_string(),
        ..Default::default()
    };
    config
        .packages
        .insert("f_update_dryrun".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy first
    run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs {
        packages: Some(vec!["f_update_dryrun".to_string()]),
        profile: None,
        ignore_errors: false,
        clean: Some(false),
        dry_run: false,
    }))))
    .expect("Deploy failed");

    // Modify the deployed file
    fs::write(fixture.cwd.join("src/.update_dryrun"), "modified\n").expect("Failed to modify file");

    // Update with dry-run using packages update subcommand
    run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: Some(PackagesCommand::Update(UpdateArgs {
            packages: Some(vec!["f_update_dryrun".to_string()]),
            profile: None,
            ignore_errors: false,
            clean: Some(false),
            dry_run: true,
        })),
    }))))
    .expect("Packages update dry-run failed");

    // Verify the backup was NOT updated
    let content = fs::read_to_string(fixture.cwd.join("dotfiles/f_update_dryrun"))
        .expect("Failed to read backup");
    assert_eq!(
        content, "original\n",
        "Backup should not be updated in dry-run"
    );
}

#[test]
fn test_packages_multiple_packages_deploy() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create multiple test packages
    fixture.create_test_file("f_multi1", "content1\n");
    fixture.create_test_file("f_multi2", "content2\n");
    fixture.create_test_file("f_multi3", "content3\n");

    let mut config = fixture.get_config();
    for i in 1..=3 {
        let package = dotr_dear::package::Package {
            name: format!("f_multi{}", i),
            src: format!("dotfiles/f_multi{}", i),
            dest: format!("src/.multi{}", i),
            ..Default::default()
        };
        config.packages.insert(format!("f_multi{}", i), package);
    }
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy multiple packages using packages deploy subcommand
    run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: Some(PackagesCommand::Deploy(DeployArgs {
            packages: Some(vec![
                "f_multi1".to_string(),
                "f_multi2".to_string(),
                "f_multi3".to_string(),
            ]),
            profile: None,
            ignore_errors: false,
            clean: Some(false),
            dry_run: false,
        })),
    }))))
    .expect("Packages deploy multiple failed");

    // Verify all files were deployed
    fixture.assert_file_exists("src/.multi1");
    fixture.assert_file_exists("src/.multi2");
    fixture.assert_file_exists("src/.multi3");
}

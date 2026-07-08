use std::{fs, path::PathBuf};

use dotr_dear::{
    cli::{
        Cli, Command, InitArgs, PackagesArgs, PackagesCommand, PackagesListArgs, ProfileRemoveArgs,
        ProfilesAddArgs, ProfilesArgs, ProfilesCommand, ProfilesListArgs, RemovePackageArgs,
        run_cli,
    },
    config::Config,
    package::Package,
    profile::Profile,
};

mod common;

struct TestFixture {
    cwd: PathBuf,
}

impl TestFixture {
    fn new() -> Self {
        let temp_dir = std::env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
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

    fn write_file(&self, path: &str, content: &str) {
        let file_path = self.cwd.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        fs::write(file_path, content).expect("Failed to write file");
    }

    fn assert_file_exists(&self, path: &str, message: &str) {
        assert!(self.cwd.join(path).exists(), "{}", message);
    }

    fn read_file(&self, path: &str) -> String {
        fs::read_to_string(self.cwd.join(path)).expect("Failed to read file")
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        common::teardown(&self.cwd);
    }
}

// ==========================
// Packages Subcommand Tests
// ==========================

#[test]
fn test_packages_list_empty() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: Some(PackagesCommand::List(PackagesListArgs {
            verbose: false,
            plain: false,
        })),
    }))));

    assert!(result.is_ok());
}

#[test]
fn test_packages_list_with_packages() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add some packages to config
    let mut config = fixture.get_config();
    let pkg1 = Package::new("test-pkg1", "dotfiles/pkg1", "dest/pkg1");
    let pkg2 = Package::new("test-pkg2", "dotfiles/pkg2", "dest/pkg2");

    config.packages.insert("test-pkg1".to_string(), pkg1);
    config.packages.insert("test-pkg2".to_string(), pkg2);

    // Add packages to default profile
    config
        .profiles
        .entry("default".to_string())
        .or_insert_with(|| Profile::new("default"))
        .dependencies
        .push("test-pkg1".to_string());
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("test-pkg2".to_string());

    config.save(&fixture.cwd).expect("Failed to save config");

    let result = run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: Some(PackagesCommand::List(PackagesListArgs {
            verbose: false,
            plain: false,
        })),
    }))));

    assert!(result.is_ok());
}

#[test]
fn test_packages_list_verbose() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add a package with details
    let mut config = fixture.get_config();

    // Create the dependency packages first
    let dep1 = Package::new("dep1", "dotfiles/dep1", "dest/dep1");
    let dep2 = Package::new("dep2", "dotfiles/dep2", "dest/dep2");
    config.packages.insert("dep1".to_string(), dep1);
    config.packages.insert("dep2".to_string(), dep2);

    // Now create the main package with dependencies
    let mut pkg = Package::new("test-pkg", "dotfiles/pkg", "dest/pkg");
    pkg.dependencies = Some(vec!["dep1".to_string(), "dep2".to_string()]);
    pkg.targets
        .insert("target1".to_string(), "dest1".to_string());

    config.packages.insert("test-pkg".to_string(), pkg);
    config
        .profiles
        .entry("default".to_string())
        .or_insert_with(|| Profile::new("default"))
        .dependencies
        .push("test-pkg".to_string());

    config.save(&fixture.cwd).expect("Failed to save config");

    let result = run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: Some(PackagesCommand::List(PackagesListArgs {
            verbose: true,
            plain: false,
        })),
    }))));

    if let Err(e) = &result {
        println!("Error: {:?}", e);
    }
    assert!(result.is_ok());
}

#[test]
fn test_packages_list_with_specific_profile() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create packages
    let mut config = fixture.get_config();
    let pkg1 = Package::new("pkg1", "dotfiles/pkg1", "dest/pkg1");
    let pkg2 = Package::new("pkg2", "dotfiles/pkg2", "dest/pkg2");

    config.packages.insert("pkg1".to_string(), pkg1);
    config.packages.insert("pkg2".to_string(), pkg2);

    // Create test profile with only pkg1
    let mut profile = Profile::new("test-profile");
    profile.dependencies.push("pkg1".to_string());
    config.profiles.insert("test-profile".to_string(), profile);

    config.save(&fixture.cwd).expect("Failed to save config");

    let result = run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: Some("test-profile".to_string()),
        command: Some(PackagesCommand::List(PackagesListArgs {
            verbose: false,
            plain: false,
        })),
    }))));

    assert!(result.is_ok());
}

#[test]
fn test_packages_list_skipped_packages() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create package with skip flag
    let mut config = fixture.get_config();
    let mut pkg1 = Package::new("pkg1", "dotfiles/pkg1", "dest/pkg1");
    pkg1.skip = true;
    let pkg2 = Package::new("pkg2", "dotfiles/pkg2", "dest/pkg2");

    config.packages.insert("pkg1".to_string(), pkg1);
    config.packages.insert("pkg2".to_string(), pkg2);

    config
        .profiles
        .entry("default".to_string())
        .or_insert_with(|| Profile::new("default"))
        .dependencies
        .extend(vec!["pkg1".to_string(), "pkg2".to_string()]);

    config.save(&fixture.cwd).expect("Failed to save config");

    let result = run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: Some(PackagesCommand::List(PackagesListArgs {
            verbose: false,
            plain: false,
        })),
    }))));

    assert!(result.is_ok());
    // Skipped packages should not appear in the list
}

// ==========================
// Profiles Subcommand Tests
// ==========================

#[test]
fn test_profiles_list_empty() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(fixture.get_cli(Some(Command::Profiles(ProfilesArgs {
        command: Some(ProfilesCommand::List(ProfilesListArgs {
            verbose: false,
            plain: false,
        })),
    }))));

    assert!(result.is_ok());
}

#[test]
fn test_profiles_list_with_profiles() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add profiles
    let mut config = fixture.get_config();
    let profile1 = Profile::new("profile1");
    let profile2 = Profile::new("profile2");

    config.profiles.insert("profile1".to_string(), profile1);
    config.profiles.insert("profile2".to_string(), profile2);

    config.save(&fixture.cwd).expect("Failed to save config");

    let result = run_cli(fixture.get_cli(Some(Command::Profiles(ProfilesArgs {
        command: Some(ProfilesCommand::List(ProfilesListArgs {
            verbose: false,
            plain: false,
        })),
    }))));

    assert!(result.is_ok());
}

#[test]
fn test_profiles_list_verbose() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add profile with details
    let mut config = fixture.get_config();
    let mut profile = Profile::new("test-profile");
    profile.dependencies = vec!["pkg1".to_string(), "pkg2".to_string()];
    profile.variables.insert(
        "VAR1".to_string(),
        toml::Value::String("value1".to_string()),
    );
    profile
        .prompts
        .insert("PROMPT1".to_string(), "Enter value".to_string());

    config.profiles.insert("test-profile".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    let result = run_cli(fixture.get_cli(Some(Command::Profiles(ProfilesArgs {
        command: Some(ProfilesCommand::List(ProfilesListArgs {
            verbose: true,
            plain: false,
        })),
    }))));

    assert!(result.is_ok());
}

#[test]
fn test_profiles_add_new_profile() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(fixture.get_cli(Some(Command::Profiles(ProfilesArgs {
        command: Some(ProfilesCommand::Add(ProfilesAddArgs {
            name: "new-profile".to_string(),
            set_as_current: false,
        })),
    }))));

    assert!(result.is_ok());

    // Verify profile was added
    let config = fixture.get_config();
    assert!(
        config.profiles.contains_key("new-profile"),
        "Profile should be added to config"
    );
}

#[test]
fn test_profiles_add_duplicate_fails() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add a profile first
    let mut config = fixture.get_config();
    config.profiles.insert(
        "existing-profile".to_string(),
        Profile::new("existing-profile"),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // Try to add the same profile again
    let result = run_cli(fixture.get_cli(Some(Command::Profiles(ProfilesArgs {
        command: Some(ProfilesCommand::Add(ProfilesAddArgs {
            name: "existing-profile".to_string(),
            set_as_current: false,
        })),
    }))));

    assert!(result.is_err(), "Adding duplicate profile should fail");
}

#[test]
fn test_profiles_add_with_set_as_current() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(fixture.get_cli(Some(Command::Profiles(ProfilesArgs {
        command: Some(ProfilesCommand::Add(ProfilesAddArgs {
            name: "current-profile".to_string(),
            set_as_current: true,
        })),
    }))));

    assert!(result.is_ok());

    // Verify profile was added
    let config = fixture.get_config();
    assert!(
        config.profiles.contains_key("current-profile"),
        "Profile should be added to config"
    );

    // Verify it was set as current in .uservariables.toml
    fixture.assert_file_exists(".uservariables.toml", "User variables file should exist");
    let uservars_content = fixture.read_file(".uservariables.toml");
    assert!(
        uservars_content.contains("DOTR_PROFILE"),
        "DOTR_PROFILE should be set in user variables"
    );
    assert!(
        uservars_content.contains("current-profile"),
        "Profile name should be in user variables"
    );
}

#[test]
fn test_profiles_add_preserves_existing_uservariables() {
    let fixture = TestFixture::new();
    fixture.init();

    // Set some existing user variables
    fixture.write_file(".uservariables.toml", r#"EXISTING_VAR = "value""#);

    let result = run_cli(fixture.get_cli(Some(Command::Profiles(ProfilesArgs {
        command: Some(ProfilesCommand::Add(ProfilesAddArgs {
            name: "new-profile".to_string(),
            set_as_current: true,
        })),
    }))));

    assert!(result.is_ok());

    // Verify existing variable is preserved
    let uservars_content = fixture.read_file(".uservariables.toml");
    assert!(
        uservars_content.contains("EXISTING_VAR"),
        "Existing user variable should be preserved"
    );
    assert!(
        uservars_content.contains("DOTR_PROFILE"),
        "DOTR_PROFILE should be added"
    );
}

// ==========================
// No Command Tests
// ==========================

#[test]
fn test_packages_no_subcommand() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: None,
    }))));

    assert!(
        result.is_ok(),
        "Should handle missing subcommand gracefully"
    );
}

#[test]
fn test_profiles_no_subcommand() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(fixture.get_cli(Some(Command::Profiles(ProfilesArgs { command: None }))));

    assert!(
        result.is_ok(),
        "Should handle missing subcommand gracefully"
    );
}

// ==========================
// Remove Command Tests
// ==========================

#[test]
fn test_remove_command_success() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a package
    let mut config = fixture.get_config();
    let pkg = Package::new("test-pkg", "dotfiles/test-pkg", "dest/test-pkg");
    config.packages.insert("test-pkg".to_string(), pkg);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create the package directory
    fixture.write_file("dotfiles/test-pkg/file.txt", "content");

    let result = run_cli(fixture.get_cli(Some(Command::Remove(RemovePackageArgs {
        packages: Some(vec!["test-pkg".to_string()]),
        force: false,
        remove_orphans: false,
        dry_run: false,
        profile: None,
    }))));

    assert!(result.is_ok(), "Remove command should succeed");

    // Verify package was removed
    let config = fixture.get_config();
    assert!(
        !config.packages.contains_key("test-pkg"),
        "Package should be removed from config"
    );
}

#[test]
fn test_remove_command_with_force() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a package and add it to a profile dependency
    let mut config = fixture.get_config();
    let pkg = Package::new("test-pkg", "dotfiles/test-pkg", "dest/test-pkg");
    config.packages.insert("test-pkg".to_string(), pkg);

    let mut test_profile = Profile::new("test-profile");
    test_profile.dependencies.push("test-pkg".to_string());
    config
        .profiles
        .insert("test-profile".to_string(), test_profile);

    config.save(&fixture.cwd).expect("Failed to save config");
    fixture.write_file("dotfiles/test-pkg/file.txt", "content");

    // Should succeed with force flag
    let result = run_cli(fixture.get_cli(Some(Command::Remove(RemovePackageArgs {
        packages: Some(vec!["test-pkg".to_string()]),
        force: true,
        remove_orphans: false,
        dry_run: false,
        profile: None,
    }))));

    assert!(result.is_ok(), "Remove with force should succeed");

    let config = fixture.get_config();
    assert!(!config.packages.contains_key("test-pkg"));
}

#[test]
fn test_remove_command_dry_run() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();
    let pkg = Package::new("test-pkg", "dotfiles/test-pkg", "dest/test-pkg");
    config.packages.insert("test-pkg".to_string(), pkg);
    config.save(&fixture.cwd).expect("Failed to save config");

    let result = run_cli(fixture.get_cli(Some(Command::Remove(RemovePackageArgs {
        packages: Some(vec!["test-pkg".to_string()]),
        force: false,
        remove_orphans: false,
        dry_run: true,
        profile: None,
    }))));

    assert!(result.is_ok(), "Dry run should succeed");

    // Package should still exist
    let config = fixture.get_config();
    assert!(
        config.packages.contains_key("test-pkg"),
        "Package should not be removed in dry run"
    );
}

#[test]
fn test_remove_command_nonexistent_package() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(fixture.get_cli(Some(Command::Remove(RemovePackageArgs {
        packages: Some(vec!["nonexistent".to_string()]),
        force: false,
        remove_orphans: false,
        dry_run: false,
        profile: None,
    }))));

    assert!(result.is_err(), "Should fail for nonexistent package");
}

// ==========================
// Packages Remove Subcommand Tests
// ==========================

#[test]
fn test_packages_remove_subcommand() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a package
    let mut config = fixture.get_config();
    let pkg = Package::new("test-pkg", "dotfiles/test-pkg", "dest/test-pkg");
    config.packages.insert("test-pkg".to_string(), pkg);
    config.save(&fixture.cwd).expect("Failed to save config");
    fixture.write_file("dotfiles/test-pkg/file.txt", "content");

    let result = run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: Some(PackagesCommand::Remove(RemovePackageArgs {
            packages: Some(vec!["test-pkg".to_string()]),
            force: false,
            remove_orphans: false,
            dry_run: false,
            profile: None,
        })),
    }))));

    assert!(result.is_ok(), "Packages remove should succeed");

    let config = fixture.get_config();
    assert!(!config.packages.contains_key("test-pkg"));
}

#[test]
fn test_packages_remove_with_orphan_cleanup() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();

    // Create an orphan package
    let orphan_pkg = Package::new("orphan-pkg", "dotfiles/orphan-pkg", "dest/orphan");
    config.packages.insert("orphan-pkg".to_string(), orphan_pkg);

    // Create a used package
    let used_pkg = Package::new("used-pkg", "dotfiles/used-pkg", "dest/used");
    config.packages.insert("used-pkg".to_string(), used_pkg);

    let mut profile = config.profiles.get_mut("default").unwrap().clone();
    profile.dependencies.push("used-pkg".to_string());
    config.profiles.insert("default".to_string(), profile);

    config.save(&fixture.cwd).expect("Failed to save config");

    fixture.write_file("dotfiles/orphan-pkg/file.txt", "orphan");
    fixture.write_file("dotfiles/used-pkg/file.txt", "used");

    let result = run_cli(fixture.get_cli(Some(Command::Packages(PackagesArgs {
        profile: None,
        command: Some(PackagesCommand::Remove(RemovePackageArgs {
            packages: Some(vec![]), // Empty list, only remove orphans
            force: false,
            remove_orphans: true,
            dry_run: false,
            profile: None,
        })),
    }))));

    assert!(result.is_ok(), "Remove orphans should succeed");

    let config = fixture.get_config();
    assert!(!config.packages.contains_key("orphan-pkg"));
    assert!(config.packages.contains_key("used-pkg"));
}

// ==========================
// Profiles Remove Subcommand Tests
// ==========================

#[test]
fn test_profiles_remove_subcommand() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add a test profile
    let mut config = fixture.get_config();
    let profile = Profile::new("test-profile");
    config.profiles.insert("test-profile".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    let result = run_cli(fixture.get_cli(Some(Command::Profiles(ProfilesArgs {
        command: Some(ProfilesCommand::Remove(ProfileRemoveArgs {
            name: "test-profile".to_string(),
            dry_run: false,
            remove_orphans: false,
        })),
    }))));

    assert!(result.is_ok(), "Profiles remove should succeed");

    let config = fixture.get_config();
    assert!(!config.profiles.contains_key("test-profile"));
}

#[test]
fn test_profiles_remove_default_fails() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(fixture.get_cli(Some(Command::Profiles(ProfilesArgs {
        command: Some(ProfilesCommand::Remove(ProfileRemoveArgs {
            name: "default".to_string(),
            dry_run: false,
            remove_orphans: false,
        })),
    }))));

    assert!(result.is_err(), "Should not allow removing default profile");
}

#[test]
fn test_profiles_remove_with_orphan_cleanup() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();

    // Create a profile with a package
    let mut profile = Profile::new("test-profile");
    profile.dependencies.push("profile-pkg".to_string());
    config.profiles.insert("test-profile".to_string(), profile);

    let pkg = Package::new("profile-pkg", "dotfiles/profile-pkg", "dest/profile-pkg");
    config.packages.insert("profile-pkg".to_string(), pkg);

    config.save(&fixture.cwd).expect("Failed to save config");
    fixture.write_file("dotfiles/profile-pkg/file.txt", "content");

    let result = run_cli(fixture.get_cli(Some(Command::Profiles(ProfilesArgs {
        command: Some(ProfilesCommand::Remove(ProfileRemoveArgs {
            name: "test-profile".to_string(),
            dry_run: false,
            remove_orphans: true,
        })),
    }))));

    assert!(
        result.is_ok(),
        "Profiles remove with orphans should succeed"
    );

    let config = fixture.get_config();
    assert!(!config.profiles.contains_key("test-profile"));
    assert!(!config.packages.contains_key("profile-pkg"));
}

#[test]
fn test_profiles_remove_dry_run() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();
    let profile = Profile::new("test-profile");
    config.profiles.insert("test-profile".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    let result = run_cli(fixture.get_cli(Some(Command::Profiles(ProfilesArgs {
        command: Some(ProfilesCommand::Remove(ProfileRemoveArgs {
            name: "test-profile".to_string(),
            dry_run: true,
            remove_orphans: false,
        })),
    }))));

    assert!(result.is_ok(), "Dry run should succeed");

    let config = fixture.get_config();
    assert!(
        config.profiles.contains_key("test-profile"),
        "Profile should not be removed in dry run"
    );
}

#[test]
fn test_profiles_remove_nonexistent() {
    let fixture = TestFixture::new();
    fixture.init();

    let result = run_cli(fixture.get_cli(Some(Command::Profiles(ProfilesArgs {
        command: Some(ProfilesCommand::Remove(ProfileRemoveArgs {
            name: "nonexistent".to_string(),
            dry_run: false,
            remove_orphans: false,
        })),
    }))));

    assert!(result.is_err(), "Should fail for nonexistent profile");
}

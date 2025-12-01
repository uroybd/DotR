use std::fs;

use dotr_dear::{
    cli::{DeployArgs, InitArgs, run_cli},
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

    fn get_cli(&self, command: Option<dotr_dear::cli::Command>) -> dotr_dear::cli::Cli {
        dotr_dear::cli::Cli {
            command,
            working_dir: Some(PLAYGROUND_DIR.to_string()),
        }
    }

    fn init(&self) {
        run_cli(self.get_cli(Some(dotr_dear::cli::Command::Init(InitArgs {}))))
            .expect("Init failed");
    }

    fn deploy(&self, packages: Option<Vec<String>>, profile: Option<String>) {
        run_cli(
            self.get_cli(Some(dotr_dear::cli::Command::Deploy(DeployArgs {
                packages,
                profile,
                ignore_errors: false,
                clean: false,
                dry_run: false,
            }))),
        )
        .expect("Deploy failed");
    }

    fn get_config(&self) -> Config {
        Config::from_path(&self.cwd).expect("Failed to load config")
    }

    fn assert_file_exists(&self, path: &str, message: &str) {
        assert!(self.cwd.join(path).exists(), "{}", message);
    }

    fn assert_file_content_contains(&self, path: &str, content: &str, message: &str) {
        let file_content = fs::read_to_string(self.cwd.join(path))
            .unwrap_or_else(|_| panic!("Failed to read file: {}", path));
        assert!(file_content.contains(content), "{}", message);
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        common::teardown(&self.cwd);
    }
}

#[test]
fn test_dest_with_variable_substitution() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add a custom variable
    let mut config = fixture.get_config();
    config.variables.insert(
        "CONFIG_DIR".to_string(),
        toml::Value::String(".myconfigdir".to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create a test file in dotfiles
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_test_config"), "test content\n")
        .expect("Failed to create test file");

    // Add package with templated dest
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_test_config".to_string(),
        src: "dotfiles/f_test_config".to_string(),
        dest: "src/{{ CONFIG_DIR }}/config.txt".to_string(),
        ..Default::default()
    };
    config.packages.insert("f_test_config".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy the package
    fixture.deploy(Some(vec!["f_test_config".to_string()]), None);

    // Verify the file was deployed to the correct location
    fixture.assert_file_exists(
        "src/.myconfigdir/config.txt",
        "File should be deployed to templated dest path",
    );

    fixture.assert_file_content_contains(
        "src/.myconfigdir/config.txt",
        "test content",
        "Deployed file should have correct content",
    );
}

#[test]
fn test_dest_with_home_variable() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a test file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_bashrc"), "export TEST=1\n")
        .expect("Failed to create test file");

    // Add package with HOME variable in dest
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_bashrc".to_string(),
        src: "dotfiles/f_bashrc".to_string(),
        dest: "{{ HOME }}/.bashrc_test".to_string(),
        ..Default::default()
    };
    config.packages.insert("f_bashrc".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_bashrc".to_string()]), None);

    // Verify deployment - should be in home directory
    let home = std::env::home_dir().expect("Failed to get home directory");
    let expected_path = home.join(".bashrc_test");
    assert!(
        expected_path.exists(),
        "File should be deployed to home directory"
    );

    // Cleanup
    std::fs::remove_file(&expected_path).ok();
}

#[test]
fn test_dest_with_user_variable() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a test file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_user_config"), "config\n")
        .expect("Failed to create test file");

    // Add package with USER variable in dest
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_user_config".to_string(),
        src: "dotfiles/f_user_config".to_string(),
        dest: "src/{{ USER }}_config.txt".to_string(),
        ..Default::default()
    };
    config.packages.insert("f_user_config".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_user_config".to_string()]), None);

    // Get the current username
    let username = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .expect("Failed to get username");

    // Verify the file was deployed with username in path
    fixture.assert_file_exists(
        &format!("src/{}_config.txt", username),
        "File should be deployed with username in path",
    );
}

#[test]
fn test_dest_with_multiple_variables() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add custom variables
    let mut config = fixture.get_config();
    config.variables.insert(
        "APP_NAME".to_string(),
        toml::Value::String("myapp".to_string()),
    );
    config.variables.insert(
        "VERSION".to_string(),
        toml::Value::String("1.0".to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create a test file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_app_config"), "app config\n")
        .expect("Failed to create test file");

    // Add package with multiple variables in dest
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_app_config".to_string(),
        src: "dotfiles/f_app_config".to_string(),
        dest: "src/{{ APP_NAME }}/v{{ VERSION }}/config.txt".to_string(),
        ..Default::default()
    };
    config.packages.insert("f_app_config".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_app_config".to_string()]), None);

    // Verify
    fixture.assert_file_exists(
        "src/myapp/v1.0/config.txt",
        "File should be deployed with all variables substituted",
    );
}

#[test]
fn test_targets_with_templated_dest() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add a variable
    let mut config = fixture.get_config();
    config.variables.insert(
        "DEV_DIR".to_string(),
        toml::Value::String("development".to_string()),
    );
    config.variables.insert(
        "PROD_DIR".to_string(),
        toml::Value::String("production".to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create profiles
    let mut config = fixture.get_config();
    let dev_profile = dotr_dear::profile::Profile::new("dev");
    let prod_profile = dotr_dear::profile::Profile::new("prod");
    config.profiles.insert("dev".to_string(), dev_profile);
    config.profiles.insert("prod".to_string(), prod_profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create a test file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_env_config"), "env config\n")
        .expect("Failed to create test file");

    // Add package with templated targets
    let mut config = fixture.get_config();
    let mut targets = std::collections::HashMap::new();
    targets.insert(
        "dev".to_string(),
        "src/{{ DEV_DIR }}/config.txt".to_string(),
    );
    targets.insert(
        "prod".to_string(),
        "src/{{ PROD_DIR }}/config.txt".to_string(),
    );

    let package = dotr_dear::package::Package {
        name: "f_env_config".to_string(),
        src: "dotfiles/f_env_config".to_string(),
        dest: "src/default/config.txt".to_string(),
        targets,
        ..Default::default()
    };
    config.packages.insert("f_env_config".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy with dev profile
    fixture.deploy(
        Some(vec!["f_env_config".to_string()]),
        Some("dev".to_string()),
    );

    // Verify dev deployment
    fixture.assert_file_exists(
        "src/development/config.txt",
        "File should be deployed to dev path with variable substitution",
    );

    // Clean up for prod test
    std::fs::remove_file(fixture.cwd.join("src/development/config.txt"))
        .expect("Failed to cleanup dev file");

    // Deploy with prod profile
    fixture.deploy(
        Some(vec!["f_env_config".to_string()]),
        Some("prod".to_string()),
    );

    // Verify prod deployment
    fixture.assert_file_exists(
        "src/production/config.txt",
        "File should be deployed to prod path with variable substitution",
    );
}

#[test]
fn test_dest_with_package_variables() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a test file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_pkg_config"), "pkg config\n")
        .expect("Failed to create test file");

    // Add package with package-level variables
    let mut config = fixture.get_config();
    let mut pkg_variables = toml::Table::new();
    pkg_variables.insert(
        "PKG_DIR".to_string(),
        toml::Value::String("package_dir".to_string()),
    );

    let package = dotr_dear::package::Package {
        name: "f_pkg_config".to_string(),
        src: "dotfiles/f_pkg_config".to_string(),
        dest: "src/{{ PKG_DIR }}/config.txt".to_string(),
        variables: pkg_variables,
        ..Default::default()
    };
    config.packages.insert("f_pkg_config".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_pkg_config".to_string()]), None);

    // Verify
    fixture.assert_file_exists(
        "src/package_dir/config.txt",
        "File should be deployed using package-level variables",
    );
}

#[test]
fn test_dest_with_profile_variables() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a profile with variables
    let mut config = fixture.get_config();
    let mut profile = dotr_dear::profile::Profile::new("custom");
    profile.variables.insert(
        "PROFILE_DIR".to_string(),
        toml::Value::String("custom_dir".to_string()),
    );
    config.profiles.insert("custom".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create a test file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_profile_config"),
        "profile config\n",
    )
    .expect("Failed to create test file");

    // Add package using profile variable
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_profile_config".to_string(),
        src: "dotfiles/f_profile_config".to_string(),
        dest: "src/{{ PROFILE_DIR }}/config.txt".to_string(),
        ..Default::default()
    };
    config
        .packages
        .insert("f_profile_config".to_string(), package);

    // Add package to profile dependencies
    config
        .profiles
        .get_mut("custom")
        .unwrap()
        .dependencies
        .push("f_profile_config".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy with custom profile
    fixture.deploy(
        Some(vec!["f_profile_config".to_string()]),
        Some("custom".to_string()),
    );

    // Verify
    fixture.assert_file_exists(
        "src/custom_dir/config.txt",
        "File should be deployed using profile-level variables",
    );
}

#[test]
fn test_dest_with_variable_priority() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add global variable
    let mut config = fixture.get_config();
    config
        .variables
        .insert("DIR".to_string(), toml::Value::String("global".to_string()));
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create a test file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_priority_test"),
        "priority test\n",
    )
    .expect("Failed to create test file");

    // Add package without package variable
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_priority_test".to_string(),
        src: "dotfiles/f_priority_test".to_string(),
        dest: "src/{{ DIR }}/config.txt".to_string(),
        ..Default::default()
    };
    config
        .packages
        .insert("f_priority_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy with default profile (should use global variable)
    fixture.deploy(Some(vec!["f_priority_test".to_string()]), None);

    // Verify global variable was used
    fixture.assert_file_exists(
        "src/global/config.txt",
        "Global variable should be used when no override exists",
    );

    // Clean up for next test
    std::fs::remove_file(fixture.cwd.join("src/global/config.txt")).expect("Failed to cleanup");

    // Create profile with same variable (should override global)
    let mut config = fixture.get_config();
    let mut profile = dotr_dear::profile::Profile::new("test_profile");
    profile.variables.insert(
        "DIR".to_string(),
        toml::Value::String("profile".to_string()),
    );
    profile.dependencies.push("f_priority_test".to_string());
    config.profiles.insert("test_profile".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy with profile
    fixture.deploy(
        Some(vec!["f_priority_test".to_string()]),
        Some("test_profile".to_string()),
    );

    // Verify profile variable overrides global (profile > package > global)
    fixture.assert_file_exists(
        "src/profile/config.txt",
        "Profile variable should override global variable",
    );
}

#[test]
fn test_dest_with_conditional_templating() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add variable for conditional
    let mut config = fixture.get_config();
    config
        .variables
        .insert("USE_CUSTOM".to_string(), toml::Value::Boolean(true));
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create a test file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_conditional"),
        "conditional config\n",
    )
    .expect("Failed to create test file");

    // Add package with conditional in dest
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_conditional".to_string(),
        src: "dotfiles/f_conditional".to_string(),
        dest: "src/{% if USE_CUSTOM %}custom{% else %}default{% endif %}/config.txt".to_string(),
        ..Default::default()
    };
    config.packages.insert("f_conditional".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_conditional".to_string()]), None);

    // Verify conditional was evaluated
    fixture.assert_file_exists(
        "src/custom/config.txt",
        "File should be deployed to custom path based on conditional",
    );
}

#[test]
fn test_dest_directory_with_templated_path() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add variable
    let mut config = fixture.get_config();
    config.variables.insert(
        "CONFIG_BASE".to_string(),
        toml::Value::String("myconfigs".to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create a directory with files
    fs::create_dir_all(fixture.cwd.join("dotfiles/d_config_dir/subdir"))
        .expect("Failed to create test directory");
    fs::write(
        fixture.cwd.join("dotfiles/d_config_dir/file1.txt"),
        "file1\n",
    )
    .expect("Failed to create file1");
    fs::write(
        fixture.cwd.join("dotfiles/d_config_dir/subdir/file2.txt"),
        "file2\n",
    )
    .expect("Failed to create file2");

    // Add package with templated dest for directory
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "d_config_dir".to_string(),
        src: "dotfiles/d_config_dir".to_string(),
        dest: "src/{{ CONFIG_BASE }}/".to_string(),
        ..Default::default()
    };
    config.packages.insert("d_config_dir".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["d_config_dir".to_string()]), None);

    // Verify all files were deployed to templated path
    fixture.assert_file_exists(
        "src/myconfigs/file1.txt",
        "Directory files should be deployed to templated path",
    );
    fixture.assert_file_exists(
        "src/myconfigs/subdir/file2.txt",
        "Nested directory files should be deployed to templated path",
    );
}

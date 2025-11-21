use std::{fs, path::PathBuf};

use dotr::{
    cli::{DeployArgs, InitArgs, run_cli},
    config::Config,
};

mod common;

const PLAYGROUND_DIR: &str = "tests/playground";

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

    fn deploy(&self, packages: Option<Vec<String>>) {
        run_cli(self.get_cli(Some(dotr::cli::Command::Deploy(DeployArgs {
            packages,
            profile: None,
        }))));
    }

    fn get_config(&self) -> Config {
        Config::from_path(&self.cwd).expect("Failed to load config")
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        common::teardown(&self.cwd);
    }
}

#[test]
fn test_package_variables_basic() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a template file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_pkg_var_test"),
        "Package var: {{ PKG_VAR }}\n",
    )
    .expect("Failed to create template");

    // Create package with variables
    let mut config = fixture.get_config();
    let mut pkg_vars = toml::Table::new();
    pkg_vars.insert(
        "PKG_VAR".to_string(),
        toml::Value::String("package_value".to_string()),
    );

    let package = dotr::package::Package {
        name: "f_pkg_var_test".to_string(),
        src: "dotfiles/f_pkg_var_test".to_string(),
        dest: "src/.pkg_var_test".to_string(),
        dependencies: None,
        variables: pkg_vars,
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
    };
    config
        .packages
        .insert("f_pkg_var_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_pkg_var_test".to_string()]));

    // Verify the file was deployed with package variable
    let content = fs::read_to_string(fixture.cwd.join("src/.pkg_var_test"))
        .expect("Failed to read deployed file");
    assert!(
        content.contains("Package var: package_value"),
        "Package variable should be used in template"
    );
}

#[test]
fn test_package_variables_override_config_variables() {
    let fixture = TestFixture::new();
    fixture.init();

    // Remove any existing .uservariables.toml to avoid interference
    let uservars_path = fixture.cwd.join(".uservariables.toml");
    if uservars_path.exists() {
        fs::remove_file(&uservars_path).ok();
    }

    // Add config variable
    let mut config = fixture.get_config();
    config.variables.insert(
        "MY_VAR".to_string(),
        toml::Value::String("config_value".to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create a template file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_override_test"),
        "Value: {{ MY_VAR }}\n",
    )
    .expect("Failed to create template");

    // Create package with variable that overrides config
    let mut config = fixture.get_config();
    let mut pkg_vars = toml::Table::new();
    pkg_vars.insert(
        "MY_VAR".to_string(),
        toml::Value::String("package_value".to_string()),
    );

    let package = dotr::package::Package {
        name: "f_override_test".to_string(),
        src: "dotfiles/f_override_test".to_string(),
        dest: "src/.override_test".to_string(),
        dependencies: None,
        variables: pkg_vars,
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
    };
    config
        .packages
        .insert("f_override_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_override_test".to_string()]));

    // Verify package variable overrode config variable
    let content = fs::read_to_string(fixture.cwd.join("src/.override_test"))
        .expect("Failed to read deployed file");
    assert!(
        content.contains("Value: package_value"),
        "Package variable should override config variable"
    );
}

#[test]
fn test_package_variables_overridden_by_user_variables() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create .uservariables.toml
    fs::write(
        fixture.cwd.join(".uservariables.toml"),
        r#"
MY_VAR = "user_value"
"#,
    )
    .expect("Failed to create .uservariables.toml");

    // Create a template file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_user_override_test"),
        "Value: {{ MY_VAR }}\n",
    )
    .expect("Failed to create template");

    // Create package with variable
    let mut config = fixture.get_config();
    let mut pkg_vars = toml::Table::new();
    pkg_vars.insert(
        "MY_VAR".to_string(),
        toml::Value::String("package_value".to_string()),
    );

    let package = dotr::package::Package {
        name: "f_user_override_test".to_string(),
        src: "dotfiles/f_user_override_test".to_string(),
        dest: "src/.user_override_test".to_string(),
        dependencies: None,
        variables: pkg_vars,
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
    };
    config
        .packages
        .insert("f_user_override_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_user_override_test".to_string()]));

    // Verify user variable overrode package variable
    let content = fs::read_to_string(fixture.cwd.join("src/.user_override_test"))
        .expect("Failed to read deployed file");
    assert!(
        content.contains("Value: user_value"),
        "User variable should override package variable"
    );
}

#[test]
fn test_package_variables_with_nested_structures() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a template file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_nested_test"),
        "Host: {{ database.host }}\nPort: {{ database.port }}\n",
    )
    .expect("Failed to create template");

    // Create package with nested variables
    let mut config = fixture.get_config();
    let mut pkg_vars = toml::Table::new();
    let mut db_config = toml::map::Map::new();
    db_config.insert(
        "host".to_string(),
        toml::Value::String("localhost".to_string()),
    );
    db_config.insert("port".to_string(), toml::Value::Integer(5432));
    pkg_vars.insert("database".to_string(), toml::Value::Table(db_config));

    let package = dotr::package::Package {
        name: "f_nested_test".to_string(),
        src: "dotfiles/f_nested_test".to_string(),
        dest: "src/.nested_test".to_string(),
        dependencies: None,
        variables: pkg_vars,
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
    };
    config.packages.insert("f_nested_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_nested_test".to_string()]));

    // Verify nested variables were used
    let content = fs::read_to_string(fixture.cwd.join("src/.nested_test"))
        .expect("Failed to read deployed file");
    assert!(
        content.contains("Host: localhost"),
        "Nested host should be rendered"
    );
    assert!(
        content.contains("Port: 5432"),
        "Nested port should be rendered"
    );
}

#[test]
fn test_package_variables_persist_after_save() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create package with variables
    let mut config = fixture.get_config();
    let mut pkg_vars = toml::Table::new();
    pkg_vars.insert(
        "VAR1".to_string(),
        toml::Value::String("value1".to_string()),
    );
    pkg_vars.insert(
        "VAR2".to_string(),
        toml::Value::String("value2".to_string()),
    );

    let package = dotr::package::Package {
        name: "test_package".to_string(),
        src: "dotfiles/test".to_string(),
        dest: "src/.test".to_string(),
        dependencies: None,
        variables: pkg_vars,
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
    };
    config.packages.insert("test_package".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload config and verify variables persist
    let reloaded_config = fixture.get_config();
    let pkg = reloaded_config
        .packages
        .get("test_package")
        .expect("Package should exist");

    assert_eq!(
        pkg.variables.get("VAR1"),
        Some(&toml::Value::String("value1".to_string()))
    );
    assert_eq!(
        pkg.variables.get("VAR2"),
        Some(&toml::Value::String("value2".to_string()))
    );
}

#[test]
fn test_package_variables_priority_order() {
    let fixture = TestFixture::new();
    fixture.init();

    // Set up all three levels: config, package, user
    let mut config = fixture.get_config();
    config.variables.insert(
        "TEST_VAR".to_string(),
        toml::Value::String("config_value".to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create .uservariables.toml
    fs::write(
        fixture.cwd.join(".uservariables.toml"),
        r#"
TEST_VAR = "user_value"
"#,
    )
    .expect("Failed to create .uservariables.toml");

    // Create a template file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_priority_test"),
        "Value: {{ TEST_VAR }}\n",
    )
    .expect("Failed to create template");

    // Create package with variable
    let mut config = fixture.get_config();
    let mut pkg_vars = toml::Table::new();
    pkg_vars.insert(
        "TEST_VAR".to_string(),
        toml::Value::String("package_value".to_string()),
    );

    let package = dotr::package::Package {
        name: "f_priority_test".to_string(),
        src: "dotfiles/f_priority_test".to_string(),
        dest: "src/.priority_test".to_string(),
        dependencies: None,
        variables: pkg_vars,
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
    };
    config
        .packages
        .insert("f_priority_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_priority_test".to_string()]));

    // Verify user variable has highest priority
    let content = fs::read_to_string(fixture.cwd.join("src/.priority_test"))
        .expect("Failed to read deployed file");
    assert!(
        content.contains("Value: user_value"),
        "Priority should be: user > package > config"
    );
}

#[test]
fn test_multiple_packages_with_different_variables() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create two template files
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_pkg1"),
        "Package 1: {{ PKG1_VAR }}\n",
    )
    .expect("Failed to create template 1");
    fs::write(
        fixture.cwd.join("dotfiles/f_pkg2"),
        "Package 2: {{ PKG2_VAR }}\n",
    )
    .expect("Failed to create template 2");

    // Create first package with its variables
    let mut config = fixture.get_config();
    let mut pkg1_vars = toml::Table::new();
    pkg1_vars.insert(
        "PKG1_VAR".to_string(),
        toml::Value::String("value1".to_string()),
    );

    let package1 = dotr::package::Package {
        name: "f_pkg1".to_string(),
        src: "dotfiles/f_pkg1".to_string(),
        dest: "src/.pkg1".to_string(),
        dependencies: None,
        variables: pkg1_vars,
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
    };

    // Create second package with its variables
    let mut pkg2_vars = toml::Table::new();
    pkg2_vars.insert(
        "PKG2_VAR".to_string(),
        toml::Value::String("value2".to_string()),
    );

    let package2 = dotr::package::Package {
        name: "f_pkg2".to_string(),
        src: "dotfiles/f_pkg2".to_string(),
        dest: "src/.pkg2".to_string(),
        dependencies: None,
        variables: pkg2_vars,
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: std::collections::HashMap::new(),
        skip: false,
    };

    config.packages.insert("f_pkg1".to_string(), package1);
    config.packages.insert("f_pkg2".to_string(), package2);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy both
    fixture.deploy(Some(vec!["f_pkg1".to_string(), "f_pkg2".to_string()]));

    // Verify each package used its own variables
    let content1 =
        fs::read_to_string(fixture.cwd.join("src/.pkg1")).expect("Failed to read deployed file 1");
    let content2 =
        fs::read_to_string(fixture.cwd.join("src/.pkg2")).expect("Failed to read deployed file 2");

    assert!(
        content1.contains("Package 1: value1"),
        "Package 1 should use its own variable"
    );
    assert!(
        content2.contains("Package 2: value2"),
        "Package 2 should use its own variable"
    );
}

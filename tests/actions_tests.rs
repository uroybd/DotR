use std::{collections::HashMap, fs, path::PathBuf};

use dotr::{
    cli::{DeployUpdateArgs, InitArgs, run_cli},
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
        run_cli(self.get_cli(Some(dotr::cli::Command::Init(InitArgs {})))).expect("Init failed");

        // Set SHELL to /bin/sh in config for consistent test execution
        let mut config = self.get_config();
        config.variables.insert(
            "SHELL".to_string(),
            toml::Value::String("/bin/sh".to_string()),
        );
        config.save(&self.cwd).expect("Failed to save config");
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
fn test_pre_action_basic() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create src directory
    fs::create_dir_all(fixture.cwd.join("src")).expect("Failed to create src dir");

    // Create a simple template file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_pre_action_test"),
        "Test content\n",
    )
    .expect("Failed to create file");

    // Create package with pre_action that creates a marker file
    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "f_pre_action_test".to_string(),
        src: "dotfiles/f_pre_action_test".to_string(),
        dest: "src/.pre_action_test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec!["touch src/pre_action_marker.txt".to_string()],
        post_actions: Vec::new(),
        targets: HashMap::new(),
        skip: false,
    };
    config
        .packages
        .insert("f_pre_action_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_pre_action_test".to_string()]));

    // Verify pre-action was executed (marker file exists)
    assert!(
        fixture.cwd.join("src/pre_action_marker.txt").exists(),
        "Pre-action marker file should exist"
    );

    // Verify main file was deployed
    assert!(
        fixture.cwd.join("src/.pre_action_test").exists(),
        "Main file should be deployed"
    );
}

#[test]
fn test_post_action_basic() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a simple template file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_post_action_test"),
        "Test content\n",
    )
    .expect("Failed to create file");

    // Create package with post_action that creates a marker file
    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "f_post_action_test".to_string(),
        src: "dotfiles/f_post_action_test".to_string(),
        dest: "src/.post_action_test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: Vec::new(),
        post_actions: vec!["touch src/post_action_marker.txt".to_string()],
        targets: HashMap::new(),
        skip: false,
    };
    config
        .packages
        .insert("f_post_action_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_post_action_test".to_string()]));

    // Verify post-action was executed (marker file exists)
    assert!(
        fixture.cwd.join("src/post_action_marker.txt").exists(),
        "Post-action marker file should exist"
    );

    // Verify main file was deployed
    assert!(
        fixture.cwd.join("src/.post_action_test").exists(),
        "Main file should be deployed"
    );
}

#[test]
fn test_pre_and_post_actions_together() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a simple template file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_both_actions_test"),
        "Test content\n",
    )
    .expect("Failed to create file");

    // Create package with both pre and post actions
    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "f_both_actions_test".to_string(),
        src: "dotfiles/f_both_actions_test".to_string(),
        dest: "src/.both_actions_test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec!["echo 'pre' > src/both_pre_marker.txt".to_string()],
        post_actions: vec!["echo 'post' > src/both_post_marker.txt".to_string()],
        targets: HashMap::new(),
        skip: false,
    };
    config
        .packages
        .insert("f_both_actions_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_both_actions_test".to_string()]));

    // Verify both actions were executed
    assert!(
        fixture.cwd.join("src/both_pre_marker.txt").exists(),
        "Pre-action marker file should exist"
    );
    assert!(
        fixture.cwd.join("src/both_post_marker.txt").exists(),
        "Post-action marker file should exist"
    );

    // Verify marker content
    let pre_content =
        fs::read_to_string(fixture.cwd.join("src/both_pre_marker.txt")).expect("Failed to read");
    let post_content =
        fs::read_to_string(fixture.cwd.join("src/both_post_marker.txt")).expect("Failed to read");

    assert!(
        pre_content.contains("pre"),
        "Pre-action content should be correct"
    );
    assert!(
        post_content.contains("post"),
        "Post-action content should be correct"
    );
}

#[test]
fn test_multiple_pre_actions() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a simple template file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_multi_pre_test"),
        "Test content\n",
    )
    .expect("Failed to create file");

    // Create package with multiple pre-actions
    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "f_multi_pre_test".to_string(),
        src: "dotfiles/f_multi_pre_test".to_string(),
        dest: "src/.multi_pre_test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![
            "echo 'action1' > src/pre_action1.txt".to_string(),
            "echo 'action2' > src/pre_action2.txt".to_string(),
            "echo 'action3' > src/pre_action3.txt".to_string(),
        ],
        post_actions: Vec::new(),
        targets: HashMap::new(),
        skip: false,
    };
    config
        .packages
        .insert("f_multi_pre_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_multi_pre_test".to_string()]));

    // Verify all pre-actions were executed in order
    assert!(
        fixture.cwd.join("src/pre_action1.txt").exists(),
        "Pre-action 1 marker should exist"
    );
    assert!(
        fixture.cwd.join("src/pre_action2.txt").exists(),
        "Pre-action 2 marker should exist"
    );
    assert!(
        fixture.cwd.join("src/pre_action3.txt").exists(),
        "Pre-action 3 marker should exist"
    );

    // Verify content
    let content1 =
        fs::read_to_string(fixture.cwd.join("src/pre_action1.txt")).expect("Failed to read");
    let content2 =
        fs::read_to_string(fixture.cwd.join("src/pre_action2.txt")).expect("Failed to read");
    let content3 =
        fs::read_to_string(fixture.cwd.join("src/pre_action3.txt")).expect("Failed to read");

    assert!(content1.contains("action1"));
    assert!(content2.contains("action2"));
    assert!(content3.contains("action3"));
}

#[test]
fn test_multiple_post_actions() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a simple template file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_multi_post_test"),
        "Test content\n",
    )
    .expect("Failed to create file");

    // Create package with multiple post-actions
    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "f_multi_post_test".to_string(),
        src: "dotfiles/f_multi_post_test".to_string(),
        dest: "src/.multi_post_test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: Vec::new(),
        post_actions: vec![
            "echo 'action1' > src/post_action1.txt".to_string(),
            "echo 'action2' > src/post_action2.txt".to_string(),
            "echo 'action3' > src/post_action3.txt".to_string(),
        ],
        targets: HashMap::new(),
        skip: false,
    };
    config
        .packages
        .insert("f_multi_post_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_multi_post_test".to_string()]));

    // Verify all post-actions were executed in order
    assert!(
        fixture.cwd.join("src/post_action1.txt").exists(),
        "Post-action 1 marker should exist"
    );
    assert!(
        fixture.cwd.join("src/post_action2.txt").exists(),
        "Post-action 2 marker should exist"
    );
    assert!(
        fixture.cwd.join("src/post_action3.txt").exists(),
        "Post-action 3 marker should exist"
    );

    // Verify content
    let content1 =
        fs::read_to_string(fixture.cwd.join("src/post_action1.txt")).expect("Failed to read");
    let content2 =
        fs::read_to_string(fixture.cwd.join("src/post_action2.txt")).expect("Failed to read");
    let content3 =
        fs::read_to_string(fixture.cwd.join("src/post_action3.txt")).expect("Failed to read");

    assert!(content1.contains("action1"));
    assert!(content2.contains("action2"));
    assert!(content3.contains("action3"));
}

#[test]
fn test_actions_with_variables() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a simple template file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_action_var_test"),
        "Test content\n",
    )
    .expect("Failed to create file");

    // Create package with variables and actions that use them
    let mut config = fixture.get_config();
    let mut pkg_vars = toml::Table::new();
    pkg_vars.insert(
        "ACTION_VAR".to_string(),
        toml::Value::String("variable_value".to_string()),
    );

    let package = dotr::package::Package {
        name: "f_action_var_test".to_string(),
        src: "dotfiles/f_action_var_test".to_string(),
        dest: "src/.action_var_test".to_string(),
        dependencies: None,
        variables: pkg_vars,
        pre_actions: vec!["echo '{{ ACTION_VAR }}' > src/action_var_marker.txt".to_string()],
        post_actions: Vec::new(),
        targets: HashMap::new(),
        skip: false,
    };
    config
        .packages
        .insert("f_action_var_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_action_var_test".to_string()]));

    // Verify action used the variable
    let content = fs::read_to_string(fixture.cwd.join("src/action_var_marker.txt"))
        .expect("Failed to read marker");
    assert!(
        content.contains("variable_value"),
        "Action should use package variable"
    );
}

#[test]
fn test_actions_persist_after_save() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create package with actions
    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "test_persist".to_string(),
        src: "dotfiles/test".to_string(),
        dest: "src/.test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec!["echo 'pre1'".to_string(), "echo 'pre2'".to_string()],
        post_actions: vec!["echo 'post1'".to_string(), "echo 'post2'".to_string()],
        targets: HashMap::new(),
        skip: false,
    };
    config.packages.insert("test_persist".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload config and verify actions persist
    let reloaded_config = fixture.get_config();
    let pkg = reloaded_config
        .packages
        .get("test_persist")
        .expect("Package should exist");

    assert_eq!(pkg.pre_actions.len(), 2, "Should have 2 pre-actions");
    assert_eq!(pkg.post_actions.len(), 2, "Should have 2 post-actions");
    assert_eq!(pkg.pre_actions[0], "echo 'pre1'");
    assert_eq!(pkg.pre_actions[1], "echo 'pre2'");
    assert_eq!(pkg.post_actions[0], "echo 'post1'");
    assert_eq!(pkg.post_actions[1], "echo 'post2'");
}

#[test]
fn test_actions_execution_order() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create src directory
    fs::create_dir_all(fixture.cwd.join("src")).expect("Failed to create src dir");

    // Create a simple template file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_order_test"), "Test content\n")
        .expect("Failed to create file");

    // Create package with actions that write to a log file
    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "f_order_test".to_string(),
        src: "dotfiles/f_order_test".to_string(),
        dest: "src/.order_test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![
            "echo 'pre1' > src/order_log.txt".to_string(),
            "echo 'pre2' >> src/order_log.txt".to_string(),
        ],
        post_actions: vec![
            "echo 'post1' >> src/order_log.txt".to_string(),
            "echo 'post2' >> src/order_log.txt".to_string(),
        ],
        targets: HashMap::new(),
        skip: false,
    };
    config.packages.insert("f_order_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_order_test".to_string()]));

    // Verify execution order
    let log_content =
        fs::read_to_string(fixture.cwd.join("src/order_log.txt")).expect("Failed to read log");
    let lines: Vec<&str> = log_content.lines().collect();

    assert_eq!(lines.len(), 4, "Should have 4 log entries");
    assert!(lines[0].contains("pre1"), "First should be pre1");
    assert!(lines[1].contains("pre2"), "Second should be pre2");
    assert!(lines[2].contains("post1"), "Third should be post1");
    assert!(lines[3].contains("post2"), "Fourth should be post2");
}

#[test]
fn test_empty_actions_dont_fail() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a simple template file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_no_actions_test"),
        "Test content\n",
    )
    .expect("Failed to create file");

    // Create package with no actions
    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "f_no_actions_test".to_string(),
        src: "dotfiles/f_no_actions_test".to_string(),
        dest: "src/.no_actions_test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: Vec::new(),
        post_actions: Vec::new(),
        targets: HashMap::new(),
        skip: false,
    };
    config
        .packages
        .insert("f_no_actions_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy should succeed without actions
    fixture.deploy(Some(vec!["f_no_actions_test".to_string()]));

    // Verify main file was deployed
    assert!(
        fixture.cwd.join("src/.no_actions_test").exists(),
        "Main file should be deployed"
    );
}

#[test]
fn test_actions_with_complex_commands() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a simple template file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_complex_test"),
        "Test content\n",
    )
    .expect("Failed to create file");

    // Create package with complex shell commands
    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "f_complex_test".to_string(),
        src: "dotfiles/f_complex_test".to_string(),
        dest: "src/.complex_test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec!["mkdir -p src/nested/dir && touch src/nested/dir/file.txt".to_string()],
        post_actions: vec![
            "test -f src/.complex_test && echo 'deployed' > src/deploy_check.txt".to_string(),
        ],
        targets: HashMap::new(),
        skip: false,
    };
    config
        .packages
        .insert("f_complex_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_complex_test".to_string()]));

    // Verify complex pre-action created nested directory
    assert!(
        fixture.cwd.join("src/nested/dir/file.txt").exists(),
        "Complex pre-action should create nested structure"
    );

    // Verify complex post-action checked for deployment
    assert!(
        fixture.cwd.join("src/deploy_check.txt").exists(),
        "Complex post-action should run conditional"
    );
}

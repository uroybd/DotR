use std::{fs, path::PathBuf};

use dotr_dear::{
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

    fn get_cli(&self, command: Option<dotr_dear::cli::Command>) -> dotr_dear::cli::Cli {
        dotr_dear::cli::Cli {
            command,
            working_dir: Some(PLAYGROUND_DIR.to_string()),
        }
    }

    fn init(&self) {
        run_cli(self.get_cli(Some(dotr_dear::cli::Command::Init(InitArgs {}))))
            .expect("Init failed");

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
            self.get_cli(Some(dotr_dear::cli::Command::Deploy(DeployArgs {
                packages,
                profile: None,
                ignore_errors: false,
                clean: Some(false),
                dry_run: false,
                ..Default::default()
            }))),
        )
        .expect("Deploy failed");
    }

    fn deploy_with_args(&self, packages: Option<Vec<String>>, args: DeployArgs) {
        run_cli(
            self.get_cli(Some(dotr_dear::cli::Command::Deploy(DeployArgs {
                packages,
                profile: None,
                ignore_errors: false,
                clean: Some(false),
                dry_run: false,
                ..args
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
    let package = dotr_dear::package::Package {
        name: "f_pre_action_test".to_string(),
        src: "dotfiles/f_pre_action_test".to_string(),
        dest: "src/.pre_action_test".to_string(),
        pre_actions: vec!["touch src/pre_action_marker.txt".to_string()],
        ..Default::default()
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
    let package = dotr_dear::package::Package {
        name: "f_post_action_test".to_string(),
        src: "dotfiles/f_post_action_test".to_string(),
        dest: "src/.post_action_test".to_string(),
        post_actions: vec!["touch src/post_action_marker.txt".to_string()],
        ..Default::default()
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
    let package = dotr_dear::package::Package {
        name: "f_both_actions_test".to_string(),
        src: "dotfiles/f_both_actions_test".to_string(),
        dest: "src/.both_actions_test".to_string(),
        pre_actions: vec!["echo 'pre' > src/both_pre_marker.txt".to_string()],
        post_actions: vec!["echo 'post' > src/both_post_marker.txt".to_string()],
        ..Default::default()
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
    let package = dotr_dear::package::Package {
        name: "f_multi_pre_test".to_string(),
        src: "dotfiles/f_multi_pre_test".to_string(),
        dest: "src/.multi_pre_test".to_string(),
        pre_actions: vec![
            "echo 'action1' > src/pre_action1.txt".to_string(),
            "echo 'action2' > src/pre_action2.txt".to_string(),
            "echo 'action3' > src/pre_action3.txt".to_string(),
        ],
        ..Default::default()
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
    let package = dotr_dear::package::Package {
        name: "f_multi_post_test".to_string(),
        src: "dotfiles/f_multi_post_test".to_string(),
        dest: "src/.multi_post_test".to_string(),
        post_actions: vec![
            "echo 'action1' > src/post_action1.txt".to_string(),
            "echo 'action2' > src/post_action2.txt".to_string(),
            "echo 'action3' > src/post_action3.txt".to_string(),
        ],
        ..Default::default()
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

    let package = dotr_dear::package::Package {
        name: "f_action_var_test".to_string(),
        src: "dotfiles/f_action_var_test".to_string(),
        dest: "src/.action_var_test".to_string(),
        pre_actions: vec!["echo '{{ ACTION_VAR }}' > src/action_var_marker.txt".to_string()],
        variables: pkg_vars,
        ..Default::default()
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
    let package = dotr_dear::package::Package {
        name: "test_persist".to_string(),
        src: "dotfiles/test".to_string(),
        dest: "src/.test".to_string(),
        pre_actions: vec!["echo 'pre1'".to_string(), "echo 'pre2'".to_string()],
        post_actions: vec!["echo 'post1'".to_string(), "echo 'post2'".to_string()],
        ..Default::default()
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

    // Create a directory with files instead of single file
    fs::create_dir_all(fixture.cwd.join("dotfiles/f_order_test"))
        .expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_order_test/file1.txt"),
        "Test content\n",
    )
    .expect("Failed to create file");

    // Create existing dest file with different content to ensure deployment happens
    let dest_path = fixture.cwd.join("src/.order_test");
    if dest_path.exists() && dest_path.is_file() {
        fs::remove_file(&dest_path).expect("Failed to remove file");
    }
    if !dest_path.exists() {
        fs::create_dir_all(&dest_path).expect("Failed to create dest dir");
    }
    fs::write(
        fixture.cwd.join("src/.order_test/file1.txt"),
        "Old content\n",
    )
    .expect("Failed to create dest file");

    // Create package with actions that write to a log file
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_order_test".to_string(),
        src: "dotfiles/f_order_test/".to_string(),
        dest: "src/.order_test/".to_string(),
        pre_actions: vec![
            "echo 'pre1' > src/order_log.txt".to_string(),
            "echo 'pre2' >> src/order_log.txt".to_string(),
        ],
        post_actions: vec![
            "echo 'post1' >> src/order_log.txt".to_string(),
            "echo 'post2' >> src/order_log.txt".to_string(),
        ],
        ..Default::default()
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
    let package = dotr_dear::package::Package {
        name: "f_no_actions_test".to_string(),
        src: "dotfiles/f_no_actions_test".to_string(),
        dest: "src/.no_actions_test".to_string(),
        ..Default::default()
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
    let package = dotr_dear::package::Package {
        name: "f_complex_test".to_string(),
        src: "dotfiles/f_complex_test".to_string(),
        dest: "src/.complex_test".to_string(),
        pre_actions: vec!["mkdir -p src/nested/dir && touch src/nested/dir/file.txt".to_string()],
        post_actions: vec![
            "test -f src/.complex_test && echo 'deployed' > src/deploy_check.txt".to_string(),
        ],
        ..Default::default()
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

#[test]
fn test_pre_action_failure() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a simple file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_pre_fail"), "Test content\n")
        .expect("Failed to create file");

    // Create package with a failing pre-action
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_pre_fail".to_string(),
        src: "dotfiles/f_pre_fail".to_string(),
        dest: "src/.pre_fail".to_string(),
        pre_actions: vec!["false".to_string()], // This command always fails
        post_actions: vec![],
        ..Default::default()
    };
    config.packages.insert("f_pre_fail".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy should fail due to pre-action failure
    let result = run_cli(
        fixture.get_cli(Some(dotr_dear::cli::Command::Deploy(DeployArgs {
            packages: Some(vec!["f_pre_fail".to_string()]),
            profile: None,
            ignore_errors: false,
            clean: Some(false),
            dry_run: false,
            ..Default::default()
        }))),
    );

    assert!(result.is_err(), "Deploy should fail when pre-action fails");
}

#[test]
fn test_post_action_failure() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a directory with files instead of single file
    fs::create_dir_all(fixture.cwd.join("dotfiles/f_post_fail"))
        .expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_post_fail/file1.txt"),
        "Test content\n",
    )
    .expect("Failed to create file");

    // Create existing dest file with different content to ensure deployment happens
    let dest_path = fixture.cwd.join("src/.post_fail");
    if dest_path.exists() && dest_path.is_file() {
        fs::remove_file(&dest_path).expect("Failed to remove file");
    }
    if !dest_path.exists() {
        fs::create_dir_all(&dest_path).expect("Failed to create dest dir");
    }
    fs::write(
        fixture.cwd.join("src/.post_fail/file1.txt"),
        "Different content\n",
    )
    .expect("Failed to create dest file");

    // Create package with a failing post-action
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_post_fail".to_string(),
        src: "dotfiles/f_post_fail/".to_string(),
        dest: "src/.post_fail/".to_string(),
        post_actions: vec!["exit 1".to_string()], // This command exits with error
        ..Default::default()
    };
    config.packages.insert("f_post_fail".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy should fail due to post-action failure
    let result = run_cli(
        fixture.get_cli(Some(dotr_dear::cli::Command::Deploy(DeployArgs {
            packages: Some(vec!["f_post_fail".to_string()]),
            profile: None,
            ignore_errors: false,
            clean: Some(false),
            dry_run: false,
            ..Default::default()
        }))),
    );

    assert!(result.is_err(), "Deploy should fail when post-action fails");
}

#[test]
fn test_action_with_nonexistent_command() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a simple file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_bad_cmd"), "Test content\n")
        .expect("Failed to create file");

    // Create package with a non-existent command
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_bad_cmd".to_string(),
        src: "dotfiles/f_bad_cmd".to_string(),
        dest: "src/.bad_cmd".to_string(),
        pre_actions: vec!["this_command_does_not_exist_12345".to_string()],
        ..Default::default()
    };
    config.packages.insert("f_bad_cmd".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy should fail due to non-existent command
    let result = run_cli(
        fixture.get_cli(Some(dotr_dear::cli::Command::Deploy(DeployArgs {
            packages: Some(vec!["f_bad_cmd".to_string()]),
            profile: None,
            ignore_errors: false,
            clean: Some(false),
            dry_run: false,
            ..Default::default()
        }))),
    );

    assert!(
        result.is_err(),
        "Deploy should fail when action uses non-existent command"
    );
}

#[test]
fn test_action_failure_with_error_message() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a simple file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_err_msg"), "Test content\n")
        .expect("Failed to create file");

    // Create package with an action that fails with error message
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_err_msg".to_string(),
        src: "dotfiles/f_err_msg".to_string(),
        dest: "src/.err_msg".to_string(),
        pre_actions: vec!["echo 'Error occurred' >&2 && exit 42".to_string()],
        ..Default::default()
    };
    config.packages.insert("f_err_msg".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy should fail with exit code 42
    let result = run_cli(
        fixture.get_cli(Some(dotr_dear::cli::Command::Deploy(DeployArgs {
            packages: Some(vec!["f_err_msg".to_string()]),
            profile: None,
            ignore_errors: false,
            clean: Some(false),
            dry_run: false,
            ..Default::default()
        }))),
    );

    assert!(
        result.is_err(),
        "Deploy should fail when action exits with non-zero code"
    );
}

#[test]
fn test_skip_pre_actions_skips_only_pre() {
    let fixture = TestFixture::new();
    fixture.init();

    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_skip_pre_test"),
        "Test content\n",
    )
    .expect("Failed to create file");

    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_skip_pre_test".to_string(),
        src: "dotfiles/f_skip_pre_test".to_string(),
        dest: "src/.skip_pre_test".to_string(),
        pre_actions: vec!["touch src/skip_pre_marker.txt".to_string()],
        post_actions: vec!["touch src/skip_pre_post_marker.txt".to_string()],
        ..Default::default()
    };
    config
        .packages
        .insert("f_skip_pre_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Guard against leftover marker files from prior runs polluting the assertion below.
    let _ = fs::remove_file(fixture.cwd.join("src/skip_pre_marker.txt"));

    fixture.deploy_with_args(
        Some(vec!["f_skip_pre_test".to_string()]),
        DeployArgs {
            skip_pre_actions: true,
            ..Default::default()
        },
    );

    assert!(
        !fixture.cwd.join("src/skip_pre_marker.txt").exists(),
        "Pre-action marker should NOT exist when skip_pre_actions is set"
    );
    assert!(
        fixture.cwd.join("src/skip_pre_post_marker.txt").exists(),
        "Post-action marker should still exist when only skip_pre_actions is set"
    );
    assert!(
        fixture.cwd.join("src/.skip_pre_test").exists(),
        "Main file should still be deployed"
    );
}

#[test]
fn test_skip_post_actions_skips_only_post() {
    let fixture = TestFixture::new();
    fixture.init();

    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_skip_post_test"),
        "Test content\n",
    )
    .expect("Failed to create file");

    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_skip_post_test".to_string(),
        src: "dotfiles/f_skip_post_test".to_string(),
        dest: "src/.skip_post_test".to_string(),
        pre_actions: vec!["touch src/skip_post_pre_marker.txt".to_string()],
        post_actions: vec!["touch src/skip_post_marker.txt".to_string()],
        ..Default::default()
    };
    config
        .packages
        .insert("f_skip_post_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Guard against leftover marker files from prior runs polluting the assertion below.
    let _ = fs::remove_file(fixture.cwd.join("src/skip_post_marker.txt"));

    fixture.deploy_with_args(
        Some(vec!["f_skip_post_test".to_string()]),
        DeployArgs {
            skip_post_actions: true,
            ..Default::default()
        },
    );

    assert!(
        fixture.cwd.join("src/skip_post_pre_marker.txt").exists(),
        "Pre-action marker should still exist when only skip_post_actions is set"
    );
    assert!(
        !fixture.cwd.join("src/skip_post_marker.txt").exists(),
        "Post-action marker should NOT exist when skip_post_actions is set"
    );
    assert!(
        fixture.cwd.join("src/.skip_post_test").exists(),
        "Main file should still be deployed"
    );
}

#[test]
fn test_skip_actions_skips_both_pre_and_post() {
    let fixture = TestFixture::new();
    fixture.init();

    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_skip_all_test"),
        "Test content\n",
    )
    .expect("Failed to create file");

    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_skip_all_test".to_string(),
        src: "dotfiles/f_skip_all_test".to_string(),
        dest: "src/.skip_all_test".to_string(),
        pre_actions: vec!["touch src/skip_all_pre_marker.txt".to_string()],
        post_actions: vec!["touch src/skip_all_post_marker.txt".to_string()],
        ..Default::default()
    };
    config
        .packages
        .insert("f_skip_all_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Guard against leftover marker files from prior runs polluting the assertions below.
    let _ = fs::remove_file(fixture.cwd.join("src/skip_all_pre_marker.txt"));
    let _ = fs::remove_file(fixture.cwd.join("src/skip_all_post_marker.txt"));

    fixture.deploy_with_args(
        Some(vec!["f_skip_all_test".to_string()]),
        DeployArgs {
            skip_actions: true,
            ..Default::default()
        },
    );

    assert!(
        !fixture.cwd.join("src/skip_all_pre_marker.txt").exists(),
        "Pre-action marker should NOT exist when skip_actions is set"
    );
    assert!(
        !fixture.cwd.join("src/skip_all_post_marker.txt").exists(),
        "Post-action marker should NOT exist when skip_actions is set"
    );
    assert!(
        fixture.cwd.join("src/.skip_all_test").exists(),
        "Main file should still be deployed even when actions are skipped"
    );
}

#[test]
fn test_actions_run_by_default_without_skip_flags() {
    let fixture = TestFixture::new();
    fixture.init();

    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_no_skip_test"),
        "Test content\n",
    )
    .expect("Failed to create file");

    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_no_skip_test".to_string(),
        src: "dotfiles/f_no_skip_test".to_string(),
        dest: "src/.no_skip_test".to_string(),
        pre_actions: vec!["touch src/no_skip_pre_marker.txt".to_string()],
        post_actions: vec!["touch src/no_skip_post_marker.txt".to_string()],
        ..Default::default()
    };
    config
        .packages
        .insert("f_no_skip_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    fixture.deploy_with_args(
        Some(vec!["f_no_skip_test".to_string()]),
        DeployArgs::default(),
    );

    assert!(
        fixture.cwd.join("src/no_skip_pre_marker.txt").exists(),
        "Pre-action marker should exist when no skip flags are set"
    );
    assert!(
        fixture.cwd.join("src/no_skip_post_marker.txt").exists(),
        "Post-action marker should exist when no skip flags are set"
    );
}

#[test]
fn test_pre_actions_run_once_for_multi_file_directory_package() {
    let fixture = TestFixture::new();
    fixture.init();

    // A directory package with several files - pre_actions must run exactly
    // once per deploy, not once per file walked.
    fs::create_dir_all(fixture.cwd.join("dotfiles/f_multi_file_pre_test"))
        .expect("Failed to create dotfiles dir");
    for name in ["a.txt", "b.txt", "c.txt", "d.txt"] {
        fs::write(
            fixture
                .cwd
                .join("dotfiles/f_multi_file_pre_test")
                .join(name),
            "content\n",
        )
        .expect("Failed to create file");
    }

    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_multi_file_pre_test".to_string(),
        src: "dotfiles/f_multi_file_pre_test/".to_string(),
        dest: "src/.multi_file_pre_test/".to_string(),
        pre_actions: vec!["echo 'ran' >> src/pre_run_count.txt".to_string()],
        ..Default::default()
    };
    config
        .packages
        .insert("f_multi_file_pre_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Guard against a leftover log from a prior run polluting the count below;
    // common::teardown doesn't know about this ad-hoc filename.
    let _ = fs::remove_file(fixture.cwd.join("src/pre_run_count.txt"));

    fixture.deploy(Some(vec!["f_multi_file_pre_test".to_string()]));

    let log_content = fs::read_to_string(fixture.cwd.join("src/pre_run_count.txt"))
        .expect("Failed to read pre-action log");
    let run_count = log_content.lines().count();
    assert_eq!(
        run_count, 1,
        "pre_actions should run exactly once per deploy, regardless of how many \
         files are in the package (ran {} times)",
        run_count
    );
}

#[test]
fn test_platform_and_profile_actions_wrap_the_deploy() {
    let fixture = TestFixture::new();
    fixture.init();

    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_wrapped_deploy"), "content\n")
        .expect("Failed to create file");

    let mut config = fixture.get_config();

    let mut platform = dotr_dear::platform::Platform::new("macos");
    platform.pre_actions = vec!["printf 'platform-pre\\n' >> src/wrap_log.txt".to_string()];
    platform.post_actions = vec!["printf 'platform-post\\n' >> src/wrap_log.txt".to_string()];
    config.platforms.insert("macos".to_string(), platform);

    let mut profile = dotr_dear::profile::Profile::new("mac");
    profile.platform = Some("macos".to_string());
    profile.pre_actions = vec!["printf 'profile-pre\\n' >> src/wrap_log.txt".to_string()];
    profile.post_actions = vec!["printf 'profile-post\\n' >> src/wrap_log.txt".to_string()];
    config.profiles.insert("mac".to_string(), profile);

    let package = dotr_dear::package::Package {
        name: "f_wrapped_deploy".to_string(),
        src: "dotfiles/f_wrapped_deploy".to_string(),
        dest: "src/.wrapped_deploy".to_string(),
        pre_actions: vec!["printf 'package-pre\\n' >> src/wrap_log.txt".to_string()],
        post_actions: vec!["printf 'package-post\\n' >> src/wrap_log.txt".to_string()],
        ..Default::default()
    };
    config
        .packages
        .insert("f_wrapped_deploy".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    let _ = fs::remove_file(fixture.cwd.join("src/wrap_log.txt"));

    run_cli(dotr_dear::cli::Cli {
        command: Some(dotr_dear::cli::Command::Deploy(DeployArgs {
            packages: Some(vec!["f_wrapped_deploy".to_string()]),
            profile: Some("mac".to_string()),
            clean: Some(false),
            ..Default::default()
        })),
        working_dir: Some(PLAYGROUND_DIR.to_string()),
    })
    .expect("Deploy failed");

    let log =
        fs::read_to_string(fixture.cwd.join("src/wrap_log.txt")).expect("Failed to read wrap log");
    assert_eq!(
        log, "platform-pre\nprofile-pre\npackage-pre\npackage-post\nprofile-post\nplatform-post\n",
        "platform actions wrap profile actions, which wrap the package's own actions"
    );

    assert!(
        fixture.cwd.join("src/.wrapped_deploy").exists(),
        "the package itself should still be deployed"
    );
}

#[test]
fn test_skip_actions_also_skips_platform_and_profile_actions() {
    let fixture = TestFixture::new();
    fixture.init();

    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_skip_wrap_deploy"), "content\n")
        .expect("Failed to create file");

    let mut config = fixture.get_config();

    let mut platform = dotr_dear::platform::Platform::new("macos");
    platform.pre_actions = vec!["touch src/skip_wrap_platform_pre.txt".to_string()];
    platform.post_actions = vec!["touch src/skip_wrap_platform_post.txt".to_string()];
    config.platforms.insert("macos".to_string(), platform);

    let mut profile = dotr_dear::profile::Profile::new("mac");
    profile.platform = Some("macos".to_string());
    profile.pre_actions = vec!["touch src/skip_wrap_profile_pre.txt".to_string()];
    profile.post_actions = vec!["touch src/skip_wrap_profile_post.txt".to_string()];
    config.profiles.insert("mac".to_string(), profile);

    let package = dotr_dear::package::Package {
        name: "f_skip_wrap_deploy".to_string(),
        src: "dotfiles/f_skip_wrap_deploy".to_string(),
        dest: "src/.skip_wrap_deploy".to_string(),
        ..Default::default()
    };
    config
        .packages
        .insert("f_skip_wrap_deploy".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    for marker in [
        "skip_wrap_platform_pre.txt",
        "skip_wrap_platform_post.txt",
        "skip_wrap_profile_pre.txt",
        "skip_wrap_profile_post.txt",
    ] {
        let _ = fs::remove_file(fixture.cwd.join("src").join(marker));
    }

    run_cli(dotr_dear::cli::Cli {
        command: Some(dotr_dear::cli::Command::Deploy(DeployArgs {
            packages: Some(vec!["f_skip_wrap_deploy".to_string()]),
            profile: Some("mac".to_string()),
            clean: Some(false),
            skip_actions: true,
            ..Default::default()
        })),
        working_dir: Some(PLAYGROUND_DIR.to_string()),
    })
    .expect("Deploy failed");

    for marker in [
        "skip_wrap_platform_pre.txt",
        "skip_wrap_platform_post.txt",
        "skip_wrap_profile_pre.txt",
        "skip_wrap_profile_post.txt",
    ] {
        assert!(
            !fixture.cwd.join("src").join(marker).exists(),
            "--skip-actions must skip platform/profile action '{}'",
            marker
        );
    }

    assert!(
        fixture.cwd.join("src/.skip_wrap_deploy").exists(),
        "the package itself should still be deployed"
    );
}

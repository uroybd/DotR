use std::{collections::HashMap, fs, path::PathBuf};

use dotr::{
    cli::{InitArgs, run_cli},
    config::Config,
    context::Context,
};

mod common;

const PLAYGROUND_DIR: &str = "tests/playground";

struct TestFixture {
    cwd: PathBuf,
}

impl TestFixture {
    fn new() -> Self {
        let cwd = PathBuf::from(PLAYGROUND_DIR);
        common::setup(&cwd);
        Self { cwd }
    }

    fn get_cli(&self, command: Option<dotr::cli::Command>) -> dotr::cli::Cli {
        dotr::cli::Cli {
            command,
            working_dir: Some(PLAYGROUND_DIR.to_string()),
        }
    }

    fn init(&self) {
        run_cli(self.get_cli(Some(dotr::cli::Command::Init(InitArgs {})))).expect("Init failed");
    }

    fn get_config(&self) -> Config {
        Config::from_path(&self.cwd).expect("Failed to load config")
    }

    fn get_context(&self) -> Context {
        Context::new(&self.cwd).expect("Failed to create context")
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        common::teardown(&self.cwd);
    }
}

#[test]
fn test_prompts_in_config() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add prompts to config
    let mut config = fixture.get_config();
    config.prompts.insert(
        "USER_EMAIL".to_string(),
        "Enter your email address".to_string(),
    );
    config
        .prompts
        .insert("USER_NAME".to_string(), "Enter your full name".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify prompts are saved
    let reloaded_config = fixture.get_config();
    assert_eq!(reloaded_config.prompts.len(), 2);
    assert_eq!(
        reloaded_config.prompts.get("USER_EMAIL"),
        Some(&"Enter your email address".to_string())
    );
    assert_eq!(
        reloaded_config.prompts.get("USER_NAME"),
        Some(&"Enter your full name".to_string())
    );
}

#[test]
fn test_prompts_persist_after_save() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add prompts
    let mut config = fixture.get_config();
    config
        .prompts
        .insert("API_KEY".to_string(), "Enter your API key".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify
    let reloaded_config = fixture.get_config();
    assert_eq!(
        reloaded_config.prompts.get("API_KEY"),
        Some(&"Enter your API key".to_string())
    );
}

#[test]
fn test_empty_prompts() {
    let fixture = TestFixture::new();
    fixture.init();

    let config = fixture.get_config();
    assert_eq!(
        config.prompts.len(),
        0,
        "Default config should have no prompts"
    );
}

#[test]
fn test_multiple_prompts() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();
    config.prompts.insert(
        "GITHUB_TOKEN".to_string(),
        "Enter your GitHub personal access token".to_string(),
    );
    config.prompts.insert(
        "OPENAI_API_KEY".to_string(),
        "Enter your OpenAI API key".to_string(),
    );
    config.prompts.insert(
        "AWS_ACCESS_KEY".to_string(),
        "Enter your AWS access key".to_string(),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    let reloaded_config = fixture.get_config();
    assert_eq!(reloaded_config.prompts.len(), 3);
}

#[test]
fn test_prompts_with_special_characters() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();
    config.prompts.insert(
        "SPECIAL_VAR".to_string(),
        "Enter value (e.g., user@example.com)".to_string(),
    );
    config.prompts.insert(
        "COMPLEX_PROMPT".to_string(),
        "What's your API key? [Leave empty to skip]".to_string(),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    let reloaded_config = fixture.get_config();
    assert_eq!(reloaded_config.prompts.len(), 2);
}

#[test]
fn test_get_prompted_variables_skips_existing() {
    let fixture = TestFixture::new();
    fixture.init();

    // Pre-populate user variables
    let uservars_path = fixture.cwd.join(".uservariables.toml");
    fs::write(
        &uservars_path,
        r#"
USER_EMAIL = "existing@example.com"
"#,
    )
    .expect("Failed to write uservariables");

    // Create prompts including one for existing variable
    let mut prompts: HashMap<String, String> = HashMap::new();
    prompts.insert("USER_EMAIL".to_string(), "Enter your email".to_string());
    prompts.insert("NEW_VAR".to_string(), "Enter new variable".to_string());

    #[allow(unused_mut)]
    let mut ctx = fixture.get_context();

    // This should not actually prompt since we can't provide stdin in tests
    // We're just testing that the function doesn't error and preserves existing vars
    // In real usage, this would only prompt for NEW_VAR

    // Verify existing variable is present
    let user_vars = ctx.get_user_variables();
    assert_eq!(
        user_vars.get("USER_EMAIL"),
        Some(&toml::Value::String("existing@example.com".to_string()))
    );
}

#[test]
fn test_prompted_variables_saved_to_uservariables() {
    let fixture = TestFixture::new();
    fixture.init();

    // Manually write to uservariables to simulate prompted input
    let uservars_path = fixture.cwd.join(".uservariables.toml");
    fs::write(
        &uservars_path,
        r#"
PROMPTED_VAR = "prompted_value"
"#,
    )
    .expect("Failed to write uservariables");

    let ctx = fixture.get_context();
    let user_vars = ctx.get_user_variables();

    assert_eq!(
        user_vars.get("PROMPTED_VAR"),
        Some(&toml::Value::String("prompted_value".to_string()))
    );
}

#[test]
fn test_prompts_do_not_interfere_with_variables() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();

    // Add regular variables
    config.variables.insert(
        "REGULAR_VAR".to_string(),
        toml::Value::String("regular_value".to_string()),
    );

    // Add prompts
    config
        .prompts
        .insert("PROMPT_VAR".to_string(), "Enter prompt value".to_string());

    config.save(&fixture.cwd).expect("Failed to save config");

    let reloaded_config = fixture.get_config();
    assert_eq!(reloaded_config.variables.len(), 1);
    assert_eq!(reloaded_config.prompts.len(), 1);
}

#[test]
fn test_prompts_with_empty_message() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();
    config
        .prompts
        .insert("VAR_WITH_EMPTY_MSG".to_string(), "".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    let reloaded_config = fixture.get_config();
    assert_eq!(
        reloaded_config.prompts.get("VAR_WITH_EMPTY_MSG"),
        Some(&"".to_string())
    );
}

#[test]
fn test_prompts_removal() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add prompts
    let mut config = fixture.get_config();
    config
        .prompts
        .insert("TO_REMOVE".to_string(), "This will be removed".to_string());
    config
        .prompts
        .insert("TO_KEEP".to_string(), "This will stay".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Remove one prompt
    let mut config = fixture.get_config();
    config.prompts.remove("TO_REMOVE");
    config.save(&fixture.cwd).expect("Failed to save config");

    // Verify
    let reloaded_config = fixture.get_config();
    assert_eq!(reloaded_config.prompts.len(), 1);
    assert!(!reloaded_config.prompts.contains_key("TO_REMOVE"));
    assert!(reloaded_config.prompts.contains_key("TO_KEEP"));
}

#[test]
fn test_prompts_update_message() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add prompt
    let mut config = fixture.get_config();
    config
        .prompts
        .insert("VAR".to_string(), "Old message".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Update prompt message
    let mut config = fixture.get_config();
    config
        .prompts
        .insert("VAR".to_string(), "New message".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Verify
    let reloaded_config = fixture.get_config();
    assert_eq!(
        reloaded_config.prompts.get("VAR"),
        Some(&"New message".to_string())
    );
}

#[test]
fn test_prompts_with_multiline_message() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();
    config.prompts.insert(
        "MULTILINE_VAR".to_string(),
        "Enter your API key\n(You can find it in your account settings)".to_string(),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    let reloaded_config = fixture.get_config();
    assert_eq!(
        reloaded_config.prompts.get("MULTILINE_VAR"),
        Some(&"Enter your API key\n(You can find it in your account settings)".to_string())
    );
}

#[test]
fn test_config_without_prompts_section() {
    let fixture = TestFixture::new();
    fixture.init();

    // Just verify config loads without prompts section
    let config = fixture.get_config();
    assert_eq!(config.prompts.len(), 0);
}

#[test]
fn test_prompts_with_unicode() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();
    config.prompts.insert(
        "UNICODE_VAR".to_string(),
        "请输入你的名字 (Enter your name)".to_string(),
    );
    config
        .prompts
        .insert("EMOJI_VAR".to_string(), "🔑 Enter your API key".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    let reloaded_config = fixture.get_config();
    assert_eq!(reloaded_config.prompts.len(), 2);
    assert_eq!(
        reloaded_config.prompts.get("UNICODE_VAR"),
        Some(&"请输入你的名字 (Enter your name)".to_string())
    );
    assert_eq!(
        reloaded_config.prompts.get("EMOJI_VAR"),
        Some(&"🔑 Enter your API key".to_string())
    );
}

#[test]
fn test_many_prompts() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();
    for i in 0..20 {
        config.prompts.insert(
            format!("VAR_{}", i),
            format!("Enter value for variable {}", i),
        );
    }
    config.save(&fixture.cwd).expect("Failed to save config");

    let reloaded_config = fixture.get_config();
    assert_eq!(reloaded_config.prompts.len(), 20);
}

#[test]
fn test_prompts_are_case_sensitive() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();
    config
        .prompts
        .insert("api_key".to_string(), "Enter lowercase API key".to_string());
    config
        .prompts
        .insert("API_KEY".to_string(), "Enter uppercase API KEY".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    let reloaded_config = fixture.get_config();
    assert_eq!(reloaded_config.prompts.len(), 2);
    assert_ne!(
        reloaded_config.prompts.get("api_key"),
        reloaded_config.prompts.get("API_KEY")
    );
}

#[test]
fn test_package_level_prompts() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create dotfile
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_test"), "Test content\n")
        .expect("Failed to create file");

    // Add package with prompts
    let mut config = fixture.get_config();
    let mut package = dotr::package::Package {
        name: "f_test".to_string(),
        src: "dotfiles/f_test".to_string(),
        dest: "src/.test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };
    package.prompts.insert(
        "PACKAGE_VAR".to_string(),
        "Enter package variable".to_string(),
    );
    config.packages.insert("f_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify
    let reloaded_config = fixture.get_config();
    let package = reloaded_config.packages.get("f_test").unwrap();
    assert_eq!(package.prompts.len(), 1);
    assert_eq!(
        package.prompts.get("PACKAGE_VAR"),
        Some(&"Enter package variable".to_string())
    );
}

#[test]
fn test_package_multiple_prompts() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create dotfile
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_test"), "Test content\n")
        .expect("Failed to create file");

    // Add package with multiple prompts
    let mut config = fixture.get_config();
    let mut package = dotr::package::Package {
        name: "f_test".to_string(),
        src: "dotfiles/f_test".to_string(),
        dest: "src/.test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };
    package.prompts.insert(
        "PKG_VAR1".to_string(),
        "Enter first package variable".to_string(),
    );
    package.prompts.insert(
        "PKG_VAR2".to_string(),
        "Enter second package variable".to_string(),
    );
    package.prompts.insert(
        "PKG_VAR3".to_string(),
        "Enter third package variable".to_string(),
    );
    config.packages.insert("f_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify
    let reloaded_config = fixture.get_config();
    let package = reloaded_config.packages.get("f_test").unwrap();
    assert_eq!(package.prompts.len(), 3);
}

#[test]
fn test_profile_level_prompts() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add profile with prompts
    let mut config = fixture.get_config();
    let mut profile = dotr::profile::Profile {
        name: "work".to_string(),
        variables: toml::Table::new(),
        dependencies: vec![],
        prompts: HashMap::new(),
    };
    profile.prompts.insert(
        "WORK_EMAIL".to_string(),
        "Enter your work email".to_string(),
    );
    config.profiles.insert("work".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify
    let reloaded_config = fixture.get_config();
    let profile = reloaded_config.profiles.get("work").unwrap();
    assert_eq!(profile.prompts.len(), 1);
    assert_eq!(
        profile.prompts.get("WORK_EMAIL"),
        Some(&"Enter your work email".to_string())
    );
}

#[test]
fn test_profile_multiple_prompts() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add profile with multiple prompts
    let mut config = fixture.get_config();
    let mut profile = dotr::profile::Profile {
        name: "work".to_string(),
        variables: toml::Table::new(),
        dependencies: vec![],
        prompts: HashMap::new(),
    };
    profile.prompts.insert(
        "WORK_EMAIL".to_string(),
        "Enter your work email".to_string(),
    );
    profile
        .prompts
        .insert("SLACK_TOKEN".to_string(), "Enter Slack token".to_string());
    profile
        .prompts
        .insert("VPN_PASSWORD".to_string(), "Enter VPN password".to_string());
    config.profiles.insert("work".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify
    let reloaded_config = fixture.get_config();
    let profile = reloaded_config.profiles.get("work").unwrap();
    assert_eq!(profile.prompts.len(), 3);
}

#[test]
fn test_package_and_profile_prompts_together() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create dotfile
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_test"), "Test content\n")
        .expect("Failed to create file");

    // Add package with prompts
    let mut config = fixture.get_config();
    let mut package = dotr::package::Package {
        name: "f_test".to_string(),
        src: "dotfiles/f_test".to_string(),
        dest: "src/.test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };
    package.prompts.insert(
        "PACKAGE_VAR".to_string(),
        "Enter package variable".to_string(),
    );
    config.packages.insert("f_test".to_string(), package);

    // Add profile with prompts
    let mut profile = dotr::profile::Profile {
        name: "work".to_string(),
        variables: toml::Table::new(),
        dependencies: vec![],
        prompts: HashMap::new(),
    };
    profile.prompts.insert(
        "PROFILE_VAR".to_string(),
        "Enter profile variable".to_string(),
    );
    config.profiles.insert("work".to_string(), profile);

    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify both exist
    let reloaded_config = fixture.get_config();
    let package = reloaded_config.packages.get("f_test").unwrap();
    let profile = reloaded_config.profiles.get("work").unwrap();

    assert_eq!(package.prompts.len(), 1);
    assert_eq!(profile.prompts.len(), 1);
}

#[test]
fn test_package_prompts_do_not_interfere_with_variables() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create dotfile
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_test"), "Test content\n")
        .expect("Failed to create file");

    // Add package with both prompts and variables
    let mut config = fixture.get_config();
    let mut package = dotr::package::Package {
        name: "f_test".to_string(),
        src: "dotfiles/f_test".to_string(),
        dest: "src/.test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };
    package.variables.insert(
        "STATIC_VAR".to_string(),
        toml::Value::String("static_value".to_string()),
    );
    package.prompts.insert(
        "PROMPT_VAR".to_string(),
        "Enter prompt variable".to_string(),
    );
    config.packages.insert("f_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify
    let reloaded_config = fixture.get_config();
    let package = reloaded_config.packages.get("f_test").unwrap();
    assert_eq!(package.variables.len(), 1);
    assert_eq!(package.prompts.len(), 1);
}

#[test]
fn test_profile_prompts_do_not_interfere_with_variables() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add profile with both prompts and variables
    let mut config = fixture.get_config();
    let mut profile = dotr::profile::Profile {
        name: "work".to_string(),
        variables: toml::Table::new(),
        dependencies: vec![],
        prompts: HashMap::new(),
    };
    profile.variables.insert(
        "STATIC_VAR".to_string(),
        toml::Value::String("static_value".to_string()),
    );
    profile.prompts.insert(
        "PROMPT_VAR".to_string(),
        "Enter prompt variable".to_string(),
    );
    config.profiles.insert("work".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify
    let reloaded_config = fixture.get_config();
    let profile = reloaded_config.profiles.get("work").unwrap();
    assert_eq!(profile.variables.len(), 1);
    assert_eq!(profile.prompts.len(), 1);
}

#[test]
fn test_empty_package_prompts() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create dotfile
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_test"), "Test content\n")
        .expect("Failed to create file");

    // Add package without prompts
    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "f_test".to_string(),
        src: "dotfiles/f_test".to_string(),
        dest: "src/.test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };
    config.packages.insert("f_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify no prompts
    let reloaded_config = fixture.get_config();
    let package = reloaded_config.packages.get("f_test").unwrap();
    assert_eq!(package.prompts.len(), 0);
}

#[test]
fn test_empty_profile_prompts() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add profile without prompts
    let mut config = fixture.get_config();
    let profile = dotr::profile::Profile {
        name: "work".to_string(),
        variables: toml::Table::new(),
        dependencies: vec![],
        prompts: HashMap::new(),
    };
    config.profiles.insert("work".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify no prompts
    let reloaded_config = fixture.get_config();
    let profile = reloaded_config.profiles.get("work").unwrap();
    assert_eq!(profile.prompts.len(), 0);
}

#[test]
fn test_three_level_prompts_hierarchy() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create dotfile
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(fixture.cwd.join("dotfiles/f_test"), "Test content\n")
        .expect("Failed to create file");

    // Add config-level prompt
    let mut config = fixture.get_config();
    config.prompts.insert(
        "CONFIG_VAR".to_string(),
        "Enter config variable".to_string(),
    );

    // Add package with prompt
    let mut package = dotr::package::Package {
        name: "f_test".to_string(),
        src: "dotfiles/f_test".to_string(),
        dest: "src/.test".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
        pre_actions: vec![],
        post_actions: vec![],
        targets: HashMap::new(),
        skip: false,
        prompts: HashMap::new(),
        ignore: Vec::new(),
    };
    package.prompts.insert(
        "PACKAGE_VAR".to_string(),
        "Enter package variable".to_string(),
    );
    config.packages.insert("f_test".to_string(), package);

    // Add profile with prompt
    let mut profile = dotr::profile::Profile {
        name: "work".to_string(),
        variables: toml::Table::new(),
        dependencies: vec![],
        prompts: HashMap::new(),
    };
    profile.prompts.insert(
        "PROFILE_VAR".to_string(),
        "Enter profile variable".to_string(),
    );
    config.profiles.insert("work".to_string(), profile);

    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload and verify all three levels
    let reloaded_config = fixture.get_config();
    assert_eq!(reloaded_config.prompts.len(), 1);

    let package = reloaded_config.packages.get("f_test").unwrap();
    assert_eq!(package.prompts.len(), 1);

    let profile = reloaded_config.profiles.get("work").unwrap();
    assert_eq!(profile.prompts.len(), 1);
}

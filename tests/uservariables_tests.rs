use std::{fs, path::PathBuf};

use dotr::{
    cli::{DeployArgs, InitArgs, run_cli},
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
        run_cli(self.get_cli(Some(dotr::cli::Command::Deploy(DeployArgs { packages }))));
    }

    fn get_config(&self) -> Config {
        Config::from_path(&self.cwd)
    }

    fn get_context(&self) -> Context {
        let mut ctx = Context::new(&self.cwd);
        let config = self.get_config();
        ctx.extend_variables(config.variables.clone());
        ctx
    }

    fn get_context_variables(&self) -> toml::Table {
        self.get_context().get_context_variables()
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        common::teardown(&self.cwd);
    }
}

#[test]
fn test_uservariables_basic() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create .uservariables.toml with some variables
    let uservars_path = fixture.cwd.join(".uservariables.toml");
    fs::write(
        &uservars_path,
        r#"
SECRET_KEY = "my-secret-key"
API_TOKEN = "token-12345"
DATABASE_PASSWORD = "password123"
"#,
    )
    .expect("Failed to create .uservariables.toml");

    // Load context - should include user variables
    let ctx_vars = fixture.get_context_variables();

    assert_eq!(
        ctx_vars.get("SECRET_KEY"),
        Some(&toml::Value::String("my-secret-key".to_string()))
    );
    assert_eq!(
        ctx_vars.get("API_TOKEN"),
        Some(&toml::Value::String("token-12345".to_string()))
    );
    assert_eq!(
        ctx_vars.get("DATABASE_PASSWORD"),
        Some(&toml::Value::String("password123".to_string()))
    );
}

#[test]
fn test_uservariables_override_config_variables() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add variables to config
    let mut config = fixture.get_config();
    config
        .variables
        .insert("EDITOR".to_string(), toml::Value::String("vim".to_string()));
    config
        .variables
        .insert("THEME".to_string(), toml::Value::String("dark".to_string()));
    config.save(&fixture.cwd);

    // Create .uservariables.toml that overrides EDITOR but not THEME
    let uservars_path = fixture.cwd.join(".uservariables.toml");
    fs::write(
        &uservars_path,
        r#"
EDITOR = "nvim"
SECRET = "my-secret"
"#,
    )
    .expect("Failed to create .uservariables.toml");

    // Load context - user variables should override config variables
    let ctx_vars = fixture.get_context_variables();

    // User variables should override config variables
    assert_eq!(
        ctx_vars.get("EDITOR"),
        Some(&toml::Value::String("nvim".to_string())),
        "User variable should override config variable"
    );
    // Config variable that's not overridden should remain
    assert_eq!(
        ctx_vars.get("THEME"),
        Some(&toml::Value::String("dark".to_string())),
        "Config variable should remain if not overridden"
    );
    // User-only variable should be present
    assert_eq!(
        ctx_vars.get("SECRET"),
        Some(&toml::Value::String("my-secret".to_string())),
        "User-only variable should be present"
    );
}

#[test]
fn test_uservariables_with_nested_structures() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create .uservariables.toml with nested structures
    let uservars_path = fixture.cwd.join(".uservariables.toml");
    fs::write(
        &uservars_path,
        r#"
[database]
host = "secret-db.example.com"
password = "secret-password"
port = 5432

[api]
key = "secret-api-key"
endpoint = "https://secret.api.com"
"#,
    )
    .expect("Failed to create .uservariables.toml");

    // Load context
    let ctx_vars = fixture.get_context_variables();

    // Check nested database config
    if let Some(toml::Value::Table(db_table)) = ctx_vars.get("database") {
        assert_eq!(
            db_table.get("host"),
            Some(&toml::Value::String("secret-db.example.com".to_string()))
        );
        assert_eq!(
            db_table.get("password"),
            Some(&toml::Value::String("secret-password".to_string()))
        );
        assert_eq!(db_table.get("port"), Some(&toml::Value::Integer(5432)));
    } else {
        panic!("database should be a table");
    }

    // Check nested api config
    if let Some(toml::Value::Table(api_table)) = ctx_vars.get("api") {
        assert_eq!(
            api_table.get("key"),
            Some(&toml::Value::String("secret-api-key".to_string()))
        );
        assert_eq!(
            api_table.get("endpoint"),
            Some(&toml::Value::String("https://secret.api.com".to_string()))
        );
    } else {
        panic!("api should be a table");
    }
}

#[test]
fn test_uservariables_used_in_templates() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create .uservariables.toml
    let uservars_path = fixture.cwd.join(".uservariables.toml");
    fs::write(
        &uservars_path,
        r#"
API_KEY = "secret-key-12345"
DATABASE_NAME = "production-db"
"#,
    )
    .expect("Failed to create .uservariables.toml");

    // Create a templated file (using simpler variables without special chars)
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_env_template"),
        "# Environment Config\nAPI_KEY={{ API_KEY }}\nDATABASE_NAME={{ DATABASE_NAME }}\n",
    )
    .expect("Failed to create template");

    // Add package
    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "f_env_template".to_string(),
        src: "dotfiles/f_env_template".to_string(),
        dest: "src/.env".to_string(),
        dependencies: None,
        variables: toml::Table::new(),
    pre_actions: Vec::new(),
    post_actions: Vec::new(),
    };
    config
        .packages
        .insert("f_env_template".to_string(), package);
    config.save(&fixture.cwd);

    // Deploy
    fixture.deploy(Some(vec!["f_env_template".to_string()]));

    // Check deployed file uses user variables
    let content =
        fs::read_to_string(fixture.cwd.join("src/.env")).expect("Failed to read deployed file");

    assert!(
        content.contains("API_KEY=secret-key-12345"),
        "User variable should be used in template: {}",
        content
    );
    assert!(
        content.contains("DATABASE_NAME=production-db"),
        "User variable should be used in template: {}",
        content
    );
}

#[test]
fn test_gitignore_created_on_init() {
    let fixture = TestFixture::new();

    // Clean up any existing files
    let gitignore_path = fixture.cwd.join(".gitignore");
    if gitignore_path.exists() {
        fs::remove_file(&gitignore_path).ok();
    }

    // Initialize
    fixture.init();

    // Check that .gitignore was created
    assert!(
        gitignore_path.exists(),
        ".gitignore should be created during init"
    );

    // Check that it contains .uservariables.toml
    let gitignore_content = fs::read_to_string(&gitignore_path).expect("Failed to read .gitignore");

    assert!(
        gitignore_content.contains(".uservariables.toml"),
        ".gitignore should contain .uservariables.toml"
    );
}

#[test]
fn test_uservariables_not_saved_to_config() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create .uservariables.toml
    let uservars_path = fixture.cwd.join(".uservariables.toml");
    fs::write(
        &uservars_path,
        r#"
SECRET = "should-not-be-in-config"
"#,
    )
    .expect("Failed to create .uservariables.toml");

    // Load context (user variables get merged from .uservariables.toml)
    let ctx_vars = fixture.get_context_variables();

    // User variable should be present in context
    assert_eq!(
        ctx_vars.get("SECRET"),
        Some(&toml::Value::String("should-not-be-in-config".to_string())),
        "User variables should be present in context"
    );

    // Load config and save it
    let config = fixture.get_config();
    config.save(&fixture.cwd);

    // Read config.toml directly
    let config_content =
        fs::read_to_string(fixture.cwd.join("config.toml")).expect("Failed to read config.toml");

    // User variables should NOT be in config.toml (preserved separately in .uservariables.toml)
    assert!(
        !config_content.contains("SECRET"),
        "User variables should not be saved to config.toml"
    );
    assert!(
        !config_content.contains("should-not-be-in-config"),
        "User variables should not be saved to config.toml"
    );
}

#[test]
fn test_uservariables_with_no_file() {
    let fixture = TestFixture::new();
    fixture.init();

    // Ensure .uservariables.toml doesn't exist
    let uservars_path = fixture.cwd.join(".uservariables.toml");
    if uservars_path.exists() {
        fs::remove_file(&uservars_path).ok();
    }

    // Should load config without errors
    let config = fixture.get_config();

    // Should work fine with just config variables (no user variables)
    assert!(
        config.variables.is_empty() || !config.variables.contains_key("NONEXISTENT"),
        "Config should work without .uservariables.toml"
    );
}

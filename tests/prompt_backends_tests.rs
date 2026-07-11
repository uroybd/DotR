use std::{fs, path::PathBuf};

use dotr_dear::{
    cli::{InitArgs, run_cli},
    config::Config,
    prompt_store::PromptBackendType,
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
        // A prior interrupted run can leave this behind.
        let _ = fs::remove_file(cwd.join(".uservariables.toml"));
        Self { cwd }
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

    fn get_config(&self) -> Config {
        Config::from_path(&self.cwd).expect("Failed to load config")
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.cwd.join(".uservariables.toml"));
        common::teardown(&self.cwd);
    }
}

// ==========================
// prompt_backend: config-level default + profile-level override.
// There's deliberately no per-prompt backend — backend choice is a policy
// (repo default, or machine-specific via profile), not something that
// varies prompt by prompt.
// ==========================

#[test]
fn test_prompt_backend_defaults_to_none_and_is_omitted_from_output() {
    let fixture = TestFixture::new();
    fixture.init();

    let config = fixture.get_config();
    assert_eq!(config.prompt_backend, None);

    let raw = fs::read_to_string(fixture.cwd.join("config.toml")).unwrap();
    assert!(!raw.contains("prompt_backend"));
}

#[test]
fn test_config_level_prompt_backend_round_trips() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();
    config.prompt_backend = Some(PromptBackendType::Keychain);
    config.save(&fixture.cwd).expect("Failed to save config");

    let reloaded = fixture.get_config();
    assert_eq!(reloaded.prompt_backend, Some(PromptBackendType::Keychain));

    let raw = fs::read_to_string(fixture.cwd.join("config.toml")).unwrap();
    assert!(raw.contains("prompt_backend = \"keychain\""));
}

#[test]
fn test_profile_level_prompt_backend_round_trips() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();
    let mut profile = dotr_dear::profile::Profile::new("work");
    profile.prompt_backend = Some(PromptBackendType::Bitwarden);
    config.profiles.insert("work".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    let reloaded = fixture.get_config();
    let profile = reloaded.profiles.get("work").unwrap();
    assert_eq!(profile.prompt_backend, Some(PromptBackendType::Bitwarden));
}

#[test]
fn test_unknown_config_level_prompt_backend_is_rejected() {
    let fixture = TestFixture::new();
    fixture.init();

    fs::write(
        fixture.cwd.join("config.toml"),
        r#"
banner = false
symlink = false
prompt_backend = "not-a-real-backend"

[profiles.default]
dependencies = []
"#,
    )
    .unwrap();

    let err = Config::from_path(&fixture.cwd).unwrap_err();
    assert!(
        err.to_string().contains("not-a-real-backend"),
        "error should name the bad backend: {err}"
    );
}

#[test]
fn test_unknown_profile_level_prompt_backend_is_rejected() {
    let fixture = TestFixture::new();
    fixture.init();

    fs::write(
        fixture.cwd.join("config.toml"),
        r#"
banner = false
symlink = false

[profiles.default]
dependencies = []
prompt_backend = "not-a-real-backend"
"#,
    )
    .unwrap();

    let err = Config::from_path(&fixture.cwd).unwrap_err();
    assert!(
        err.to_string().contains("not-a-real-backend"),
        "error should name the bad backend: {err}"
    );
}

#[test]
fn test_prompts_stay_plain_strings() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();
    config
        .prompts
        .insert("PLAIN".to_string(), "Enter a value".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    let reloaded = fixture.get_config();
    assert_eq!(
        reloaded.prompts.get("PLAIN"),
        Some(&"Enter a value".to_string())
    );
}

// ==========================
// bitwarden_note config field
// ==========================

#[test]
fn test_bitwarden_note_defaults_to_none_and_is_omitted_from_output() {
    let fixture = TestFixture::new();
    fixture.init();

    let config = fixture.get_config();
    assert_eq!(config.bitwarden_note, None);

    let raw = fs::read_to_string(fixture.cwd.join("config.toml")).unwrap();
    assert!(!raw.contains("bitwarden_note"));
}

#[test]
fn test_bitwarden_note_round_trips_when_set() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();
    config.bitwarden_note = Some("my-team-secrets".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    let reloaded = fixture.get_config();
    assert_eq!(reloaded.bitwarden_note, Some("my-team-secrets".to_string()));
}

#[test]
fn test_profile_level_bitwarden_note_round_trips() {
    let fixture = TestFixture::new();
    fixture.init();

    let mut config = fixture.get_config();
    let mut profile = dotr_dear::profile::Profile::new("work");
    profile.bitwarden_note = Some("work-secrets".to_string());
    config.profiles.insert("work".to_string(), profile);
    config.save(&fixture.cwd).expect("Failed to save config");

    let reloaded = fixture.get_config();
    let profile = reloaded.profiles.get("work").unwrap();
    assert_eq!(profile.bitwarden_note, Some("work-secrets".to_string()));
}

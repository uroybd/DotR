use std::{fs, path::PathBuf};

use dotr::{
    cli::{DeployUpdateArgs, ImportArgs, InitArgs, run_cli},
    config::Config,
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

    fn import(&self, path: &str) {
        run_cli(self.get_cli(Some(dotr::cli::Command::Import(ImportArgs {
            path: path.to_string(),
            name: None,
            profile: None,
        }))))
        .expect("Import failed");
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

    fn update(&self, packages: Option<Vec<String>>) {
        run_cli(
            self.get_cli(Some(dotr::cli::Command::Update(DeployUpdateArgs {
                packages,
                profile: None,
            }))),
        )
        .expect("Update failed");
    }

    fn get_config(&self) -> Config {
        Config::from_path(&self.cwd).expect("Failed to load config")
    }

    fn write_file(&self, path: &str, content: &str) {
        let file_path = self.cwd.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file_path, content).unwrap_or_else(|_| panic!("Failed to write file: {}", path));
    }

    fn assert_file_exists(&self, path: &str, message: &str) {
        assert!(self.cwd.join(path).exists(), "{}", message);
    }

    #[allow(dead_code)]
    fn assert_file_not_exists(&self, path: &str, message: &str) {
        assert!(!self.cwd.join(path).exists(), "{}", message);
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        common::teardown(&self.cwd);
    }
}

#[test]
fn test_ignore_single_file_pattern() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create test directory with files
    fixture.write_file("src/testdir/file1.txt", "content1");
    fixture.write_file("src/testdir/file2.log", "log content");
    fixture.write_file("src/testdir/file3.txt", "content3");

    fixture.import("src/testdir/");

    // Add ignore pattern for .log files
    let mut config = fixture.get_config();
    let package = config.packages.get_mut("d_testdir").unwrap();
    package.ignore.push("*.log".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["d_testdir".to_string()]));

    // Verify .txt files are deployed but .log is ignored
    fixture.assert_file_exists("src/testdir/file1.txt", "file1.txt should be deployed");
    fixture.assert_file_exists("src/testdir/file3.txt", "file3.txt should be deployed");
}

#[test]
fn test_ignore_multiple_patterns() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create test directory with various files
    fixture.write_file("src/testdir/file.txt", "text");
    fixture.write_file("src/testdir/file.log", "log");
    fixture.write_file("src/testdir/file.tmp", "temp");
    fixture.write_file("src/testdir/file.md", "markdown");

    fixture.import("src/testdir/");

    // Add multiple ignore patterns
    let mut config = fixture.get_config();
    let package = config.packages.get_mut("d_testdir").unwrap();
    package.ignore.push("*.log".to_string());
    package.ignore.push("*.tmp".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["d_testdir".to_string()]));

    // Verify only .txt and .md are deployed
    fixture.assert_file_exists("src/testdir/file.txt", "file.txt should be deployed");
    fixture.assert_file_exists("src/testdir/file.md", "file.md should be deployed");
}

#[test]
fn test_ignore_directory_pattern() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create nested directories
    fixture.write_file("src/testdir/keep/file.txt", "keep");
    fixture.write_file("src/testdir/ignore_me/file.txt", "ignore");
    fixture.write_file("src/testdir/also_keep/file.txt", "keep");

    fixture.import("src/testdir/");

    // Add ignore pattern for specific directory
    let mut config = fixture.get_config();
    let package = config.packages.get_mut("d_testdir").unwrap();
    package.ignore.push("ignore_me/*".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["d_testdir".to_string()]));

    // Verify keep directories exist but ignore_me doesn't
    fixture.assert_file_exists("src/testdir/keep/file.txt", "keep dir should be deployed");
    fixture.assert_file_exists(
        "src/testdir/also_keep/file.txt",
        "also_keep dir should be deployed",
    );
}

#[test]
fn test_ignore_nested_file_pattern() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create nested structure
    fixture.write_file("src/testdir/subdir/file.txt", "text");
    fixture.write_file("src/testdir/subdir/file.cache", "cache");
    fixture.write_file("src/testdir/another/file.txt", "text2");
    fixture.write_file("src/testdir/another/file.cache", "cache2");

    fixture.import("src/testdir/");

    // Ignore all .cache files in any subdirectory
    let mut config = fixture.get_config();
    let package = config.packages.get_mut("d_testdir").unwrap();
    package.ignore.push("**/*.cache".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["d_testdir".to_string()]));

    // Verify .txt files are deployed but .cache files are ignored
    fixture.assert_file_exists(
        "src/testdir/subdir/file.txt",
        "nested txt should be deployed",
    );
    fixture.assert_file_exists(
        "src/testdir/another/file.txt",
        "nested txt should be deployed",
    );
}

#[test]
fn test_ignore_during_update() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create and import directory
    fixture.write_file("src/testdir/important.txt", "important");
    fixture.write_file("src/testdir/cache.tmp", "cache");
    fixture.import("src/testdir/");

    // Add ignore pattern
    let mut config = fixture.get_config();
    let package = config.packages.get_mut("d_testdir").unwrap();
    package.ignore.push("*.tmp".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy first
    fixture.deploy(Some(vec!["d_testdir".to_string()]));

    // Modify both files
    fixture.write_file("src/testdir/important.txt", "modified important");
    fixture.write_file("src/testdir/cache.tmp", "modified cache");

    // Update - should only update important.txt
    fixture.update(Some(vec!["d_testdir".to_string()]));

    // Verify update respects ignore patterns
    let important_content =
        fs::read_to_string(fixture.cwd.join("dotfiles/d_testdir/important.txt")).unwrap();
    assert_eq!(
        important_content, "modified important",
        "important.txt should be updated"
    );
}

#[test]
fn test_ignore_with_exact_filename() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create files
    fixture.write_file("src/testdir/README.md", "readme");
    fixture.write_file("src/testdir/.DS_Store", "macos");
    fixture.write_file("src/testdir/config.json", "config");

    fixture.import("src/testdir/");

    // Ignore specific file by name
    let mut config = fixture.get_config();
    let package = config.packages.get_mut("d_testdir").unwrap();
    package.ignore.push(".DS_Store".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["d_testdir".to_string()]));

    // Verify .DS_Store is ignored
    fixture.assert_file_exists("src/testdir/README.md", "README should be deployed");
    fixture.assert_file_exists("src/testdir/config.json", "config should be deployed");
}

#[test]
fn test_ignore_patterns_persist() {
    let fixture = TestFixture::new();
    fixture.init();

    fixture.write_file("src/testdir/file.txt", "content");
    fixture.import("src/testdir/");

    // Add ignore patterns
    let mut config = fixture.get_config();
    let package = config.packages.get_mut("d_testdir").unwrap();
    package.ignore.push("*.log".to_string());
    package.ignore.push("*.tmp".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Reload config and verify patterns persist
    let reloaded_config = fixture.get_config();
    let package = reloaded_config.packages.get("d_testdir").unwrap();
    assert_eq!(package.ignore.len(), 2, "Should have 2 ignore patterns");
    assert!(package.ignore.contains(&"*.log".to_string()));
    assert!(package.ignore.contains(&"*.tmp".to_string()));
}

#[test]
fn test_empty_ignore_patterns() {
    let fixture = TestFixture::new();
    fixture.init();

    fixture.write_file("src/testdir/file.txt", "content");
    fixture.import("src/testdir/");

    // Verify default empty ignore list
    let config = fixture.get_config();
    let package = config.packages.get("d_testdir").unwrap();
    assert_eq!(
        package.ignore.len(),
        0,
        "Default ignore list should be empty"
    );

    // Deploy should work normally
    fixture.deploy(Some(vec!["d_testdir".to_string()]));
    fixture.assert_file_exists("src/testdir/file.txt", "File should be deployed");
}

#[test]
fn test_ignore_node_modules_pattern() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a project structure with node_modules
    fixture.write_file("src/project/package.json", "{}");
    fixture.write_file("src/project/src/index.js", "code");
    fixture.write_file("src/project/node_modules/lib/file.js", "library");
    fixture.write_file("src/project/node_modules/another/file.js", "another");

    fixture.import("src/project/");

    // Ignore node_modules directory
    let mut config = fixture.get_config();
    let package = config.packages.get_mut("d_project").unwrap();
    package.ignore.push("node_modules/**".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["d_project".to_string()]));

    // Verify project files are deployed but node_modules is ignored
    fixture.assert_file_exists(
        "src/project/package.json",
        "package.json should be deployed",
    );
    fixture.assert_file_exists(
        "src/project/src/index.js",
        "source files should be deployed",
    );
}

#[test]
fn test_ignore_hidden_files() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create files including hidden files
    fixture.write_file("src/testdir/visible.txt", "visible");
    fixture.write_file("src/testdir/.hidden", "hidden");
    fixture.write_file("src/testdir/.another_hidden", "hidden2");

    fixture.import("src/testdir/");

    // Ignore all hidden files
    let mut config = fixture.get_config();
    let package = config.packages.get_mut("d_testdir").unwrap();
    package.ignore.push(".*".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["d_testdir".to_string()]));

    // Verify visible file is deployed
    fixture.assert_file_exists("src/testdir/visible.txt", "visible file should be deployed");
}

#[test]
fn test_ignore_specific_subdirectory() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create nested structure
    fixture.write_file("src/testdir/src/main.rs", "code");
    fixture.write_file("src/testdir/target/debug/app", "binary");
    fixture.write_file("src/testdir/target/release/app", "binary");
    fixture.write_file("src/testdir/Cargo.toml", "manifest");

    fixture.import("src/testdir/");

    // Ignore target directory (like Rust projects)
    let mut config = fixture.get_config();
    let package = config.packages.get_mut("d_testdir").unwrap();
    package.ignore.push("target/**".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["d_testdir".to_string()]));

    // Verify source files are deployed but target is ignored
    fixture.assert_file_exists("src/testdir/src/main.rs", "source should be deployed");
    fixture.assert_file_exists("src/testdir/Cargo.toml", "manifest should be deployed");
}

#[test]
fn test_ignore_with_complex_glob() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create various test files
    fixture.write_file("src/testdir/test_file.txt", "test");
    fixture.write_file("src/testdir/spec_file.txt", "spec");
    fixture.write_file("src/testdir/prod_file.txt", "prod");

    fixture.import("src/testdir/");

    // Ignore files starting with test_ or spec_
    let mut config = fixture.get_config();
    let package = config.packages.get_mut("d_testdir").unwrap();
    package.ignore.push("test_*".to_string());
    package.ignore.push("spec_*".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["d_testdir".to_string()]));

    // Verify only prod file is deployed
    fixture.assert_file_exists("src/testdir/prod_file.txt", "prod file should be deployed");
}

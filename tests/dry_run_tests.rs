use std::{fs, path::PathBuf};

use dotr_dear::cli::{Cli, Command, DeployArgs, ImportArgs, InitArgs, UpdateArgs, run_cli};

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

    fn get_cli(&self, command: Option<Command>) -> Cli {
        Cli {
            command,
            working_dir: Some(PLAYGROUND_DIR.to_string()),
        }
    }

    fn init(&self) {
        run_cli(self.get_cli(Some(Command::Init(InitArgs {})))).expect("Init failed");
    }

    fn import(&self, path: &str) {
        run_cli(self.get_cli(Some(Command::Import(ImportArgs {
            path: path.to_string(),
            ..Default::default()
        }))))
        .expect("Import failed");
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        common::teardown(&self.cwd);
    }
}

#[test]
fn test_dry_run_deploy_no_files_created() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create source file in src (to be imported)
    fs::create_dir_all(fixture.cwd.join("src")).expect("Failed to create src dir");
    fs::write(fixture.cwd.join("src/.test"), "Test content\n").expect("Failed to create file");

    // Import the file
    fixture.import("src/.test");

    // Remove the destination file
    fs::remove_file(fixture.cwd.join("src/.test")).expect("Failed to remove destination");

    // Deploy with dry run
    let args = DeployArgs {
        dry_run: true,
        ..Default::default()
    };
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(args))));
    assert!(result.is_ok(), "Dry run deploy should succeed");

    // Verify file was NOT created
    assert!(
        !fixture.cwd.join("src/.test").exists(),
        "File should not be created in dry run mode"
    );
}

#[test]
fn test_dry_run_deploy_directory() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create directory with files
    fs::create_dir_all(fixture.cwd.join("src/testdir")).expect("Failed to create test dir");
    fs::write(fixture.cwd.join("src/testdir/file1.txt"), "content1")
        .expect("Failed to create file1");
    fs::write(fixture.cwd.join("src/testdir/file2.txt"), "content2")
        .expect("Failed to create file2");

    // Import the directory
    fixture.import("src/testdir");

    // Remove the destination
    fs::remove_dir_all(fixture.cwd.join("src/testdir")).expect("Failed to remove directory");

    // Deploy with dry run
    let args = DeployArgs {
        dry_run: true,
        ..Default::default()
    };
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(args))));
    assert!(result.is_ok(), "Dry run deploy should succeed");

    // Verify directory and files were NOT created
    assert!(
        !fixture.cwd.join("src/testdir").exists(),
        "Directory should not be created in dry run mode"
    );
}

#[test]
fn test_dry_run_deploy_no_backup_created() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create source file and deploy it first
    fs::create_dir_all(fixture.cwd.join("src")).expect("Failed to create src dir");
    fs::write(fixture.cwd.join("src/.test"), "Old content\n").expect("Failed to create file");

    fixture.import("src/.test");

    // Verify it was deployed (in dotfiles)
    assert!(fixture.cwd.join("dotfiles/f_test").exists());

    // Modify the source file in dotfiles
    fs::write(fixture.cwd.join("dotfiles/f_test"), "New content\n")
        .expect("Failed to update source");

    // Deploy with dry run (should try to overwrite but not actually do it)
    let args = DeployArgs {
        dry_run: true,
        ..Default::default()
    };
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(args))));
    assert!(result.is_ok(), "Dry run deploy should succeed");

    // Verify backup was NOT created
    assert!(
        !fixture.cwd.join("src/.test.bak").exists(),
        "Backup should not be created in dry run mode"
    );

    // Verify original file is unchanged
    let content =
        fs::read_to_string(fixture.cwd.join("src/.test")).expect("Failed to read original file");
    assert_eq!(
        content, "Old content\n",
        "Original file should remain unchanged in dry run mode"
    );
}

#[test]
fn test_dry_run_deploy_clean_no_files_removed() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create directory with files
    fs::create_dir_all(fixture.cwd.join("src/testdir")).expect("Failed to create dir");
    fs::write(fixture.cwd.join("src/testdir/file1.txt"), "content1")
        .expect("Failed to create file1");

    // Import and deploy normally first
    fixture.import("src/testdir");
    run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))))
        .expect("Initial deploy failed");

    // Add an extra file to the destination
    fs::write(fixture.cwd.join("src/testdir/extra.txt"), "extra content")
        .expect("Failed to create extra file");

    // Deploy with dry run and clean
    let args = DeployArgs {
        dry_run: true,
        clean: Some(true),
        ..Default::default()
    };
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(args))));
    assert!(result.is_ok(), "Dry run deploy with clean should succeed");

    // Verify extra file was NOT removed
    assert!(
        fixture.cwd.join("src/testdir/extra.txt").exists(),
        "Extra file should not be removed in dry run mode"
    );
}

#[test]
fn test_dry_run_update_no_files_modified() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create and deploy a file first
    fs::create_dir_all(fixture.cwd.join("src")).expect("Failed to create src dir");
    fs::write(fixture.cwd.join("src/.test"), "Version 1\n").expect("Failed to create file");

    fixture.import("src/.test");

    // Deploy normally first
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))));
    assert!(result.is_ok(), "Initial deploy should succeed");

    // Verify file was deployed
    assert!(fixture.cwd.join("src/.test").exists());

    // Modify the destination file manually
    fs::write(fixture.cwd.join("src/.test"), "Version 2\n").expect("Failed to update file");

    // Update with dry run
    let args = UpdateArgs {
        dry_run: true,
        ..Default::default()
    };
    let result = run_cli(fixture.get_cli(Some(Command::Update(args))));
    assert!(result.is_ok(), "Dry run update should succeed");

    // Verify source file (in dotfiles) was NOT updated
    let content = fs::read_to_string(fixture.cwd.join("dotfiles/f_test"))
        .expect("Failed to read source file");
    assert_eq!(
        content, "Version 1\n",
        "Source file should not be updated in dry run mode"
    );
}

#[test]
fn test_dry_run_update_with_clean() {
    let fixture = TestFixture::new();
    fixture.init();

    // Setup
    fs::create_dir_all(fixture.cwd.join("src/testdir")).expect("Failed to create dir");
    fs::write(fixture.cwd.join("src/testdir/file1.txt"), "content").expect("Failed to create file");

    fixture.import("src/testdir");

    // Deploy normally first
    run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))))
        .expect("Initial deploy failed");

    // Add extra file to source (in dotfiles)
    fs::write(fixture.cwd.join("dotfiles/d_testdir/extra.txt"), "extra")
        .expect("Failed to create extra file in source");

    // Update with dry run and clean
    let args = UpdateArgs {
        dry_run: true,
        clean: Some(true),
        ..Default::default()
    };
    let result = run_cli(fixture.get_cli(Some(Command::Update(args))));
    assert!(result.is_ok(), "Dry run update with clean should succeed");

    // Verify extra file still only exists in source, not backed up
    assert!(
        !fixture.cwd.join("src/testdir/extra.txt").exists(),
        "Extra file from dotfiles should not be backed up in dry run mode"
    );
}

#[test]
fn test_normal_deploy_after_dry_run() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a file
    fs::create_dir_all(fixture.cwd.join("src")).expect("Failed to create src dir");
    fs::write(fixture.cwd.join("src/.test"), "Test content\n").expect("Failed to create file");

    fixture.import("src/.test");

    // Remove destination
    fs::remove_file(fixture.cwd.join("src/.test")).expect("Failed to remove file");

    // First do dry run
    let dry_args = DeployArgs {
        dry_run: true,
        ..Default::default()
    };
    run_cli(fixture.get_cli(Some(Command::Deploy(dry_args)))).expect("Dry run should succeed");

    assert!(
        !fixture.cwd.join("src/.test").exists(),
        "File should not exist after dry run"
    );

    // Now do actual deploy
    run_cli(fixture.get_cli(Some(Command::Deploy(DeployArgs::default()))))
        .expect("Normal deploy should succeed");

    // Verify file was created
    assert!(
        fixture.cwd.join("src/.test").exists(),
        "File should exist after normal deploy"
    );
    let content =
        fs::read_to_string(fixture.cwd.join("src/.test")).expect("Failed to read deployed file");
    assert_eq!(content, "Test content\n", "Content should match");
}

#[test]
fn test_dry_run_shows_what_would_happen() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a file
    fs::create_dir_all(fixture.cwd.join("src")).expect("Failed to create src dir");
    fs::write(fixture.cwd.join("src/.test"), "Test content\n").expect("Failed to create file");

    fixture.import("src/.test");

    // Remove destination
    fs::remove_file(fixture.cwd.join("src/.test")).expect("Failed to remove file");

    // Deploy with dry run (this should show messages about what would happen)
    let args = DeployArgs {
        dry_run: true,
        ..Default::default()
    };
    let result = run_cli(fixture.get_cli(Some(Command::Deploy(args))));

    // Should succeed even though nothing was actually deployed
    assert!(result.is_ok(), "Dry run should succeed");

    // File should NOT exist
    assert!(
        !fixture.cwd.join("src/.test").exists(),
        "File should not be created in dry run"
    );
}

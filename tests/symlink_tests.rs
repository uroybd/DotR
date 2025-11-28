use std::{fs, path::PathBuf};

use dotr_dear::{
    cli::{DeployArgs, ImportArgs, InitArgs, UpdateArgs, run_cli},
    config::Config,
    utils::SYMLINK_FOLDER,
};

mod common;

const PLAYGROUND_DIR: &str = "tests/playground";
const BASHRC_PATH: &str = "src/.bashrc";
const NVIM_PATH: &str = "src/nvim";
const ZSHRC_PATH: &str = "src/.zshrc";

struct TestFixture {
    cwd: PathBuf,
}

impl TestFixture {
    fn new() -> Self {
        let cwd = PathBuf::from(PLAYGROUND_DIR);
        common::setup(&cwd);
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

    fn import(&self, path: &str, symlink: bool) {
        run_cli(
            self.get_cli(Some(dotr_dear::cli::Command::Import(ImportArgs {
                path: path.to_string(),
                symlink,
                ..Default::default()
            }))),
        )
        .expect("Import failed");
    }

    fn deploy(&self, packages: Option<Vec<String>>) {
        run_cli(
            self.get_cli(Some(dotr_dear::cli::Command::Deploy(DeployArgs {
                packages,
                profile: None,
                ignore_errors: false,
                clean: false,
                dry_run: false,
            }))),
        )
        .expect("Deploy failed");
    }

    fn update(&self, packages: Option<Vec<String>>) {
        run_cli(
            self.get_cli(Some(dotr_dear::cli::Command::Update(UpdateArgs {
                packages,
                profile: None,
                ignore_errors: false,
                clean: false,
                dry_run: false,
            }))),
        )
        .expect("Update failed");
    }

    fn get_config(&self) -> Config {
        Config::from_path(&self.cwd).expect("Failed to load config")
    }

    fn assert_file_exists(&self, path: &str, message: &str) {
        assert!(self.cwd.join(path).exists(), "{}", message);
    }

    fn assert_file_not_exists(&self, path: &str, message: &str) {
        assert!(!self.cwd.join(path).exists(), "{}", message);
    }

    fn assert_is_symlink(&self, path: &str, message: &str) {
        let full_path = self.cwd.join(path);
        assert!(
            full_path.exists(),
            "Path {} should exist for {}",
            path,
            message
        );
        assert!(full_path.is_symlink(), "{}", message);
    }

    fn assert_not_symlink(&self, path: &str, message: &str) {
        let full_path = self.cwd.join(path);
        if full_path.exists() {
            assert!(!full_path.is_symlink(), "{}", message);
        }
    }

    fn assert_symlink_target(&self, symlink_path: &str, expected_target: &str, message: &str) {
        let full_path = self.cwd.join(symlink_path);
        assert!(
            full_path.is_symlink(),
            "Path {} should be a symlink",
            symlink_path
        );

        let target = fs::read_link(&full_path)
            .unwrap_or_else(|_| panic!("Failed to read symlink: {}", symlink_path));

        // Convert expected_target to absolute path for comparison
        let expected_relative = self.cwd.join(expected_target);
        let expected_absolute = std::path::absolute(&expected_relative)
            .unwrap_or_else(|_| panic!("Failed to get absolute path for: {}", expected_target));
        assert_eq!(target, expected_absolute, "{}", message);
    }

    fn write_file(&self, path: &str, content: &str) {
        let file_path = self.cwd.join(path);
        if let Some(parent) = file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(file_path, content).unwrap_or_else(|_| panic!("Failed to write file: {}", path));
    }

    fn read_file(&self, path: &str) -> String {
        let file_path = self.cwd.join(path);
        fs::read_to_string(&file_path).unwrap_or_else(|_| panic!("Failed to read file: {}", path))
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        // Clean up the deployed directory before teardown (unless NO_CLEANUP is set)
        if std::env::var("NO_CLEANUP").is_err() {
            let deployed_dir = self.cwd.join(SYMLINK_FOLDER);
            if deployed_dir.exists() {
                let _ = fs::remove_dir_all(&deployed_dir);
            }
        }
        common::teardown(&self.cwd);
    }
}

#[test]
fn test_import_with_symlink_flag() {
    let fixture = TestFixture::new();
    fixture.init();
    fixture.import(BASHRC_PATH, true);

    let config = fixture.get_config();
    let package = config
        .packages
        .values()
        .next()
        .expect("Should have a package");

    assert!(package.symlink, "Package should have symlink flag enabled");
}

#[test]
fn test_import_without_symlink_flag() {
    let fixture = TestFixture::new();
    fixture.init();
    fixture.import(BASHRC_PATH, false);

    let config = fixture.get_config();
    let package = config
        .packages
        .values()
        .next()
        .expect("Should have a package");

    assert!(
        !package.symlink,
        "Package should not have symlink flag enabled"
    );
}

#[test]
fn test_deploy_with_symlink_creates_deployed_folder() {
    let fixture = TestFixture::new();
    fixture.init();
    fixture.import(BASHRC_PATH, true);

    // Import with symlink flag automatically deploys
    fixture.assert_file_exists(
        SYMLINK_FOLDER,
        "deployed folder should be created after import with symlink",
    );
}

#[test]
fn test_deploy_with_symlink_creates_symlink_at_dest() {
    let fixture = TestFixture::new();
    fixture.init();
    fixture.import(BASHRC_PATH, true);

    // Import with symlink flag automatically deploys and creates symlink
    fixture.assert_is_symlink(
        BASHRC_PATH,
        "Destination should be a symlink after import with symlink",
    );
}

#[test]
fn test_deploy_with_symlink_points_to_deployed_folder() {
    let fixture = TestFixture::new();
    fixture.init();
    fixture.import(BASHRC_PATH, true);

    let config = fixture.get_config();
    let package_name = config
        .packages
        .keys()
        .next()
        .expect("Should have a package");

    // Import with symlink flag automatically deploys and creates symlink
    let expected_target = format!("{}/{}", SYMLINK_FOLDER, package_name);
    fixture.assert_symlink_target(
        BASHRC_PATH,
        &expected_target,
        "Symlink should point to deployed folder after import with symlink",
    );
}

#[test]
fn test_deploy_with_symlink_deploys_to_deployed_folder() {
    let fixture = TestFixture::new();
    fixture.init();
    fixture.import(BASHRC_PATH, true);

    let config = fixture.get_config();
    let package_name = config
        .packages
        .keys()
        .next()
        .expect("Should have a package");

    // Import with symlink flag automatically deploys, so deployed folder should already exist
    let deployed_file = format!("{}/{}", SYMLINK_FOLDER, package_name);
    fixture.assert_file_exists(
        &deployed_file,
        "File should be deployed to deployed folder after import with symlink",
    );
}

#[test]
fn test_deploy_with_symlink_directory() {
    let fixture = TestFixture::new();
    fixture.init();
    fixture.import(NVIM_PATH, true);

    let config = fixture.get_config();
    let package_name = config
        .packages
        .keys()
        .next()
        .expect("Should have a package");

    // Import with symlink flag automatically deploys
    // Check that the deployed directory contains the files
    let deployed_init = format!("{}/{}/init.lua", SYMLINK_FOLDER, package_name);
    fixture.assert_file_exists(
        &deployed_init,
        "init.lua should be in deployed folder after import with symlink",
    );

    // Check that destination is a symlink
    fixture.assert_is_symlink(
        NVIM_PATH,
        "nvim directory should be a symlink after import with symlink",
    );
}

#[test]
fn test_deploy_without_symlink_does_not_create_deployed_folder() {
    let fixture = TestFixture::new();
    fixture.init();
    fixture.import(BASHRC_PATH, false);

    fixture.deploy(None);

    fixture.assert_file_not_exists(
        SYMLINK_FOLDER,
        "deployed folder should not be created for non-symlink packages",
    );
}

#[test]
fn test_deploy_without_symlink_creates_regular_file() {
    let fixture = TestFixture::new();
    fixture.init();
    fixture.import(BASHRC_PATH, false);

    fixture.deploy(None);

    fixture.assert_not_symlink(
        BASHRC_PATH,
        "Destination should be a regular file, not a symlink",
    );
}

#[test]
fn test_symlink_replaces_existing_file() {
    let fixture = TestFixture::new();
    fixture.init();

    // File already exists from setup in src/.bashrc
    // Import with symlink should replace it with a symlink
    fixture.import(BASHRC_PATH, true);

    fixture.assert_is_symlink(
        BASHRC_PATH,
        "Existing file should be replaced with symlink after import with symlink",
    );
}

#[test]
fn test_symlink_replaces_existing_directory() {
    let fixture = TestFixture::new();
    fixture.init();

    // Directory already exists from setup in src/nvim/
    // Import with symlink should replace it with a symlink
    fixture.import(NVIM_PATH, true);

    fixture.assert_is_symlink(
        NVIM_PATH,
        "Existing directory should be replaced with symlink after import with symlink",
    );
}

#[test]
fn test_symlink_replaces_existing_symlink() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create an existing symlink at the destination
    let bashrc = fixture.cwd.join(BASHRC_PATH);
    let temp_target = fixture.cwd.join("src/.bashrc.old");

    // Create target file for the old symlink
    fs::write(&temp_target, "# Old target\n").expect("Failed to write temp file");

    // Remove existing file first
    if bashrc.exists() {
        fs::remove_file(&bashrc).expect("Failed to remove existing file");
    }

    // Create old symlink with relative path
    std::os::unix::fs::symlink(".bashrc.old", &bashrc).expect("Failed to create symlink");

    // Import with symlink should replace the old symlink with a new one
    fixture.import(BASHRC_PATH, true);

    // Should still be a symlink but pointing to new location
    fixture.assert_is_symlink(
        BASHRC_PATH,
        "Should remain a symlink after import with symlink",
    );

    let config = fixture.get_config();
    let package_name = config
        .packages
        .keys()
        .next()
        .expect("Should have a package");
    let expected_target = format!("{}/{}", SYMLINK_FOLDER, package_name);

    fixture.assert_symlink_target(
        BASHRC_PATH,
        &expected_target,
        "Symlink should point to new deployed folder after import with symlink",
    );
}

#[test]
fn test_multiple_symlink_packages() {
    let fixture = TestFixture::new();
    fixture.init();

    fixture.import(BASHRC_PATH, true);
    fixture.import(ZSHRC_PATH, true);
    fixture.import(NVIM_PATH, true);

    // All should be symlinked after import with symlink flag
    fixture.assert_is_symlink(BASHRC_PATH, "bashrc should be symlinked");
    fixture.assert_is_symlink(ZSHRC_PATH, "zshrc should be symlinked");
    fixture.assert_is_symlink(NVIM_PATH, "nvim should be symlinked");

    // Verify deployed folder has all packages
    let config = fixture.get_config();
    for package_name in config.packages.keys() {
        let deployed_path = fixture.cwd.join(SYMLINK_FOLDER).join(package_name);
        assert!(
            deployed_path.exists(),
            "Package {} should be in deployed folder",
            package_name
        );
    }
}

#[test]
fn test_mixed_symlink_and_regular_packages() {
    let fixture = TestFixture::new();
    fixture.init();

    fixture.import(BASHRC_PATH, true); // symlink
    fixture.import(ZSHRC_PATH, false); // regular

    // Deploy regular package manually
    fixture.deploy(Some(vec!["f_zshrc".to_string()]));

    fixture.assert_is_symlink(BASHRC_PATH, "bashrc should be symlinked");
    fixture.assert_not_symlink(ZSHRC_PATH, "zshrc should be a regular file");
}

#[test]
fn test_update_symlink_package() {
    let fixture = TestFixture::new();
    fixture.init();
    fixture.import(BASHRC_PATH, true);

    // Modify the file through the symlink (which modifies deployed/f_bashrc)
    fixture.write_file(BASHRC_PATH, "# Modified through symlink\n");

    // Update the package (backs up from dest to dotfiles)
    fixture.update(None);

    // Verify the dotfiles source was updated
    let config = fixture.get_config();
    let package = config
        .packages
        .values()
        .next()
        .expect("Should have a package");
    let dotfiles_content = fixture.read_file(&package.src);
    assert!(
        dotfiles_content.contains("Modified through symlink"),
        "Dotfiles source should be updated"
    );

    // Verify symlink still exists and points correctly
    fixture.assert_is_symlink(BASHRC_PATH, "bashrc should still be a symlink after update");
}

#[test]
fn test_symlink_config_serialization() {
    let fixture = TestFixture::new();
    fixture.init();
    fixture.import(BASHRC_PATH, true);

    // Read the config file
    let config_content = fixture.read_file("config.toml");

    // Verify symlink field is present
    assert!(
        config_content.contains("symlink = true"),
        "Config should contain symlink = true"
    );
}

#[test]
fn test_symlink_not_backed_up() {
    let fixture = TestFixture::new();
    fixture.init();

    // Import with symlink - existing file is backed up to dotfiles during import
    fixture.import(BASHRC_PATH, true);

    // Verify no .dotrbak backup was created at destination (since we create symlink)
    fixture.assert_file_not_exists(
        "src/.bashrc.dotrbak",
        "Symlink deployment should not create backups at destination",
    );
}

#[test]
fn test_symlink_with_nested_directory() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create nested directory structure (already exists from common setup)
    let nested_path = "src/config/alacritty";
    fixture.import(nested_path, true);

    // Import with symlink automatically creates symlink
    fixture.assert_is_symlink(
        nested_path,
        "Nested directory should be symlinked after import with symlink",
    );

    // Verify parent directories are created
    let parent_dir = fixture.cwd.join("src/config");
    assert!(parent_dir.exists(), "Parent directory should exist");
}

#[test]
fn test_gitignore_includes_deployed_folder() {
    let fixture = TestFixture::new();
    fixture.init();

    let gitignore_content = fixture.read_file(".gitignore");
    assert!(
        gitignore_content.contains("deployed"),
        "gitignore should contain 'deployed' folder"
    );
}

#[test]
fn test_symlink_dry_run() {
    let fixture = TestFixture::new();
    fixture.init();

    // Import without symlink first
    fixture.import(BASHRC_PATH, false);

    // Manually update config to add symlink flag
    let mut config = fixture.get_config();
    let pkg_name = config
        .packages
        .keys()
        .next()
        .expect("Should have package")
        .clone();
    config.packages.get_mut(&pkg_name).unwrap().symlink = true;
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy with dry run
    run_cli(
        fixture.get_cli(Some(dotr_dear::cli::Command::Deploy(DeployArgs {
            packages: None,
            profile: None,
            ignore_errors: false,
            clean: false,
            dry_run: true,
        }))),
    )
    .expect("Dry run deploy failed");

    // Verify nothing was actually created
    fixture.assert_file_not_exists(
        SYMLINK_FOLDER,
        "deployed folder should not be created in dry run",
    );

    fixture.assert_not_symlink(BASHRC_PATH, "Symlink should not be created in dry run");
}

#[test]
fn test_symlink_preserves_file_through_link() {
    let fixture = TestFixture::new();
    fixture.init();
    fixture.import(BASHRC_PATH, true);

    // Read original content through symlink
    let original_content = fixture.read_file(BASHRC_PATH);

    // Modify through symlink
    fixture.write_file(BASHRC_PATH, "# Modified through symlink\n");

    // Verify modification persists
    let modified_content = fixture.read_file(BASHRC_PATH);
    assert_eq!(modified_content, "# Modified through symlink\n");
    assert_ne!(original_content, modified_content);
}

#[test]
fn test_symlink_directory_preserves_structure() {
    let fixture = TestFixture::new();
    fixture.init();
    fixture.import(NVIM_PATH, true);

    // Verify symlinked directory has correct structure
    let init_lua = fixture.cwd.join(NVIM_PATH).join("init.lua");
    assert!(
        init_lua.exists(),
        "init.lua should be accessible through symlink"
    );

    let content = fs::read_to_string(&init_lua).expect("Failed to read init.lua");
    assert!(!content.is_empty(), "File should have content");
}

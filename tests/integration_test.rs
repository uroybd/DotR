use std::{fs, path::PathBuf};

use dotr::{
    cli::{copydots::copy_dir_all, run_cli},
    utils,
};

mod common;

fn get_default_cli() -> dotr::cli::Cli {
    dotr::cli::Cli {
        command: None,
        working_dir: "tests/playground".to_string(),
    }
}

fn get_init_cli() -> dotr::cli::Cli {
    dotr::cli::Cli {
        command: Some(dotr::cli::Command::Init {}),
        working_dir: "tests/playground".to_string(),
    }
}

fn get_pathbuf() -> PathBuf {
    PathBuf::from("tests/playground")
}

#[test]
fn test_no_command() {
    // Simulate no command line arguments
    let cwd = get_pathbuf();
    let args = get_default_cli();
    run_cli(args);
    // Since no command is provided, we expect no config file or dotfiles directory to be created
    let config_path = cwd.join("config.toml");
    assert!(!config_path.exists(), "config.toml should not be created");
    let dotfiles_dir = cwd.join("dotfiles");
    assert!(
        !dotfiles_dir.exists(),
        "dotfiles directory should not be created"
    );
    common::teardown(&cwd);
}

#[test]
fn test_init_config() {
    let cwd = get_pathbuf();
    // Simulate command line arguments for "init"
    let args = get_init_cli();
    run_cli(args);
    // Check if config file is created
    let config_path = cwd.join("config.toml");
    assert!(config_path.exists(), "config.toml should be created");
    // Check if dotfiles directory is created
    let dotfiles_dir = cwd.join("dotfiles");
    assert!(
        dotfiles_dir.exists(),
        "dotfiles directory should be created"
    );
    common::teardown(&cwd);
}

#[test]
fn test_import_dots() {
    // First, initialize the config
    let cwd = get_pathbuf();
    let init_args = get_init_cli();
    run_cli(init_args);
    // Now, simulate command line arguments for "import"
    let import_path = "src/nvim/";
    let mut import_cli = get_default_cli();
    import_cli.command = Some(dotr::cli::Command::Import {
        path: import_path.to_string(),
    });
    run_cli(import_cli);
    // Load the config and verify the imported package
    let conf = dotr::config::load_config(&cwd.clone());
    // Print verbose information for debugging
    println!("Loaded config: {:?}", conf);
    let package_name = dotr::cli::importdots::get_package_name(import_path, &cwd);
    assert!(
        conf.packages.contains_key(&package_name),
        "Config should contain the imported package"
    );
    let package = conf.packages.get(&package_name).unwrap();
    assert_eq!(
        package.src, import_path,
        "Package src should match the imported path"
    );
    assert_eq!(
        package.dest,
        format!("dotfiles/{}", package_name),
        "Package dest should be correctly set"
    );
    // Verify that files are copied to the dotfiles directory
    let dest_path_str = format!("dotfiles/{}", package_name);
    let dest_path = cwd.join(dest_path_str);
    assert!(
        dest_path.exists(),
        "Destination path for imported package should exist"
    );
    let expected_file = dest_path.join("init.lua");
    assert!(
        expected_file.exists(),
        "Expected file init.vim should be copied to the destination"
    );
    common::teardown(&cwd);
}

#[test]
fn test_canonical_linking() {
    let cwd = get_pathbuf();
    let path_from_home = utils::resolve_path("~/.config", &cwd);
    let path_from_root = utils::resolve_path("/Volumes/Repos/", &cwd);
    let path_from_cwd = utils::resolve_path("src/nvim", &cwd);
    println!("From Home {}", path_from_home.display());
    println!("From Root {}", path_from_root.display());
    println!("From CWD {}", path_from_cwd.display());
    assert_eq!(1, 1);
}

#[test]
fn test_copy_dots() {
    // First, initialize the config
    let cwd = get_pathbuf();
    let init_args = get_init_cli();
    run_cli(init_args);
    // Now, simulate command line arguments for "import"
    let import_path = "src/nvim/";
    let mut import_cli = get_default_cli();
    import_cli.command = Some(dotr::cli::Command::Import {
        path: import_path.to_string(),
    });
    run_cli(import_cli);
    // Backup "src/nvim/"
    let abs_import_path = cwd.join(import_path);
    let backup_path = cwd.join("src/nvim.bak/");
    copy_dir_all(abs_import_path.clone(), backup_path.clone())
        .expect("Failed to backup original directory");
    let mut copy_cli = get_default_cli();
    copy_cli.command = Some(dotr::cli::Command::Copy {});
    run_cli(copy_cli);
    // src/nvim/init.lua.dotrbak should exist
    assert!(
        cwd.join("src/nvim.dotrbak/").exists(),
        "Backup file should exist"
    );
    // remove src/nvim and restore from backup
    fs::remove_dir_all(abs_import_path.clone()).expect("Failed to remove original directory");
    fs::rename(backup_path, abs_import_path).expect("Failed to restore backup.");
    common::teardown(&cwd);
}

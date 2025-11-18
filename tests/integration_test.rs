use std::path::PathBuf;

use dotr::cli::run_cli;

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

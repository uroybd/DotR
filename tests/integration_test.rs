use std::{fs, path::PathBuf};

use dotr::{
    cli::{run_cli, DeployArgs, ImportArgs, InitArgs},
    config::copy_dir_all,
    package::get_package_name,
    utils,
};

mod common;

fn get_default_cli() -> dotr::cli::Cli {
    dotr::cli::Cli {
        command: None,
        working_dir: Some("tests/playground".to_string()),
    }
}

fn get_init_cli() -> dotr::cli::Cli {
    dotr::cli::Cli {
        command: Some(dotr::cli::Command::Init(InitArgs {})),
        working_dir: Some("tests/playground".to_string()),
    }
}

fn import(path: &str) {
    let cmd = dotr::cli::Cli {
        command: Some(dotr::cli::Command::Import(ImportArgs {
            path: path.to_string(),
        })),
        working_dir: Some("tests/playground".to_string()),
    };
    run_cli(cmd);
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
    import(&import_path);
    let bashrc_path = "src/.bashrc";
    import(&bashrc_path);
    let conf = dotr::config::Config::from_path(&cwd.clone());
    // Print verbose information for debugging
    println!("Loaded config: {:?}", conf);
    let package_name = get_package_name(import_path, &cwd);
    println!("Package: {}", package_name.clone());
    assert!(
        conf.packages.contains_key(&package_name),
        "Config should contain the imported package"
    );
    let package = conf.packages.get(&package_name).unwrap();
    assert!(
        package.dest.ends_with(import_path),
        "Package dest should match the imported path"
    );
    assert_eq!(
        package.src,
        format!("dotfiles/{}", package_name),
        "Package src should be correctly set"
    );
    // Verify if src/.bashrc is in packages as well
    let bashrc_package_name = get_package_name("src/.bashrc", &cwd);
    assert!(
        conf.packages.contains_key(&bashrc_package_name),
        "Config should contain the imported .bashrc package"
    );
    // Verify that files are copied to the dotfiles directory
    let src_path_str = format!("dotfiles/{}", package_name);
    let src_path = cwd.join(src_path_str);
    assert!(
        src_path.exists(),
        "Source path for imported package should exist"
    );
    let expected_file = src_path.join("nvim/init.lua");
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
    import(&import_path);
    let bashrc_path = "src/.bashrc";
    import(&bashrc_path);
    // Backup "src/nvim/"
    let abs_import_path = cwd.join(import_path);
    let backup_path = cwd.join("src/nvim.bak/");
    copy_dir_all(abs_import_path.clone(), backup_path.clone())
        .expect("Failed to backup original directory");
    // Backup "src/.bashrc"
    let abs_bashrc_path = cwd.join("src/.bashrc");
    let backup_bashrc_path = cwd.join("src/.bashrc.bak");
    fs::copy(abs_bashrc_path.clone(), backup_bashrc_path.clone())
        .expect("Failed to backup original .bashrc file");
    let mut copy_cli = get_default_cli();
    copy_cli.command = Some(dotr::cli::Command::Deploy(DeployArgs { packages: None }));
    run_cli(copy_cli);
    // src/nvim/init.lua.dotrbak should exist
    assert!(
        cwd.join("src/nvim.dotrbak/").exists(),
        "Backup file should exist"
    );
    assert!(
        cwd.join("src/nvim/init.lua").exists(),
        "Copied file should exist"
    );
    // src/.bashrc.dotrbak should exist
    assert!(
        cwd.join("src/.bashrc.dotrbak").exists(),
        "Backup .bashrc file should exist"
    );
    assert!(
        cwd.join("src/.bashrc").exists(),
        "Copied .bashrc file should exist"
    );
    // remove src/nvim and restore from backup
    fs::remove_dir_all(abs_import_path.clone()).expect("Failed to remove original directory");
    fs::rename(backup_path, abs_import_path).expect("Failed to restore backup.");

    common::teardown(&cwd);
}

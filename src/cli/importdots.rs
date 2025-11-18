use std::path::PathBuf;

use crate::config::{Config, Package};

pub fn import_dots(path: &str, conf: &mut Config, cwd: &PathBuf) {
    println!("Importing dotfiles from path: {}", path);
    let package_name = get_package_name(path, cwd); // Implementation for importing dotfiles goes here
                                                    // In toml, packages will be represented as an array of tables
                                                    // [[packages.nvim]]
                                                    // name = "nvim"
                                                    // src = "/home/user/.config/nvim"
                                                    // dest = "dotfiles/nvim"
                                                    // dependencies = ["git", "curl"]
                                                    // Check if the package already exists in the config (in the above case d_nvim)

    let dest_path_str = format!("dotfiles/{}", package_name);

    let package = Package {
        src: path.to_string(),
        dest: dest_path_str.clone(),
        dependencies: vec![],
    };
    // Add the new package to the config
    conf.packages.insert(package_name.clone(), package);
    // Save the updated config
    conf.save(cwd);
    // Recursively copy the files from src to dest
    let dest_path = cwd.join(dest_path_str);
    let src_path = cwd.join(path);
    if !dest_path.exists() {
        std::fs::create_dir_all(dest_path.clone()).expect("Failed to create destination directory");
    }
    for entry in walkdir::WalkDir::new(src_path.clone()) {
        let entry = entry.expect("Failed to read directory entry");
        let relative_path = entry
            .path()
            .strip_prefix(src_path.clone())
            .expect("Failed to get relative path");
        let dest_file_path = dest_path.join(relative_path);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dest_file_path).expect("Failed to create directory");
        } else {
            std::fs::copy(entry.path(), &dest_file_path).expect("Failed to copy file");
        }
    }
    println!("Package '{}' imported successfully.", package_name);
}

pub fn get_package_name(pathstr: &str, cwd: &PathBuf) -> String {
    let mut path = cwd.join(pathstr);
    // Print current dir
    println!("Getting package name from path: {}", path.display());
    // get absolute path
    path = std::fs::canonicalize(path).expect("Failed to canonicalize path");
    // Throw error if path does not exist
    if !path.exists() {
        panic!("Path does not exist: {}", pathstr);
    }
    let last_component = path
        .components()
        .last()
        .expect("Path has no components")
        .as_os_str()
        .to_str()
        .expect("Failed to convert OsStr to str");
    let mut package_name = last_component.trim_start_matches('.').to_string();

    // Remove any trailing version numbers
    if let Some(pos) = package_name.rfind('-') {
        package_name.truncate(pos);
    }
    return package_name;
}

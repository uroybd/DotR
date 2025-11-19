use std::{
    collections::HashMap,
    path::Path,
};

use crate::config;

pub fn init_config(cwd: &Path) {
    // If config.toml already exists, do nothing
    let config_path = cwd.join("config.toml");
    if config_path.exists() {
        println!("config.toml already exists. Initialization skipped.");
        return;
    }
    // Here you would add the logic to create a default config file
    let default_config = config::Config {
        banner: true,
        packages: HashMap::new(),
    };
    let toml_string = toml::to_string(&default_config).expect("Failed to serialize default config");
    std::fs::write(config_path, toml_string).expect("Failed to write default config.toml");
    std::fs::create_dir_all(cwd.join("dotfiles")).expect("Failed to create dotfiles directory");
    println!("Default config.toml created.");
}

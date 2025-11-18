use std::{
    fmt::format,
    fs, io,
    path::{Path, PathBuf},
};

use crate::{config::Config, utils::resolve_path};

pub fn copy_dots(conf: &Config, cwd: &PathBuf) {
    println!("Copying dotfiles...");
    for (_, pkg) in conf.packages.iter() {
        let src_path = resolve_path(&pkg.src, cwd);
        let dest_path = cwd.join(&pkg.dest);
        if dest_path.exists() {
            if dest_path.is_dir() {
                let backup_path = src_path.with_extension("dotrbak");
                // Delete previous backup
                if backup_path.exists() {
                    std::fs::remove_dir_all(backup_path.clone())
                        .expect("Error removing previous backup");
                }
                println!(
                    "Src {}, backup {}",
                    src_path.clone().display(),
                    backup_path.clone().display()
                );
                std::fs::rename(src_path.clone(), backup_path.clone()).expect("Failed to backup");
                // Copy from dest_path to src_path
                copy_dir_all(dest_path, src_path.clone()).expect("Error copying config");
            } else {
                // create backup extension. e.g. init.lua -> init.lua.dotrbak
                let prev_extension = src_path.extension().unwrap().to_str().unwrap();
                let ext = format!("{}.dotrbak", prev_extension);
                let backup_path = src_path.with_extension(ext);
                std::fs::rename(&src_path, &backup_path).expect("Failed to backup existing file");
                std::fs::copy(dest_path, src_path).expect("Error copying dotfiles");
            }
        }
    }
}

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

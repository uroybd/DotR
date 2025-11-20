use std::path::{Path, PathBuf};

pub const BACKUP_EXT: &str = "dotrbak";

/// Resolve a path string to an absolute PathBuf
/// - If the path starts with '/', it's treated as an absolute path
/// - If the path starts with '~', it's treated as relative to the home directory
/// - Otherwise, it's treated as relative to the current working directory (cwd)
pub fn resolve_path(path: &str, cwd: &Path) -> PathBuf {
    if path.starts_with('/') {
        PathBuf::from(path)
    } else if path.starts_with("~") {
        let home_dir = std::env::home_dir().expect("Failed to get home directory");
        // remove first segment of the path
        let p = path.splitn(2, '/').collect::<Vec<&str>>();
        // print for debug
        home_dir.join(p[1..].join("/"))
    } else {
        let p = cwd.join(path);
        std::path::absolute(&p).expect("Failed to get absolute path")
    }
}

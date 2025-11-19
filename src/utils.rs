use std::path::PathBuf;

pub fn resolve_path(path: &str, cwd: &PathBuf) -> PathBuf {
    // Absolute:
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

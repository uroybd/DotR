use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub const BACKUP_EXT: &str = "dotrbak";
pub const SYMLINK_FOLDER: &str = "deployed";

/// Resolve a path string to an absolute PathBuf
/// - If the path starts with '/', it's treated as an absolute path
/// - If the path starts with '~', it's treated as relative to the home directory
/// - Otherwise, it's treated as relative to the current working directory (cwd)
pub fn resolve_path(path: &str, cwd: &Path) -> anyhow::Result<PathBuf> {
    if path.starts_with('/') {
        Ok(PathBuf::from(path))
    } else if path.starts_with("~") {
        let home_dir = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
        let p = path.splitn(2, '/').collect::<Vec<&str>>();
        Ok(home_dir.join(p[1..].join("/")))
    } else {
        let p = cwd.join(path);
        Ok(std::path::absolute(&p)?)
    }
}

/// Convert an absolute path to use ~ notation if it's in the home directory
/// - If the path is within the home directory, converts it to ~/...
/// - Otherwise, returns the original path as a string
pub fn normalize_home_path(path: &str) -> String {
    if path.starts_with('~') {
        return path.to_string();
    }

    if let Ok(home_dir) = std::env::var("HOME").map(PathBuf::from) {
        let home_str = home_dir.to_string_lossy();

        if path == home_str.as_ref() {
            return "~".to_string();
        }

        let home_with_slash = format!("{}/", home_str);
        if path.starts_with(&home_with_slash) {
            let relative = &path[home_str.len()..];
            return format!("~{}", relative);
        }
    }

    path.to_string()
}

pub const COLOR_WARNING: &str = "\x1b[33m"; // Yellow
pub const COLOR_ERROR: &str = "\x1b[31m"; // Red
pub const COLOR_INFO: &str = "\x1b[34m"; // Blue
pub const COLOR_FATAL: &str = "\x1b[35m"; // Magenta
pub const RESET_COLOR: &str = "\x1b[0m"; // Reset

pub enum LogLevel {
    Warning,
    Error,
    Info,
    Fatal,
}

impl LogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
            LogLevel::Info => "INFO",
            LogLevel::Fatal => "FATAL",
        }
    }

    pub fn to_colorful_str(&self) -> String {
        match self {
            LogLevel::Warning => format!("{}[{}]{}", COLOR_WARNING, self.as_str(), RESET_COLOR),
            LogLevel::Error => format!("{}[{}]{}", COLOR_ERROR, self.as_str(), RESET_COLOR),
            LogLevel::Info => format!("{}[{}]{}", COLOR_INFO, self.as_str(), RESET_COLOR),
            LogLevel::Fatal => format!("{}[{}]{}", COLOR_FATAL, self.as_str(), RESET_COLOR),
        }
    }
}

pub fn cprintln(message: &str, level: &LogLevel) {
    match level {
        LogLevel::Error | LogLevel::Fatal => {
            eprintln!("{} {}", level.to_colorful_str(), message);
        }
        LogLevel::Warning | LogLevel::Info => {
            println!("{} {}", level.to_colorful_str(), message);
        }
    }
}

pub fn get_string_from_value(v: Option<&toml::Value>, field_name: &str) -> anyhow::Result<String> {
    Ok(
        v.ok_or_else(|| anyhow::anyhow!("'{}' is required", field_name))?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'{}' must be a string", field_name))?
            .to_string(),
    )
}

pub fn get_string_hashmap_from_value(
    v: Option<&toml::Value>,
) -> anyhow::Result<HashMap<String, String>> {
    match v {
        Some(value) => value
            .as_table()
            .ok_or_else(|| anyhow::anyhow!("field must be a table"))?
            .iter()
            .map(|(key, value)| {
                let s = value
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("table values must be strings"))?;
                Ok((key.clone(), s.to_string()))
            })
            .collect::<Result<HashMap<_, _>, _>>(),
        None => Ok(HashMap::new()),
    }
}

pub fn get_vec_string_from_value(v: Option<&toml::Value>) -> anyhow::Result<Vec<String>> {
    match v {
        Some(block) => block
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("field must be an array"))?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| anyhow::anyhow!("array elements must be strings"))
                    .map(|s| s.to_string())
            })
            .collect::<Result<Vec<_>, _>>(),
        None => Ok(Vec::new()),
    }
}

pub fn is_empty_table(t: &toml::Table) -> bool {
    t.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_resolve_path_absolute() {
        let cwd = PathBuf::from("/some/cwd");
        let path = "/absolute/path";
        let resolved = resolve_path(path, &cwd).expect("Failed to resolve path");
        assert_eq!(resolved, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_resolve_path_with_tilde() {
        let cwd = PathBuf::from("/some/cwd");
        let home = std::env::home_dir().expect("Failed to get home directory");

        // Test ~/subdir
        let path = "~/Documents";
        let resolved = resolve_path(path, &cwd).unwrap();
        assert_eq!(resolved, home.join("Documents"));

        // Test just ~
        let path = "~";
        let resolved = resolve_path(path, &cwd).unwrap();
        assert_eq!(resolved, home);
    }

    #[test]
    fn test_resolve_path_relative() {
        let cwd = PathBuf::from("/some/cwd");
        let path = "relative/path";
        let resolved = resolve_path(path, &cwd).unwrap();

        // Should be absolute path based on cwd
        assert!(resolved.is_absolute());
        assert!(resolved.ends_with("relative/path"));
    }

    #[test]
    fn test_resolve_path_dot_relative() {
        let cwd = PathBuf::from("/some/cwd");
        let path = "./file.txt";
        let resolved = resolve_path(path, &cwd).unwrap();

        assert!(resolved.is_absolute());
        assert!(resolved.ends_with("file.txt"));
    }

    #[test]
    fn test_resolve_path_parent_relative() {
        let cwd = PathBuf::from("/some/cwd/subdir");
        let path = "../file.txt";
        let resolved = resolve_path(path, &cwd).unwrap();

        assert!(resolved.is_absolute());
    }

    #[test]
    fn test_normalize_home_path_already_normalized() {
        let path = "~/.config/nvim";
        let normalized = normalize_home_path(path);
        assert_eq!(normalized, "~/.config/nvim");
    }

    #[test]
    fn test_normalize_home_path_in_home_directory() {
        let home = std::env::home_dir().expect("Failed to get home directory");
        let home_str = home.to_string_lossy();

        // Test a path in home directory
        let path = format!("{}/.config/nvim", home_str);
        let normalized = normalize_home_path(&path);
        assert_eq!(normalized, "~/.config/nvim");
    }

    #[test]
    fn test_normalize_home_path_home_root() {
        let home = std::env::home_dir().expect("Failed to get home directory");
        let home_str = home.to_string_lossy().to_string();

        // Test the home directory itself
        let normalized = normalize_home_path(&home_str);
        assert_eq!(normalized, "~");
    }

    #[test]
    fn test_normalize_home_path_outside_home() {
        let path = "/etc/config";
        let normalized = normalize_home_path(path);
        assert_eq!(normalized, "/etc/config");

        let path = "/tmp/test";
        let normalized = normalize_home_path(path);
        assert_eq!(normalized, "/tmp/test");
    }

    #[test]
    fn test_normalize_home_path_with_trailing_slash() {
        let home = std::env::home_dir().expect("Failed to get home directory");
        let home_str = home.to_string_lossy();

        let path = format!("{}/.config/", home_str);
        let normalized = normalize_home_path(&path);
        assert_eq!(normalized, "~/.config/");
    }

    #[test]
    fn test_normalize_home_path_deep_nested() {
        let home = std::env::home_dir().expect("Failed to get home directory");
        let home_str = home.to_string_lossy();

        let path = format!("{}/a/b/c/d/e/f", home_str);
        let normalized = normalize_home_path(&path);
        assert_eq!(normalized, "~/a/b/c/d/e/f");
    }

    #[test]
    fn test_backup_ext_constant() {
        assert_eq!(BACKUP_EXT, "dotrbak");
    }

    #[test]
    fn test_resolve_path_empty_relative() {
        let cwd = PathBuf::from("/some/cwd");
        let path = "";
        let resolved = resolve_path(path, &cwd).unwrap();

        assert!(resolved.is_absolute());
    }

    #[test]
    fn test_normalize_home_path_similar_prefix() {
        // Test that paths that start with home-like prefix but aren't in home work
        let home = std::env::home_dir().expect("Failed to get home directory");
        let home_str = home.to_string_lossy();

        // Create a path that has home as substring but isn't actually in home
        let fake_path = format!("{}_fake/config", home_str);
        let normalized = normalize_home_path(&fake_path);
        // Should not be normalized since it's not actually in home
        assert_eq!(normalized, fake_path);
    }

    #[test]
    fn test_resolve_and_normalize_round_trip() {
        let cwd = PathBuf::from("/some/cwd");
        let home = std::env::home_dir().expect("Failed to get home directory");

        // Start with tilde path
        let original = "~/.bashrc";

        // Resolve it
        let resolved = resolve_path(original, &cwd).unwrap();
        assert_eq!(resolved, home.join(".bashrc"));

        // Normalize it back
        let normalized = normalize_home_path(resolved.to_string_lossy().as_ref());
        assert_eq!(normalized, original);
    }

    #[test]
    fn test_normalize_home_path_with_spaces() {
        let home = std::env::home_dir().expect("Failed to get home directory");
        let home_str = home.to_string_lossy();

        let path = format!("{}/My Documents/file.txt", home_str);
        let normalized = normalize_home_path(&path);
        assert_eq!(normalized, "~/My Documents/file.txt");
    }

    #[test]
    fn test_normalize_home_path_with_dots() {
        let home = std::env::home_dir().expect("Failed to get home directory");
        let home_str = home.to_string_lossy();

        let path = format!("{}/.config/.hidden/..dotfile", home_str);
        let normalized = normalize_home_path(&path);
        assert_eq!(normalized, "~/.config/.hidden/..dotfile");
    }
}

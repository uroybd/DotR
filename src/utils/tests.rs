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

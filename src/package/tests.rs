#[cfg(test)]
mod package_name_tests {
    use crate::cli::ImportArgs;
    use crate::package::get_pkg_name_and_rel_path;
    use std::env;

    #[test]
    fn test_get_pkg_name_and_rel_path_file_with_extension() {
        let temp_dir = env::temp_dir().join("dotr_test_pkg_name_1");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let test_file = temp_dir.join("test.conf");
        std::fs::write(&test_file, "test").unwrap();

        let args = ImportArgs {
            path: test_file.to_str().unwrap().to_string(),
            name: None,
            profile: None,
        };

        let (pkg_name, pkg_ns) = get_pkg_name_and_rel_path(&args, &temp_dir).unwrap();

        // Package name has extension with underscore, pkg_ns preserves dot
        assert_eq!(pkg_name, "f_test_conf");
        assert_eq!(pkg_ns, "f_test.conf");

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_pkg_name_and_rel_path_file_without_extension() {
        let temp_dir = env::temp_dir().join("dotr_test_pkg_name_2");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let test_file = temp_dir.join("bashrc");
        std::fs::write(&test_file, "test").unwrap();

        let args = ImportArgs {
            path: test_file.to_str().unwrap().to_string(),
            name: None,
            profile: None,
        };

        let (pkg_name, pkg_ns) = get_pkg_name_and_rel_path(&args, &temp_dir).unwrap();

        assert_eq!(pkg_name, "f_bashrc");
        assert_eq!(pkg_ns, "f_bashrc");

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_pkg_name_and_rel_path_dotfile() {
        let temp_dir = env::temp_dir().join("dotr_test_pkg_name_3");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let test_file = temp_dir.join(".bashrc");
        std::fs::write(&test_file, "test").unwrap();

        let args = ImportArgs {
            path: test_file.to_str().unwrap().to_string(),
            name: None,
            profile: None,
        };

        let (pkg_name, pkg_ns) = get_pkg_name_and_rel_path(&args, &temp_dir).unwrap();

        assert_eq!(pkg_name, "f_bashrc");
        assert_eq!(pkg_ns, "f_bashrc");

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_pkg_name_and_rel_path_directory() {
        let temp_dir = env::temp_dir().join("dotr_test_pkg_name_4");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let test_dir = temp_dir.join("nvim");
        std::fs::create_dir_all(&test_dir).unwrap();

        let args = ImportArgs {
            path: test_dir.to_str().unwrap().to_string(),
            name: None,
            profile: None,
        };

        let (pkg_name, pkg_ns) = get_pkg_name_and_rel_path(&args, &temp_dir).unwrap();

        assert_eq!(pkg_name, "d_nvim");
        assert_eq!(pkg_ns, "d_nvim");

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_pkg_name_and_rel_path_with_custom_name() {
        let temp_dir = env::temp_dir().join("dotr_test_pkg_name_5");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let test_file = temp_dir.join("init.lua");
        std::fs::write(&test_file, "test").unwrap();

        let args = ImportArgs {
            path: test_file.to_str().unwrap().to_string(),
            name: Some("starship".to_string()),
            profile: None,
        };

        let (pkg_name, pkg_ns) = get_pkg_name_and_rel_path(&args, &temp_dir).unwrap();

        assert_eq!(pkg_name, "f_starship");
        assert_eq!(pkg_ns, "f_starship.lua");

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_pkg_name_and_rel_path_with_version_number() {
        let temp_dir = env::temp_dir().join("dotr_test_pkg_name_6");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let test_file = temp_dir.join("package-1.2.3.tar.gz");
        std::fs::write(&test_file, "test").unwrap();

        let args = ImportArgs {
            path: test_file.to_str().unwrap().to_string(),
            name: None,
            profile: None,
        };

        let (pkg_name, pkg_ns) = get_pkg_name_and_rel_path(&args, &temp_dir).unwrap();

        // Version numbers are NOT removed anymore
        assert_eq!(pkg_name, "f_package_1_2_3_tar_gz");
        assert_eq!(pkg_ns, "f_package_1_2_3_tar.gz");

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_pkg_name_and_rel_path_special_chars_replaced() {
        let temp_dir = env::temp_dir().join("dotr_test_pkg_name_7");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let test_file = temp_dir.join("my-config.file.conf");
        std::fs::write(&test_file, "test").unwrap();

        let args = ImportArgs {
            path: test_file.to_str().unwrap().to_string(),
            name: None,
            profile: None,
        };

        let (pkg_name, pkg_ns) = get_pkg_name_and_rel_path(&args, &temp_dir).unwrap();

        // All '-' and '.' replaced with '_' in package name, no version removal
        assert_eq!(pkg_name, "f_my_config_file_conf");
        assert_eq!(pkg_ns, "f_my_config_file.conf");

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_pkg_name_and_rel_path_template_file() {
        let temp_dir = env::temp_dir().join("dotr_test_pkg_name_8");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let test_file = temp_dir.join(".bashrc.template");
        std::fs::write(&test_file, "test").unwrap();

        let args = ImportArgs {
            path: test_file.to_str().unwrap().to_string(),
            name: None,
            profile: None,
        };

        let (pkg_name, pkg_ns) = get_pkg_name_and_rel_path(&args, &temp_dir).unwrap();

        // .template extension included with underscore in name, dot in pkg_ns
        assert_eq!(pkg_name, "f_bashrc_template");
        assert_eq!(pkg_ns, "f_bashrc.template");

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_pkg_name_and_rel_path_custom_name_no_version_removal() {
        let temp_dir = env::temp_dir().join("dotr_test_pkg_name_9");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let test_file = temp_dir.join("package-1.2.3.conf");
        std::fs::write(&test_file, "test").unwrap();

        let args = ImportArgs {
            path: test_file.to_str().unwrap().to_string(),
            name: Some("my-custom-name".to_string()),
            profile: None,
        };

        let (pkg_name, pkg_ns) = get_pkg_name_and_rel_path(&args, &temp_dir).unwrap();

        // With custom name, version numbers should NOT be removed, but special chars replaced
        assert_eq!(pkg_name, "f_my_custom_name");
        assert_eq!(pkg_ns, "f_my_custom_name.conf");

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_get_pkg_name_and_rel_path_dotfile_with_extension() {
        let temp_dir = env::temp_dir().join("dotr_test_pkg_name_10");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let test_file = temp_dir.join(".config.yml");
        std::fs::write(&test_file, "test").unwrap();

        let args = ImportArgs {
            path: test_file.to_str().unwrap().to_string(),
            name: None,
            profile: None,
        };

        let (pkg_name, pkg_ns) = get_pkg_name_and_rel_path(&args, &temp_dir).unwrap();

        // Leading '.' removed, extension with underscore in name, dot in pkg_ns
        assert_eq!(pkg_name, "f_config_yml");
        assert_eq!(pkg_ns, "f_config.yml");

        std::fs::remove_dir_all(&temp_dir).ok();
    }
}

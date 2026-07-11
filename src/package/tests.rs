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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
        };

        let (pkg_name, pkg_ns) = get_pkg_name_and_rel_path(&args, &temp_dir).unwrap();

        // Leading '.' removed, extension with underscore in name, dot in pkg_ns
        assert_eq!(pkg_name, "f_config_yml");
        assert_eq!(pkg_ns, "f_config.yml");

        std::fs::remove_dir_all(&temp_dir).ok();
    }
}

#[cfg(test)]
mod resolve_dest_tests {
    use crate::context::Context;
    use crate::package::Package;
    use crate::profile::Profile;
    use std::env;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = env::temp_dir().join(format!("dotr_test_resolve_dest_{}", name));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn ctx_with_profile(working_dir: &std::path::Path, profile: Profile) -> Context {
        let config = crate::config::Config::new();
        let (mut ctx, _) = Context::new(working_dir, &config, &None, false).unwrap();
        ctx.set_profile(profile);
        ctx
    }

    #[test]
    fn falls_back_to_dest_with_no_targets_or_platform() {
        let dir = temp_dir("1");
        let pkg = Package::new("pkg", "src", "/dest/default");
        let ctx = ctx_with_profile(&dir, Profile::new("work"));

        let resolved = pkg.resolve_dest(&ctx).unwrap();

        assert_eq!(resolved, std::path::PathBuf::from("/dest/default"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn profile_name_keyed_target_is_used_when_platform_is_unset() {
        let dir = temp_dir("2");
        let mut pkg = Package::new("pkg", "src", "/dest/default");
        pkg.targets
            .insert("work".to_string(), "/dest/work".to_string());
        let ctx = ctx_with_profile(&dir, Profile::new("work"));

        let resolved = pkg.resolve_dest(&ctx).unwrap();

        assert_eq!(resolved, std::path::PathBuf::from("/dest/work"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn platform_keyed_target_is_used_when_no_profile_name_match() {
        let dir = temp_dir("3");
        let mut pkg = Package::new("pkg", "src", "/dest/default");
        pkg.targets
            .insert("macos".to_string(), "/dest/macos".to_string());
        let profile = Profile {
            name: "work-laptop".to_string(),
            platform: Some("macos".to_string()),
            ..Default::default()
        };
        let ctx = ctx_with_profile(&dir, profile);

        let resolved = pkg.resolve_dest(&ctx).unwrap();

        assert_eq!(resolved, std::path::PathBuf::from("/dest/macos"));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A profile-specific target is more specific than a platform-shared one,
    /// so it must win even when both are defined for the same package -
    /// otherwise there'd be no way to override the shared platform path for
    /// just one profile.
    #[test]
    fn profile_name_target_takes_precedence_over_platform_target() {
        let dir = temp_dir("4");
        let mut pkg = Package::new("pkg", "src", "/dest/default");
        pkg.targets
            .insert("macos".to_string(), "/dest/macos-shared".to_string());
        pkg.targets
            .insert("work-laptop".to_string(), "/dest/work-only".to_string());
        let profile = Profile {
            name: "work-laptop".to_string(),
            platform: Some("macos".to_string()),
            ..Default::default()
        };
        let ctx = ctx_with_profile(&dir, profile);

        let resolved = pkg.resolve_dest(&ctx).unwrap();

        assert_eq!(resolved, std::path::PathBuf::from("/dest/work-only"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn falls_back_to_dest_when_platform_set_but_no_matching_target() {
        let dir = temp_dir("5");
        let pkg = Package::new("pkg", "src", "/dest/default");
        let profile = Profile {
            name: "work-laptop".to_string(),
            platform: Some("macos".to_string()),
            ..Default::default()
        };
        let ctx = ctx_with_profile(&dir, profile);

        let resolved = pkg.resolve_dest(&ctx).unwrap();

        assert_eq!(resolved, std::path::PathBuf::from("/dest/default"));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Two different profiles sharing one platform both resolve to the same
    /// platform-keyed target - the actual use case the `platform` field
    /// exists for (sharing a destination across profiles without repeating
    /// it under each profile's own name).
    #[test]
    fn two_profiles_sharing_a_platform_resolve_to_the_same_target() {
        let dir = temp_dir("6");
        let mut pkg = Package::new("pkg", "src", "/dest/default");
        pkg.targets
            .insert("linux".to_string(), "/dest/linux-shared".to_string());

        let home_profile = Profile {
            name: "home".to_string(),
            platform: Some("linux".to_string()),
            ..Default::default()
        };
        let work_profile = Profile {
            name: "work".to_string(),
            platform: Some("linux".to_string()),
            ..Default::default()
        };

        let home_ctx = ctx_with_profile(&dir, home_profile);
        let work_ctx = ctx_with_profile(&dir, work_profile);

        assert_eq!(
            pkg.resolve_dest(&home_ctx).unwrap(),
            std::path::PathBuf::from("/dest/linux-shared")
        );
        assert_eq!(
            pkg.resolve_dest(&work_ctx).unwrap(),
            std::path::PathBuf::from("/dest/linux-shared")
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn platform_keyed_target_is_templated() {
        let dir = temp_dir("7");
        let mut pkg = Package::new("pkg", "src", "/dest/default");
        pkg.targets
            .insert("macos".to_string(), "{{ HOME_LABEL }}/target".to_string());
        pkg.variables.insert(
            "HOME_LABEL".to_string(),
            toml::Value::String("/dest/rendered".to_string()),
        );
        let profile = Profile {
            name: "work-laptop".to_string(),
            platform: Some("macos".to_string()),
            ..Default::default()
        };
        let ctx = ctx_with_profile(&dir, profile);

        let resolved = pkg.resolve_dest(&ctx).unwrap();

        assert_eq!(resolved, std::path::PathBuf::from("/dest/rendered/target"));
        std::fs::remove_dir_all(&dir).ok();
    }
}

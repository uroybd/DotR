#[cfg(test)]
mod filter_packages_tests {
    use crate::config::Config;
    use crate::context::Context;
    use crate::package::Package;
    use crate::profile::Profile;
    use std::env;

    #[test]
    fn test_filter_packages_respects_skip_flag() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();

        // Create a package with skip=true
        let mut pkg1 = Package::new("pkg1", "/src1", "/dest1");
        pkg1.skip = true;

        // Create a package with skip=false
        let pkg2 = Package::new("pkg2", "/src2", "/dest2");

        config.packages.insert("pkg1".to_string(), pkg1);
        config.packages.insert("pkg2".to_string(), pkg2);

        // Create a profile with both packages
        let mut profile = Profile::new("test-profile");
        profile.dependencies.push("pkg1".to_string());
        profile.dependencies.push("pkg2".to_string());

        config.profiles.insert("test-profile".to_string(), profile);

        let (ctx, _) =
            Context::new(&temp_dir, &config, &Some("test-profile".to_string()), false).unwrap();

        let filtered = config.filter_packages(&ctx.profile, &None, false).unwrap();

        // Should only contain pkg2 since pkg1 has skip=true
        assert_eq!(filtered.len(), 1);
        assert!(filtered.contains_key("pkg2"));
        assert!(!filtered.contains_key("pkg1"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_filter_packages_with_no_skip_packages() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();

        let pkg1 = Package::new("pkg1", "/src1", "/dest1");
        let pkg2 = Package::new("pkg2", "/src2", "/dest2");

        config.packages.insert("pkg1".to_string(), pkg1);
        config.packages.insert("pkg2".to_string(), pkg2);

        let mut profile = Profile::new("test-profile");
        profile.dependencies.push("pkg1".to_string());
        profile.dependencies.push("pkg2".to_string());

        config.profiles.insert("test-profile".to_string(), profile);

        let (ctx, _) =
            Context::new(&temp_dir, &config, &Some("test-profile".to_string()), false).unwrap();

        let filtered = config.filter_packages(&ctx.profile, &None, false).unwrap();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains_key("pkg1"));
        assert!(filtered.contains_key("pkg2"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_filter_packages_all_skipped() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();

        let mut pkg1 = Package::new("pkg1", "/src1", "/dest1");
        pkg1.skip = true;
        let mut pkg2 = Package::new("pkg2", "/src2", "/dest2");
        pkg2.skip = true;

        config.packages.insert("pkg1".to_string(), pkg1);
        config.packages.insert("pkg2".to_string(), pkg2);

        let mut profile = Profile::new("test-profile");
        profile.dependencies.push("pkg1".to_string());
        profile.dependencies.push("pkg2".to_string());

        config.profiles.insert("test-profile".to_string(), profile);

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let filtered = config.filter_packages(&ctx.profile, &None, false).unwrap();

        assert_eq!(filtered.len(), 0);

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_filter_packages_specific_packages_override_skip() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();

        let mut pkg1 = Package::new("pkg1", "/src1", "/dest1");
        pkg1.skip = true;
        let pkg2 = Package::new("pkg2", "/src2", "/dest2");

        config.packages.insert("pkg1".to_string(), pkg1);
        config.packages.insert("pkg2".to_string(), pkg2);

        let mut profile = Profile::new("test-profile");
        profile.dependencies.push("pkg1".to_string());
        profile.dependencies.push("pkg2".to_string());

        config.profiles.insert("test-profile".to_string(), profile);

        let (ctx, _) =
            Context::new(&temp_dir, &config, &Some("test-profile".to_string()), false).unwrap();

        // Explicitly request pkg1 even though it has skip=true
        let specific_packages = vec!["pkg1".to_string()];
        let filtered = config
            .filter_packages(&ctx.profile, &Some(specific_packages), false)
            .unwrap();

        // When specifically requested, skip flag is ignored
        assert_eq!(filtered.len(), 1);
        assert!(filtered.contains_key("pkg1"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_filter_packages_includes_dependencies_by_default() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();

        let pkg1 = Package::new("pkg1", "/src1", "/dest1");
        let mut pkg2 = Package::new("pkg2", "/src2", "/dest2");
        pkg2.dependencies = Some(vec!["pkg1".to_string()]);

        config.packages.insert("pkg1".to_string(), pkg1);
        config.packages.insert("pkg2".to_string(), pkg2);

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        // Only request pkg2 explicitly; pkg1 should be pulled in as a dependency
        let filtered = config
            .filter_packages(&ctx.profile, &Some(vec!["pkg2".to_string()]), false)
            .unwrap();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains_key("pkg1"));
        assert!(filtered.contains_key("pkg2"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_filter_packages_ignore_dependencies_excludes_deps() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();

        let pkg1 = Package::new("pkg1", "/src1", "/dest1");
        let mut pkg2 = Package::new("pkg2", "/src2", "/dest2");
        pkg2.dependencies = Some(vec!["pkg1".to_string()]);

        config.packages.insert("pkg1".to_string(), pkg1);
        config.packages.insert("pkg2".to_string(), pkg2);

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        // With ignore_dependencies=true, pkg1 should NOT be pulled in
        let filtered = config
            .filter_packages(&ctx.profile, &Some(vec!["pkg2".to_string()]), true)
            .unwrap();

        assert_eq!(filtered.len(), 1);
        assert!(filtered.contains_key("pkg2"));
        assert!(!filtered.contains_key("pkg1"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_filter_packages_ignore_dependencies_with_profile() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();

        let pkg1 = Package::new("pkg1", "/src1", "/dest1");
        let mut pkg2 = Package::new("pkg2", "/src2", "/dest2");
        pkg2.dependencies = Some(vec!["pkg1".to_string()]);

        config.packages.insert("pkg1".to_string(), pkg1);
        config.packages.insert("pkg2".to_string(), pkg2);

        let mut profile = Profile::new("test-profile");
        profile.dependencies.push("pkg2".to_string());
        config.profiles.insert("test-profile".to_string(), profile);

        let (ctx, _) =
            Context::new(&temp_dir, &config, &Some("test-profile".to_string()), false).unwrap();

        // No explicit package names (profile-driven selection), ignore_dependencies=true
        let filtered = config.filter_packages(&ctx.profile, &None, true).unwrap();

        assert_eq!(filtered.len(), 1);
        assert!(filtered.contains_key("pkg2"));
        assert!(!filtered.contains_key("pkg1"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }
}

#[cfg(test)]
mod print_stats_tests {
    use crate::config::{OpType, print_stats};
    use crate::package::BackupDeployResult;
    use std::collections::HashMap;

    #[test]
    fn test_print_stats_all_success_backup() {
        let mut stats = HashMap::new();
        stats.insert(BackupDeployResult::Success, 5);

        // Just verify it doesn't panic - output testing would require capturing stdout
        print_stats(&stats, OpType::Backup);
    }

    #[test]
    fn test_print_stats_all_success_deploy() {
        let mut stats = HashMap::new();
        stats.insert(BackupDeployResult::Success, 3);

        print_stats(&stats, OpType::Deploy);
    }

    #[test]
    fn test_print_stats_mixed_results_backup() {
        let mut stats = HashMap::new();
        stats.insert(BackupDeployResult::Success, 10);
        stats.insert(BackupDeployResult::Skipped, 2);
        stats.insert(BackupDeployResult::Failed, 1);

        print_stats(&stats, OpType::Backup);
    }

    #[test]
    fn test_print_stats_mixed_results_deploy() {
        let mut stats = HashMap::new();
        stats.insert(BackupDeployResult::Success, 8);
        stats.insert(BackupDeployResult::Skipped, 3);
        stats.insert(BackupDeployResult::Failed, 2);

        print_stats(&stats, OpType::Deploy);
    }

    #[test]
    fn test_print_stats_only_skipped() {
        let mut stats = HashMap::new();
        stats.insert(BackupDeployResult::Skipped, 5);

        print_stats(&stats, OpType::Deploy);
    }

    #[test]
    fn test_print_stats_only_failed_backup() {
        let mut stats = HashMap::new();
        stats.insert(BackupDeployResult::Failed, 3);

        print_stats(&stats, OpType::Backup);
    }

    #[test]
    fn test_print_stats_only_failed_deploy() {
        let mut stats = HashMap::new();
        stats.insert(BackupDeployResult::Failed, 4);

        print_stats(&stats, OpType::Deploy);
    }

    #[test]
    fn test_print_stats_empty_backup() {
        let stats = HashMap::new();

        print_stats(&stats, OpType::Backup);
    }

    #[test]
    fn test_print_stats_empty_deploy() {
        let stats = HashMap::new();

        print_stats(&stats, OpType::Deploy);
    }

    #[test]
    fn test_print_stats_skipped_and_failed() {
        let mut stats = HashMap::new();
        stats.insert(BackupDeployResult::Skipped, 2);
        stats.insert(BackupDeployResult::Failed, 1);

        print_stats(&stats, OpType::Deploy);
    }

    #[test]
    fn test_print_stats_success_and_skipped() {
        let mut stats = HashMap::new();
        stats.insert(BackupDeployResult::Success, 7);
        stats.insert(BackupDeployResult::Skipped, 3);

        print_stats(&stats, OpType::Backup);
    }

    #[test]
    fn test_print_stats_success_and_failed() {
        let mut stats = HashMap::new();
        stats.insert(BackupDeployResult::Success, 5);
        stats.insert(BackupDeployResult::Failed, 2);

        print_stats(&stats, OpType::Deploy);
    }

    #[test]
    fn test_print_stats_large_numbers() {
        let mut stats = HashMap::new();
        stats.insert(BackupDeployResult::Success, 100);
        stats.insert(BackupDeployResult::Skipped, 50);
        stats.insert(BackupDeployResult::Failed, 10);

        print_stats(&stats, OpType::Backup);
    }

    #[test]
    fn test_print_stats_single_success() {
        let mut stats = HashMap::new();
        stats.insert(BackupDeployResult::Success, 1);

        print_stats(&stats, OpType::Deploy);
    }

    #[test]
    fn test_print_stats_single_failed() {
        let mut stats = HashMap::new();
        stats.insert(BackupDeployResult::Failed, 1);

        print_stats(&stats, OpType::Backup);
    }
}

#[cfg(test)]
mod remove_packages_tests {
    use crate::cli::RemovePackageArgs;
    use crate::config::Config;
    use crate::context::Context;
    use crate::package::Package;
    use crate::profile::Profile;
    use std::env;

    #[test]
    fn test_remove_single_package_success() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::create_dir_all(temp_dir.join("dotfiles")).unwrap();

        let mut config = Config::new();
        let pkg = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg);

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = RemovePackageArgs {
            packages: Some(vec!["pkg1".to_string()]),
            force: false,
            remove_orphans: false,
            dry_run: false,
            profile: None,
        };

        let result = config.remove_packages(&args, &ctx);
        assert!(result.is_ok());
        assert!(!config.packages.contains_key("pkg1"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_package_not_found() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();
        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = RemovePackageArgs {
            packages: Some(vec!["nonexistent".to_string()]),
            force: false,
            remove_orphans: false,
            dry_run: false,
            profile: None,
        };

        let result = config.remove_packages(&args, &ctx);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not found in configuration")
        );

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_package_with_profile_dependency() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();
        let pkg = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg);

        let mut profile = Profile::new("test-profile");
        profile.dependencies.push("pkg1".to_string());
        config.profiles.insert("test-profile".to_string(), profile);

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = RemovePackageArgs {
            packages: Some(vec!["pkg1".to_string()]),
            force: false,
            remove_orphans: false,
            dry_run: false,
            profile: None,
        };

        let result = config.remove_packages(&args, &ctx);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("depended on by profiles")
        );

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_package_with_dependency_force() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::create_dir_all(temp_dir.join("dotfiles")).unwrap();

        let mut config = Config::new();
        let pkg = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg);

        let mut profile = Profile::new("test-profile");
        profile.dependencies.push("pkg1".to_string());
        config
            .profiles
            .insert("test-profile".to_string(), profile.clone());

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = RemovePackageArgs {
            packages: Some(vec!["pkg1".to_string()]),
            force: true,
            remove_orphans: false,
            dry_run: false,
            profile: None,
        };

        let result = config.remove_packages(&args, &ctx);
        assert!(result.is_ok());
        assert!(!config.packages.contains_key("pkg1"));
        // Verify dependency was removed from profile
        assert!(
            !config
                .profiles
                .get("test-profile")
                .unwrap()
                .dependencies
                .contains(&"pkg1".to_string())
        );

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_package_with_package_dependency() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();
        let pkg1 = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg1);

        let mut pkg2 = Package::new("pkg2", "dotfiles/pkg2", "/dest2");
        pkg2.dependencies = Some(vec!["pkg1".to_string()]);
        config.packages.insert("pkg2".to_string(), pkg2);

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = RemovePackageArgs {
            packages: Some(vec!["pkg1".to_string()]),
            force: false,
            remove_orphans: false,
            dry_run: false,
            profile: None,
        };

        let result = config.remove_packages(&args, &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("depended on by"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_multiple_packages() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::create_dir_all(temp_dir.join("dotfiles")).unwrap();

        let mut config = Config::new();
        config.packages.insert(
            "pkg1".to_string(),
            Package::new("pkg1", "dotfiles/pkg1", "/dest1"),
        );
        config.packages.insert(
            "pkg2".to_string(),
            Package::new("pkg2", "dotfiles/pkg2", "/dest2"),
        );

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = RemovePackageArgs {
            packages: Some(vec!["pkg1".to_string(), "pkg2".to_string()]),
            force: false,
            remove_orphans: false,
            dry_run: false,
            profile: None,
        };

        let result = config.remove_packages(&args, &ctx);
        assert!(result.is_ok());
        assert!(!config.packages.contains_key("pkg1"));
        assert!(!config.packages.contains_key("pkg2"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_package_dry_run() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();
        let pkg = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg);

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = RemovePackageArgs {
            packages: Some(vec!["pkg1".to_string()]),
            force: false,
            remove_orphans: false,
            dry_run: true,
            profile: None,
        };

        let result = config.remove_packages(&args, &ctx);
        assert!(result.is_ok());
        // Package should still exist in dry run mode
        assert!(config.packages.contains_key("pkg1"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_orphan_packages() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::create_dir_all(temp_dir.join("dotfiles")).unwrap();

        let mut config = Config::new();
        // Create an orphan package (not referenced by any profile)
        config.packages.insert(
            "orphan".to_string(),
            Package::new("orphan", "dotfiles/orphan", "/dest"),
        );
        // Create a package referenced by default profile
        let mut profile = config.profiles.get_mut("default").unwrap().clone();
        profile.dependencies.push("used".to_string());
        config.profiles.insert("default".to_string(), profile);
        config.packages.insert(
            "used".to_string(),
            Package::new("used", "dotfiles/used", "/dest2"),
        );

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = RemovePackageArgs {
            packages: Some(vec![]), // Empty list to trigger only orphan removal
            force: false,
            remove_orphans: true,
            dry_run: false,
            profile: None,
        };

        let result = config.remove_packages(&args, &ctx);
        assert!(result.is_ok());
        assert!(!config.packages.contains_key("orphan"));
        assert!(config.packages.contains_key("used"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_package_with_file_cleanup() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let pkg_dir = temp_dir.join("dotfiles").join("pkg1");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join("file.txt"), "content").unwrap();

        let mut config = Config::new();
        let pkg = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg);

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = RemovePackageArgs {
            packages: Some(vec!["pkg1".to_string()]),
            force: false,
            remove_orphans: false,
            dry_run: false,
            profile: None,
        };

        let result = config.remove_packages(&args, &ctx);
        assert!(result.is_ok());
        assert!(!pkg_dir.exists());

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_no_packages_specified() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();
        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = RemovePackageArgs {
            packages: None,
            force: false,
            remove_orphans: false,
            dry_run: false,
            profile: None,
        };

        let result = config.remove_packages(&args, &ctx);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No packages specified")
        );

        std::fs::remove_dir_all(&temp_dir).ok();
    }
}

#[cfg(test)]
mod remove_profile_tests {
    use crate::cli::ProfileRemoveArgs;
    use crate::config::Config;
    use crate::context::Context;
    use crate::package::Package;
    use crate::profile::Profile;
    use std::env;

    #[test]
    fn test_remove_profile_success() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();
        let profile = Profile::new("test-profile");
        config.profiles.insert("test-profile".to_string(), profile);

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = ProfileRemoveArgs {
            name: "test-profile".to_string(),
            dry_run: false,
            remove_orphans: false,
        };

        let result = config.remove_profile(&args, &ctx);
        assert!(result.is_ok());
        assert!(!config.profiles.contains_key("test-profile"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_profile_not_found() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();
        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = ProfileRemoveArgs {
            name: "nonexistent".to_string(),
            dry_run: false,
            remove_orphans: false,
        };

        let result = config.remove_profile(&args, &ctx);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not found in configuration")
        );

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_default_profile_fails() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();
        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = ProfileRemoveArgs {
            name: "default".to_string(),
            dry_run: false,
            remove_orphans: false,
        };

        let result = config.remove_profile(&args, &ctx);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Cannot remove the default profile")
        );

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_profile_dry_run() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();
        let profile = Profile::new("test-profile");
        config.profiles.insert("test-profile".to_string(), profile);

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = ProfileRemoveArgs {
            name: "test-profile".to_string(),
            dry_run: true,
            remove_orphans: false,
        };

        let result = config.remove_profile(&args, &ctx);
        assert!(result.is_ok());
        // Profile should still exist in dry run mode
        assert!(config.profiles.contains_key("test-profile"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_profile_with_orphan_cleanup() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        std::fs::create_dir_all(temp_dir.join("dotfiles")).unwrap();

        let mut config = Config::new();

        // Create a profile with a package
        let mut profile = Profile::new("test-profile");
        profile.dependencies.push("pkg1".to_string());
        config.profiles.insert("test-profile".to_string(), profile);

        let pkg = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg);

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = ProfileRemoveArgs {
            name: "test-profile".to_string(),
            dry_run: false,
            remove_orphans: true,
        };

        let result = config.remove_profile(&args, &ctx);
        assert!(result.is_ok());
        assert!(!config.profiles.contains_key("test-profile"));
        // Package should be removed as orphan
        assert!(!config.packages.contains_key("pkg1"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_profile_without_orphan_cleanup() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();

        // Create a profile with a package
        let mut profile = Profile::new("test-profile");
        profile.dependencies.push("pkg1".to_string());
        config.profiles.insert("test-profile".to_string(), profile);

        let pkg = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg);

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = ProfileRemoveArgs {
            name: "test-profile".to_string(),
            dry_run: false,
            remove_orphans: false,
        };

        let result = config.remove_profile(&args, &ctx);
        assert!(result.is_ok());
        assert!(!config.profiles.contains_key("test-profile"));
        // Package should NOT be removed without orphan cleanup
        assert!(config.packages.contains_key("pkg1"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_remove_profile_keeps_shared_packages() {
        let temp_dir = env::temp_dir().join(format!("dotr_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut config = Config::new();

        // Create two profiles sharing a package
        let mut profile1 = Profile::new("profile1");
        profile1.dependencies.push("shared".to_string());
        config.profiles.insert("profile1".to_string(), profile1);

        let mut profile2 = Profile::new("profile2");
        profile2.dependencies.push("shared".to_string());
        config.profiles.insert("profile2".to_string(), profile2);

        let pkg = Package::new("shared", "dotfiles/shared", "/dest");
        config.packages.insert("shared".to_string(), pkg);

        let (ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let args = ProfileRemoveArgs {
            name: "profile1".to_string(),
            dry_run: false,
            remove_orphans: true,
        };

        let result = config.remove_profile(&args, &ctx);
        assert!(result.is_ok());
        assert!(!config.profiles.contains_key("profile1"));
        // Shared package should still exist because profile2 uses it
        assert!(config.packages.contains_key("shared"));

        std::fs::remove_dir_all(&temp_dir).ok();
    }
}

#[cfg(test)]
mod package_safety_tests {
    use crate::config::Config;
    use crate::package::Package;
    use crate::profile::Profile;

    #[test]
    fn test_is_package_safe_to_remove_no_dependencies() {
        let mut config = Config::new();
        let pkg = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg);

        let (is_safe, dep_profiles, dep_packages) =
            config.is_package_safe_to_remove("pkg1", &[], &[]);

        assert!(is_safe);
        assert!(dep_profiles.is_empty());
        assert!(dep_packages.is_empty());
    }

    #[test]
    fn test_is_package_safe_to_remove_profile_dependency() {
        let mut config = Config::new();
        let pkg = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg);

        let mut profile = Profile::new("test-profile");
        profile.dependencies.push("pkg1".to_string());
        config.profiles.insert("test-profile".to_string(), profile);

        let (is_safe, dep_profiles, dep_packages) =
            config.is_package_safe_to_remove("pkg1", &[], &[]);

        assert!(!is_safe);
        assert_eq!(dep_profiles, vec!["test-profile"]);
        assert!(dep_packages.is_empty());
    }

    #[test]
    fn test_is_package_safe_to_remove_package_dependency() {
        let mut config = Config::new();
        let pkg1 = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg1);

        let mut pkg2 = Package::new("pkg2", "dotfiles/pkg2", "/dest2");
        pkg2.dependencies = Some(vec!["pkg1".to_string()]);
        config.packages.insert("pkg2".to_string(), pkg2);

        let (is_safe, dep_profiles, dep_packages) =
            config.is_package_safe_to_remove("pkg1", &[], &[]);

        assert!(!is_safe);
        assert!(dep_profiles.is_empty());
        assert_eq!(dep_packages, vec!["pkg2"]);
    }

    #[test]
    fn test_is_package_safe_to_remove_with_ignored_profile() {
        let mut config = Config::new();
        let pkg = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg);

        let mut profile = Profile::new("test-profile");
        profile.dependencies.push("pkg1".to_string());
        config.profiles.insert("test-profile".to_string(), profile);

        let (is_safe, dep_profiles, dep_packages) =
            config.is_package_safe_to_remove("pkg1", &["test-profile".to_string()], &[]);

        assert!(is_safe);
        assert!(dep_profiles.is_empty());
        assert!(dep_packages.is_empty());
    }

    #[test]
    fn test_is_package_safe_to_remove_with_ignored_package() {
        let mut config = Config::new();
        let pkg1 = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg1);

        let mut pkg2 = Package::new("pkg2", "dotfiles/pkg2", "/dest2");
        pkg2.dependencies = Some(vec!["pkg1".to_string()]);
        config.packages.insert("pkg2".to_string(), pkg2);

        let (is_safe, dep_profiles, dep_packages) =
            config.is_package_safe_to_remove("pkg1", &[], &["pkg2".to_string()]);

        assert!(is_safe);
        assert!(dep_profiles.is_empty());
        assert!(dep_packages.is_empty());
    }

    #[test]
    fn test_get_orphan_packages() {
        let mut config = Config::new();

        // Create orphan package
        let orphan = Package::new("orphan", "dotfiles/orphan", "/dest1");
        config.packages.insert("orphan".to_string(), orphan);

        // Create used package
        let used = Package::new("used", "dotfiles/used", "/dest2");
        config.packages.insert("used".to_string(), used);

        let mut profile = config.profiles.get_mut("default").unwrap().clone();
        profile.dependencies.push("used".to_string());
        config.profiles.insert("default".to_string(), profile);

        let orphans = config.get_orphan_packages();

        assert_eq!(orphans.len(), 1);
        assert!(orphans.contains(&"orphan".to_string()));
    }

    #[test]
    fn test_get_orphan_packages_package_depends_on_package() {
        let mut config = Config::new();

        // Create two packages where pkg2 depends on pkg1
        let pkg1 = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg1);

        let mut pkg2 = Package::new("pkg2", "dotfiles/pkg2", "/dest2");
        pkg2.dependencies = Some(vec!["pkg1".to_string()]);
        config.packages.insert("pkg2".to_string(), pkg2);

        // Add pkg2 to default profile
        let mut profile = config.profiles.get_mut("default").unwrap().clone();
        profile.dependencies.push("pkg2".to_string());
        config.profiles.insert("default".to_string(), profile);

        let orphans = config.get_orphan_packages();

        // pkg1 is not an orphan because pkg2 depends on it
        // pkg2 is not an orphan because default profile uses it
        assert_eq!(orphans.len(), 0);
    }

    #[test]
    fn test_get_orphan_packages_all_used() {
        let mut config = Config::new();

        let pkg1 = Package::new("pkg1", "dotfiles/pkg1", "/dest1");
        config.packages.insert("pkg1".to_string(), pkg1);

        let pkg2 = Package::new("pkg2", "dotfiles/pkg2", "/dest2");
        config.packages.insert("pkg2".to_string(), pkg2);

        let mut profile = config.profiles.get_mut("default").unwrap().clone();
        profile.dependencies.push("pkg1".to_string());
        profile.dependencies.push("pkg2".to_string());
        config.profiles.insert("default".to_string(), profile);

        let orphans = config.get_orphan_packages();

        assert_eq!(orphans.len(), 0);
    }
}

#[cfg(test)]
mod platforms_tests {
    use crate::config::Config;

    #[test]
    fn from_table_parses_platforms() {
        let toml_str = r#"
[platforms.macos]
variables = { EDITOR = "vim" }

[platforms.linux]
variables = { EDITOR = "nano" }
"#;
        let table: toml::Table = toml_str.parse().unwrap();
        let config = Config::from_table(&table).unwrap();

        assert_eq!(config.platforms.len(), 2);
        assert_eq!(
            config
                .platforms
                .get("macos")
                .unwrap()
                .variables
                .get("EDITOR"),
            Some(&toml::Value::String("vim".to_string()))
        );
    }

    #[test]
    fn from_table_with_no_platforms_is_empty() {
        let table: toml::Table = "".parse().unwrap();
        let config = Config::from_table(&table).unwrap();

        assert!(config.platforms.is_empty());
    }

    #[test]
    fn profile_with_matching_platform_gets_platform_variables() {
        let toml_str = r#"
[platforms.macos]
variables = { EDITOR = "vim" }

[profiles.work]
platform = "macos"
"#;
        let table: toml::Table = toml_str.parse().unwrap();
        let config = Config::from_table(&table).unwrap();

        let profile = config.profiles.get("work").unwrap();
        assert_eq!(
            profile.platform_variables.get("EDITOR"),
            Some(&toml::Value::String("vim".to_string()))
        );
    }

    #[test]
    fn profile_without_platform_has_no_platform_variables() {
        let toml_str = r#"
[platforms.macos]
variables = { EDITOR = "vim" }

[profiles.work]
"#;
        let table: toml::Table = toml_str.parse().unwrap();
        let config = Config::from_table(&table).unwrap();

        let profile = config.profiles.get("work").unwrap();
        assert!(profile.platform_variables.is_empty());
    }

    #[test]
    fn profile_referencing_undefined_platform_has_no_platform_variables() {
        let toml_str = r#"
[profiles.work]
platform = "does-not-exist"
"#;
        let table: toml::Table = toml_str.parse().unwrap();
        let config = Config::from_table(&table).unwrap();

        let profile = config.profiles.get("work").unwrap();
        assert!(profile.platform_variables.is_empty());
    }

    #[test]
    fn two_profiles_sharing_a_platform_both_get_its_variables() {
        let toml_str = r#"
[platforms.macos]
variables = { EDITOR = "vim" }

[profiles.laptop]
platform = "macos"

[profiles.desktop]
platform = "macos"
"#;
        let table: toml::Table = toml_str.parse().unwrap();
        let config = Config::from_table(&table).unwrap();

        for name in ["laptop", "desktop"] {
            let profile = config.profiles.get(name).unwrap();
            assert_eq!(
                profile.platform_variables.get("EDITOR"),
                Some(&toml::Value::String("vim".to_string())),
                "profile '{}' should carry the shared platform's variables",
                name
            );
        }
    }

    #[test]
    fn from_table_parses_platform_actions() {
        let toml_str = r#"
[platforms.macos]
pre_actions = ["brew bundle"]
post_actions = ["killall Dock"]
"#;
        let table: toml::Table = toml_str.parse().unwrap();
        let config = Config::from_table(&table).unwrap();

        let macos = config.platforms.get("macos").unwrap();
        assert_eq!(macos.pre_actions, vec!["brew bundle"]);
        assert_eq!(macos.post_actions, vec!["killall Dock"]);
    }

    #[test]
    fn from_table_parses_profile_actions() {
        let toml_str = r#"
[profiles.work]
pre_actions = ["echo start"]
post_actions = ["echo done"]
"#;
        let table: toml::Table = toml_str.parse().unwrap();
        let config = Config::from_table(&table).unwrap();

        let work = config.profiles.get("work").unwrap();
        assert_eq!(work.pre_actions, vec!["echo start"]);
        assert_eq!(work.post_actions, vec!["echo done"]);
    }
}

#[cfg(test)]
mod platform_profile_actions_tests {
    use crate::cli::DeployArgs;
    use crate::config::Config;
    use crate::context::Context;
    use std::env;
    use std::fs;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = env::temp_dir().join(format!(
            "dotr_test_platform_profile_actions_{}_{}",
            name,
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn config_with_actions() -> Config {
        let toml_str = r#"
[platforms.macos]
pre_actions = ["printf 'platform-pre\n' >> log.txt"]
post_actions = ["printf 'platform-post\n' >> log.txt"]

[profiles.work]
platform = "macos"
pre_actions = ["printf 'profile-pre\n' >> log.txt"]
post_actions = ["printf 'profile-post\n' >> log.txt"]
"#;
        let table: toml::Table = toml_str.parse().unwrap();
        Config::from_table(&table).unwrap()
    }

    fn deploy_args(dry_run: bool) -> DeployArgs {
        DeployArgs {
            dry_run,
            ..Default::default()
        }
    }

    #[test]
    fn pre_actions_run_platform_before_profile() {
        let dir = temp_dir("pre-order");
        let config = config_with_actions();
        let (ctx, _) = Context::new(&dir, &config, &Some("work".to_string()), false).unwrap();

        config
            .execute_pre_actions(&ctx, &deploy_args(false))
            .unwrap();

        let log = fs::read_to_string(dir.join("log.txt")).unwrap();
        assert_eq!(log, "platform-pre\nprofile-pre\n");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn post_actions_run_profile_before_platform() {
        let dir = temp_dir("post-order");
        let config = config_with_actions();
        let (ctx, _) = Context::new(&dir, &config, &Some("work".to_string()), false).unwrap();

        config
            .execute_post_actions(&ctx, &deploy_args(false))
            .unwrap();

        let log = fs::read_to_string(dir.join("log.txt")).unwrap();
        assert_eq!(log, "profile-post\nplatform-post\n");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dry_run_does_not_execute_actions() {
        let dir = temp_dir("dry-run");
        let config = config_with_actions();
        let (ctx, _) = Context::new(&dir, &config, &Some("work".to_string()), false).unwrap();

        config
            .execute_pre_actions(&ctx, &deploy_args(true))
            .unwrap();
        config
            .execute_post_actions(&ctx, &deploy_args(true))
            .unwrap();

        assert!(
            !dir.join("log.txt").exists(),
            "dry run must not run platform/profile actions"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn actions_are_a_noop_when_profile_has_no_platform_and_no_actions() {
        let dir = temp_dir("noop");
        let toml_str = r#"
[profiles.plain]
"#;
        let table: toml::Table = toml_str.parse().unwrap();
        let config = Config::from_table(&table).unwrap();
        let (ctx, _) = Context::new(&dir, &config, &Some("plain".to_string()), false).unwrap();

        config
            .execute_pre_actions(&ctx, &deploy_args(false))
            .unwrap();
        config
            .execute_post_actions(&ctx, &deploy_args(false))
            .unwrap();

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn profile_actions_run_without_a_matching_platform_entry() {
        let dir = temp_dir("profile-only");
        let toml_str = r#"
[profiles.work]
platform = "macos"
pre_actions = ["printf 'profile-pre\n' >> log.txt"]
"#;
        let table: toml::Table = toml_str.parse().unwrap();
        let config = Config::from_table(&table).unwrap();
        let (ctx, _) = Context::new(&dir, &config, &Some("work".to_string()), false).unwrap();

        config
            .execute_pre_actions(&ctx, &deploy_args(false))
            .unwrap();

        let log = fs::read_to_string(dir.join("log.txt")).unwrap();
        assert_eq!(log, "profile-pre\n");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn platform_variables_are_available_to_actions() {
        let dir = temp_dir("action-vars");
        let toml_str = r#"
[platforms.macos]
variables = { GREETING = "hello-from-platform" }
pre_actions = ["printf '{{ GREETING }}' > out.txt"]

[profiles.work]
platform = "macos"
"#;
        let table: toml::Table = toml_str.parse().unwrap();
        let config = Config::from_table(&table).unwrap();
        let (ctx, _) = Context::new(&dir, &config, &Some("work".to_string()), false).unwrap();

        config
            .execute_pre_actions(&ctx, &deploy_args(false))
            .unwrap();

        let out = fs::read_to_string(dir.join("out.txt")).unwrap();
        assert_eq!(out, "hello-from-platform");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_failing_action_propagates_an_error() {
        let dir = temp_dir("failing");
        let toml_str = r#"
[profiles.work]
pre_actions = ["exit 1"]
"#;
        let table: toml::Table = toml_str.parse().unwrap();
        let config = Config::from_table(&table).unwrap();
        let (ctx, _) = Context::new(&dir, &config, &Some("work".to_string()), false).unwrap();

        assert!(
            config
                .execute_pre_actions(&ctx, &deploy_args(false))
                .is_err()
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn deploy_packages_skips_actions_when_no_packages_selected() {
        let dir = temp_dir("empty-deploy");
        let config = config_with_actions();
        let (ctx, _) = Context::new(&dir, &config, &Some("work".to_string()), false).unwrap();

        // `work` has no `dependencies` and we pass no explicit packages, so
        // nothing is selected - platform/profile actions must not run.
        config.deploy_packages(&ctx, &deploy_args(false)).unwrap();

        assert!(
            !dir.join("log.txt").exists(),
            "platform/profile actions must not run when there is nothing to deploy"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn skip_actions_suppresses_both_pre_and_post() {
        let dir = temp_dir("skip-actions");
        let config = config_with_actions();
        let (ctx, _) = Context::new(&dir, &config, &Some("work".to_string()), false).unwrap();

        let args = DeployArgs {
            skip_actions: true,
            ..Default::default()
        };
        config.execute_pre_actions(&ctx, &args).unwrap();
        config.execute_post_actions(&ctx, &args).unwrap();

        assert!(
            !dir.join("log.txt").exists(),
            "--skip-actions must skip platform and profile actions too"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn skip_pre_actions_suppresses_only_pre() {
        let dir = temp_dir("skip-pre");
        let config = config_with_actions();
        let (ctx, _) = Context::new(&dir, &config, &Some("work".to_string()), false).unwrap();

        let args = DeployArgs {
            skip_pre_actions: true,
            ..Default::default()
        };
        config.execute_pre_actions(&ctx, &args).unwrap();
        config.execute_post_actions(&ctx, &args).unwrap();

        let log = fs::read_to_string(dir.join("log.txt")).unwrap();
        assert_eq!(
            log, "profile-post\nplatform-post\n",
            "--skip-pre-actions must leave post actions running"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn skip_post_actions_suppresses_only_post() {
        let dir = temp_dir("skip-post");
        let config = config_with_actions();
        let (ctx, _) = Context::new(&dir, &config, &Some("work".to_string()), false).unwrap();

        let args = DeployArgs {
            skip_post_actions: true,
            ..Default::default()
        };
        config.execute_pre_actions(&ctx, &args).unwrap();
        config.execute_post_actions(&ctx, &args).unwrap();

        let log = fs::read_to_string(dir.join("log.txt")).unwrap();
        assert_eq!(
            log, "platform-pre\nprofile-pre\n",
            "--skip-post-actions must leave pre actions running"
        );

        fs::remove_dir_all(&dir).ok();
    }
}

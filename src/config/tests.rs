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

        let ctx =
            Context::new(&temp_dir, &config, &Some("test-profile".to_string()), false).unwrap();

        let filtered = config.filter_packages(&ctx, &None).unwrap();

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

        let ctx =
            Context::new(&temp_dir, &config, &Some("test-profile".to_string()), false).unwrap();

        let filtered = config.filter_packages(&ctx, &None).unwrap();

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

        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

        let filtered = config.filter_packages(&ctx, &None).unwrap();

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

        let ctx =
            Context::new(&temp_dir, &config, &Some("test-profile".to_string()), false).unwrap();

        // Explicitly request pkg1 even though it has skip=true
        let specific_packages = vec!["pkg1".to_string()];
        let filtered = config
            .filter_packages(&ctx, &Some(specific_packages))
            .unwrap();

        // When specifically requested, skip flag is ignored
        assert_eq!(filtered.len(), 1);
        assert!(filtered.contains_key("pkg1"));

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

        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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
        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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

        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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

        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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

        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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

        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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

        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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

        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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

        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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
        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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

        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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
        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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
        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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

        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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

        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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

        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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

        let ctx = Context::new(&temp_dir, &config, &None, false).unwrap();

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

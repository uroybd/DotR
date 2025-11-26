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

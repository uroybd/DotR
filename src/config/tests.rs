#[cfg(test)]
mod print_stats_tests {
    use std::collections::HashMap;
    use crate::config::{OpType, print_stats};
    use crate::package::BackupDeployResult;

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

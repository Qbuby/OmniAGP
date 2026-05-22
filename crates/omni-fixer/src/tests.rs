#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::types::*;

    #[test]
    fn test_degradation_level_progression() {
        let level = DegradationLevel::Retry;
        assert_eq!(level.next(), Some(DegradationLevel::AugmentedContext));

        let level = DegradationLevel::AugmentedContext;
        assert_eq!(level.next(), Some(DegradationLevel::TemplateFallback));

        let level = DegradationLevel::TemplateFallback;
        assert_eq!(level.next(), Some(DegradationLevel::Escalate));

        let level = DegradationLevel::Escalate;
        assert_eq!(level.next(), None);
    }

    #[test]
    fn test_fix_log_stats() {
        let mut log = FixLog::new();
        assert_eq!(log.success_rate(), 0.0);
        assert_eq!(log.avg_duration_ms(), 0);

        log.record(FixResult {
            bug_id: Uuid::new_v4(),
            success: true,
            attempts: vec![],
            final_status: FinalStatus::Fixed,
            total_duration_ms: 1000,
        });

        log.record(FixResult {
            bug_id: Uuid::new_v4(),
            success: false,
            attempts: vec![],
            final_status: FinalStatus::MaxRetriesExceeded,
            total_duration_ms: 3000,
        });

        log.record(FixResult {
            bug_id: Uuid::new_v4(),
            success: true,
            attempts: vec![],
            final_status: FinalStatus::Fixed,
            total_duration_ms: 2000,
        });

        assert_eq!(log.success_rate(), 2.0 / 3.0);
        assert_eq!(log.avg_duration_ms(), 2000);
    }

    #[test]
    fn test_bug_report_serialization() {
        let bug = BugReport {
            id: Uuid::new_v4(),
            bug_type: BugType::CodeBug,
            severity: Severity::High,
            description: "Null reference in player.gd".to_string(),
            stack_trace: Some("at player.gd:42".to_string()),
            related_files: vec!["scripts/player.gd".to_string()],
            error_message: Some("Invalid get index 'position' on base Nil".to_string()),
            context: serde_json::json!({
                "source_code": "extends CharacterBody2D\nfunc _ready():\n\tvar x = null\n\tx.position",
            }),
        };

        let json = serde_json::to_string(&bug).unwrap();
        let deserialized: BugReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.bug_type, BugType::CodeBug);
        assert_eq!(deserialized.severity, Severity::High);
        assert_eq!(deserialized.related_files.len(), 1);
    }

    #[test]
    fn test_patch_creation() {
        let patch = Patch {
            file_path: "scripts/enemy.gd".to_string(),
            original: "var speed = 0".to_string(),
            modified: "var speed: float = 100.0".to_string(),
            description: "Fixed uninitialized speed".to_string(),
        };

        assert_ne!(patch.original, patch.modified);
        assert!(!patch.file_path.is_empty());
    }
}

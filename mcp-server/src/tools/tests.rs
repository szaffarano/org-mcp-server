// Tool tests focus on the underlying org-core functionality
// since direct tool method testing requires complex MCP machinery
#[cfg(test)]
mod tests {
    use crate::core::OrgModeRouter;
    use org_core::OrgMode;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_org_files(temp_dir: &TempDir) -> std::io::Result<()> {
        let temp_path = temp_dir.path();

        // Create multiple org files
        fs::write(
            temp_path.join("notes.org"),
            r#"* Notes
Some notes content.
"#,
        )?;

        fs::write(
            temp_path.join("tasks.org"),
            r#"* TODO Task 1
:PROPERTIES:
:ID: task-123
:END:
First task content.

** DONE Subtask
Completed subtask.
"#,
        )?;

        fs::write(
            temp_path.join("projects.org"),
            r#"* Project Alpha
Alpha project details.

* Project Beta
Beta project details.
"#,
        )?;

        // Create a non-org file (should be ignored)
        fs::write(temp_path.join("readme.txt"), "Not an org file")?;

        Ok(())
    }

    #[tokio::test]
    async fn test_org_mode_router_creation() {
        let temp_dir = TempDir::new().unwrap();
        create_test_org_files(&temp_dir).unwrap();

        let router = OrgModeRouter::with_directory(temp_dir.path().to_str().unwrap());
        assert!(router.is_ok());
    }

    #[tokio::test]
    async fn test_org_mode_router_invalid_directory() {
        let result = OrgModeRouter::with_directory("/nonexistent/directory");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_org_mode_list_files_functionality() {
        let temp_dir = TempDir::new().unwrap();
        create_test_org_files(&temp_dir).unwrap();

        let router = OrgModeRouter::with_directory(temp_dir.path().to_str().unwrap()).unwrap();
        let org_mode = router.org_mode.lock().await;

        let files = org_mode.list_files().unwrap();
        assert_eq!(files.len(), 3);
        assert!(files.contains(&"notes.org".to_string()));
        assert!(files.contains(&"tasks.org".to_string()));
        assert!(files.contains(&"projects.org".to_string()));
        // Should not contain non-org files
        assert!(!files.contains(&"readme.txt".to_string()));
    }

    #[tokio::test]
    async fn test_org_mode_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let router = OrgModeRouter::with_directory(temp_dir.path().to_str().unwrap()).unwrap();
        let org_mode = router.org_mode.lock().await;

        let files = org_mode.list_files().unwrap();
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_org_mode_direct_creation() {
        let temp_dir = TempDir::new().unwrap();
        create_test_org_files(&temp_dir).unwrap();

        let org_mode = OrgMode::new(temp_dir.path().to_str().unwrap());
        assert!(org_mode.is_ok());

        let files = org_mode.unwrap().list_files().unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn test_org_mode_invalid_directory_direct() {
        let result = OrgMode::new("/nonexistent/directory");
        assert!(result.is_err());
    }
}

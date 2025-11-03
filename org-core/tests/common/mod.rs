use std::path::Path;

use org_core::{OrgConfig, OrgMode};
use tempfile::TempDir;
use test_utils::fixtures;

#[allow(dead_code)]
pub fn create_test_org_mode(path: &Path) -> OrgMode {
    let config = OrgConfig {
        org_directory: path.to_string_lossy().to_string(),
        ..OrgConfig::default()
    };
    OrgMode::new(config).expect("Failed to create test OrgMode")
}

#[allow(dead_code)]
pub fn create_test_org_mode_with_agenda_files() -> (OrgMode, TempDir) {
    let base_date = chrono::Local::now().date_naive();
    let temp_dir = fixtures::setup_test_org_files().expect("Failed to set up test org files");

    fixtures::copy_fixtures_with_dates(&temp_dir, base_date)
        .expect("Failed to copy fixtures with dates");

    let config = OrgConfig {
        org_directory: temp_dir.path().to_string_lossy().to_string(),
        org_agenda_files: vec!["agenda.org".to_string(), "project.org".to_string()],
        ..OrgConfig::default()
    };

    let org_mode = OrgMode::new(config).expect("Failed to create test OrgMode");
    (org_mode, temp_dir)
}

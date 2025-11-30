mod common;

use chrono::{Datelike, Days, Local, TimeZone};
use org_core::org_mode::AgendaViewType;
use org_core::{OrgConfig, OrgMode, OrgModeError, Priority};
use serial_test::serial;
use std::convert::TryFrom;
use test_utils::fixtures;

use common::create_test_org_mode_with_agenda_files;

#[test]
fn test_try_from_empty_string() {
    let result = AgendaViewType::try_from("");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), AgendaViewType::CurrentWeek));
}

#[test]
fn test_try_from_today() {
    let result = AgendaViewType::try_from("today");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), AgendaViewType::Today));
}

#[test]
fn test_try_from_week() {
    let result = AgendaViewType::try_from("week");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), AgendaViewType::CurrentWeek));
}

#[test]
fn test_try_from_month() {
    let result = AgendaViewType::try_from("month");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), AgendaViewType::CurrentMonth));
}

#[test]
fn test_try_from_specific_day_valid() {
    let result = AgendaViewType::try_from("day/2025-10-20");
    assert!(result.is_ok());
    if let Ok(AgendaViewType::Day(date)) = result {
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2025-10-20");
    } else {
        panic!("Expected Day variant");
    }
}

#[test]
fn test_try_from_specific_day_invalid_format() {
    let result = AgendaViewType::try_from("day/20-10-2025");
    assert!(result.is_err());
    if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
        assert!(msg.contains("Invalid date format"));
    } else {
        panic!("Expected InvalidAgendaViewType error");
    }
}

#[test]
fn test_try_from_specific_day_invalid_date() {
    let result = AgendaViewType::try_from("day/2025-13-40");
    assert!(result.is_err());
    if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
        assert!(msg.contains("Invalid date format"));
    } else {
        panic!("Expected InvalidAgendaViewType error");
    }
}

#[test]
fn test_try_from_week_number_valid() {
    let result = AgendaViewType::try_from("week/42");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), AgendaViewType::Week(42)));
}

#[test]
fn test_try_from_week_number_invalid() {
    let result = AgendaViewType::try_from("week/invalid");
    assert!(result.is_err());
    if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
        assert!(msg.contains("Invalid week number"));
    } else {
        panic!("Expected InvalidAgendaViewType error");
    }
}

#[test]
fn test_try_from_month_number_valid() {
    let result = AgendaViewType::try_from("month/6");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), AgendaViewType::Month(6)));
}

#[test]
fn test_try_from_month_number_boundary_1() {
    let result = AgendaViewType::try_from("month/1");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), AgendaViewType::Month(1)));
}

#[test]
fn test_try_from_month_number_boundary_12() {
    let result = AgendaViewType::try_from("month/12");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), AgendaViewType::Month(12)));
}

#[test]
fn test_try_from_month_number_out_of_range_zero() {
    let result = AgendaViewType::try_from("month/0");
    assert!(result.is_err());
    if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
        assert!(msg.contains("out of range"));
    } else {
        panic!("Expected InvalidAgendaViewType error");
    }
}

#[test]
fn test_try_from_month_number_out_of_range_13() {
    let result = AgendaViewType::try_from("month/13");
    assert!(result.is_err());
    if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
        assert!(msg.contains("out of range"));
    } else {
        panic!("Expected InvalidAgendaViewType error");
    }
}

#[test]
fn test_try_from_month_number_invalid() {
    let result = AgendaViewType::try_from("month/abc");
    assert!(result.is_err());
    if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
        assert!(msg.contains("Invalid month number"));
    } else {
        panic!("Expected InvalidAgendaViewType error");
    }
}

#[test]
fn test_try_from_custom_range_valid() {
    let result = AgendaViewType::try_from("query/from/2025-10-01/to/2025-10-31");
    assert!(result.is_ok());
    if let Ok(AgendaViewType::Custom { from, to }) = result {
        assert_eq!(from.format("%Y-%m-%d").to_string(), "2025-10-01");
        assert_eq!(to.format("%Y-%m-%d").to_string(), "2025-10-31");
    } else {
        panic!("Expected Custom variant");
    }
}

#[test]
fn test_try_from_custom_range_from_greather_than_to() {
    let result = AgendaViewType::try_from("query/from/2025-10-31/to/2025-10-01");
    assert!(result.is_err());
    if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
        assert!(msg.contains("From date must be before to date"));
    } else {
        panic!("Expected InvalidAgendaViewType error");
    }
}

#[test]
fn test_try_from_custom_range_invalid_from_date() {
    let result = AgendaViewType::try_from("query/from/invalid/to/2025-10-31");
    assert!(result.is_err());
    if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
        assert!(msg.contains("Invalid from date"));
    } else {
        panic!("Expected InvalidAgendaViewType error");
    }
}

#[test]
fn test_try_from_custom_range_invalid_to_date() {
    let result = AgendaViewType::try_from("query/from/2025-10-01/to/invalid");
    assert!(result.is_err());
    if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
        assert!(msg.contains("Invalid to date"));
    } else {
        panic!("Expected InvalidAgendaViewType error");
    }
}

#[test]
fn test_try_from_unknown_format() {
    let result = AgendaViewType::try_from("unknown/format");
    assert!(result.is_err());
    if let Err(OrgModeError::InvalidAgendaViewType(msg)) = result {
        assert!(msg.contains("Unknown agenda view type format"));
    } else {
        panic!("Expected InvalidAgendaViewType error");
    }
}

#[test]
fn test_start_date_and_end_date_today() {
    let view_type = AgendaViewType::Today;
    let start = view_type.start_date();
    let end = view_type.end_date();

    assert_eq!(
        start.format("%Y-%m-%d").to_string(),
        end.format("%Y-%m-%d").to_string()
    );
}

#[test]
fn test_start_date_and_end_date_specific_day() {
    let result = AgendaViewType::try_from("day/2025-06-15");
    assert!(result.is_ok());
    let view_type = result.unwrap();

    let start = view_type.start_date();
    let end = view_type.end_date();

    assert_eq!(start.format("%Y-%m-%d").to_string(), "2025-06-15");
    assert_eq!(end.format("%Y-%m-%d").to_string(), "2025-06-15");
}

#[test]
fn test_start_date_and_end_date_current_week() {
    let view_type = AgendaViewType::CurrentWeek;
    let start = view_type.start_date();
    let end = view_type.end_date();

    // Week should span 7 days
    let duration = end.signed_duration_since(start);
    assert_eq!(duration.num_days(), 6);

    // Start should be a Monday (weekday 0)
    assert_eq!(start.weekday().num_days_from_monday(), 0);
}

#[test]
fn test_start_date_and_end_date_custom_range() {
    let result = AgendaViewType::try_from("query/from/2025-03-01/to/2025-03-15");
    assert!(result.is_ok());
    let view_type = result.unwrap();

    let start = view_type.start_date();
    let end = view_type.end_date();

    assert_eq!(start.format("%Y-%m-%d").to_string(), "2025-03-01");
    assert_eq!(end.format("%Y-%m-%d").to_string(), "2025-03-15");
}

#[test]
fn test_start_date_and_end_date_current_month() {
    let view_type = AgendaViewType::CurrentMonth;
    let start = view_type.start_date();
    let end = view_type.end_date();

    // Start should be day 1 of current month
    assert_eq!(start.day(), 1);

    // Start and end should be in same month
    assert_eq!(start.month(), end.month());
    assert_eq!(start.year(), end.year());

    // End should be last day of month
    let next_month_first = if end.month() == 12 {
        chrono::Local
            .with_ymd_and_hms(end.year() + 1, 1, 1, 0, 0, 0)
            .unwrap()
    } else {
        chrono::Local
            .with_ymd_and_hms(end.year(), end.month() + 1, 1, 0, 0, 0)
            .unwrap()
    };
    let last_day_of_month = next_month_first - chrono::Duration::days(1);
    assert_eq!(end.day(), last_day_of_month.day());
}

#[test]
fn test_start_date_and_end_date_specific_month() {
    let result = AgendaViewType::try_from("month/2");
    assert!(result.is_ok(), "Month parsing failed");
    let view_type = result.unwrap();

    let start = view_type.start_date();
    let end = view_type.end_date();

    // Should be February
    assert_eq!(start.month(), 2);
    assert_eq!(end.month(), 2);

    // Start should be day 1
    assert_eq!(start.day(), 1);

    // End should be 28 or 29 depending on leap year
    assert!(end.day() == 28 || end.day() == 29);
}

#[test]
fn test_start_date_and_end_date_december() {
    let result = AgendaViewType::try_from("month/12");
    assert!(result.is_ok());
    let view_type = result.unwrap();

    let start = view_type.start_date();
    let end = view_type.end_date();

    // Should be December
    assert_eq!(start.month(), 12);
    assert_eq!(end.month(), 12);

    // Start should be day 1
    assert_eq!(start.day(), 1);

    // End should be day 31
    assert_eq!(end.day(), 31);
}

// ============================================================================
// List Tasks Tests
// ============================================================================

#[test]
fn test_list_tasks_basic() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let tasks = org_mode
        .list_tasks(None, None, None, None)
        .expect("Failed to list tasks");

    assert!(!tasks.is_empty());
    assert!(tasks.len() >= 10, "Expected at least 10 tasks");

    assert!(tasks.iter().any(|t| t.file_path.contains("agenda.org")));
    assert!(tasks.iter().any(|t| t.file_path.contains("project.org")));
}

#[test]
fn test_list_tasks_with_limit() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let tasks = org_mode
        .list_tasks(None, None, None, Some(5))
        .expect("Failed to list tasks with limit");

    assert!(tasks.len() <= 5, "Expected at most 5 tasks");
}

#[test]
fn test_list_tasks_todo_states() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let tasks = org_mode
        .list_tasks(None, None, None, None)
        .expect("Failed to list tasks");

    let has_todo = tasks
        .iter()
        .any(|t| t.todo_state == Some("TODO".to_string()));
    let has_done = tasks
        .iter()
        .any(|t| t.todo_state == Some("DONE".to_string()));

    assert!(has_todo, "Should have TODO tasks");
    assert!(!has_done, "Should not have DONE tasks");
}

#[test]
fn test_list_tasks_priorities() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let tasks = org_mode
        .list_tasks(None, None, None, None)
        .expect("Failed to list tasks");

    let has_priority_a = tasks.iter().any(|t| t.priority == Some("A".to_string()));
    let has_priority_b = tasks.iter().any(|t| t.priority == Some("B".to_string()));
    let has_priority_c = tasks.iter().any(|t| t.priority == Some("C".to_string()));

    assert!(has_priority_a, "Should have priority A tasks");
    assert!(has_priority_b, "Should have priority B tasks");
    assert!(has_priority_c, "Should have priority C tasks");
}

#[test]
fn test_list_tasks_scheduled_deadline() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let tasks = org_mode
        .list_tasks(None, None, None, None)
        .expect("Failed to list tasks");

    let has_scheduled = tasks.iter().any(|t| t.scheduled.is_some());
    let has_deadline = tasks.iter().any(|t| t.deadline.is_some());

    assert!(has_scheduled, "Should have tasks with scheduled dates");
    assert!(has_deadline, "Should have tasks with deadline dates");

    let scheduled_task = tasks.iter().find(|t| t.scheduled.is_some()).unwrap();
    assert!(
        scheduled_task.scheduled.as_ref().unwrap().contains("2025"),
        "Scheduled date should contain year"
    );
}

#[test]
fn test_list_tasks_nested_headlines() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let tasks = org_mode
        .list_tasks(None, None, None, None)
        .expect("Failed to list tasks");

    let nested_tasks = tasks.iter().filter(|t| t.level >= 3).count();

    assert!(
        nested_tasks > 0,
        "Should have nested TODO items (level >= 3)"
    );
}

#[test]
fn test_list_tasks_file_path_handling() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let tasks = org_mode
        .list_tasks(None, None, None, None)
        .expect("Failed to list tasks");

    for task in &tasks {
        assert!(
            !task.file_path.starts_with('/'),
            "File path should be relative: {}",
            task.file_path
        );
        assert!(
            task.file_path.ends_with(".org"),
            "File path should end with .org: {}",
            task.file_path
        );
    }
}

#[test]
fn test_list_tasks_custom_todo_keywords() {
    let org_dir = fixtures::setup_test_org_files().unwrap();
    let config = OrgConfig {
        org_directory: org_dir.path().to_string_lossy().to_string(),
        org_agenda_files: vec!["agenda.org".to_string()],
        org_todo_keywords: vec![
            "TODO".to_string(),
            "IN_PROGRESS".to_string(),
            "|".to_string(),
            "DONE".to_string(),
            "CANCELLED".to_string(),
        ],
        ..OrgConfig::default()
    };
    let org_mode = OrgMode::new(config).expect("Failed to create test OrgMode");

    let tasks = org_mode
        .list_tasks(None, None, None, None)
        .expect("Failed to list tasks");

    assert!(!tasks.is_empty());
    assert!(tasks.iter().any(|t| t.todo_state.is_some()));
}

#[test]
fn test_list_tasks_empty_agenda_files() {
    let org_dir = fixtures::setup_test_org_files().unwrap();
    let config = OrgConfig {
        org_directory: org_dir.path().to_string_lossy().to_string(),
        org_agenda_files: vec!["empty.org".to_string()],
        ..OrgConfig::default()
    };
    let org_mode = OrgMode::new(config).expect("Failed to create test OrgMode");

    let tasks = org_mode
        .list_tasks(None, None, None, None)
        .expect("Failed to list tasks");

    assert!(tasks.is_empty(), "Empty file should have no tasks");
}

#[test]
fn test_list_tasks_glob_patterns() {
    let org_dir = fixtures::setup_test_org_files().unwrap();
    let config = OrgConfig {
        org_directory: org_dir.path().to_string_lossy().to_string(),
        org_agenda_files: vec!["*.org".to_string()],
        ..OrgConfig::default()
    };
    let org_mode = OrgMode::new(config).expect("Failed to create test OrgMode");

    let tasks = org_mode
        .list_tasks(None, None, None, None)
        .expect("Failed to list tasks");

    assert!(!tasks.is_empty());
    assert!(tasks.len() >= 10, "Should find tasks from multiple files");
}

#[test]
fn test_list_tasks_specific_heading_content() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let tasks = org_mode
        .list_tasks(None, None, None, None)
        .expect("Failed to list tasks");

    let quarterly_report = tasks
        .iter()
        .find(|t| t.heading.contains("Complete quarterly report"));

    assert!(
        quarterly_report.is_some(),
        "Should find quarterly report task"
    );

    let task = quarterly_report.unwrap();
    assert_eq!(task.todo_state, Some("TODO".to_string()));
    assert_eq!(task.priority, Some("A".to_string()));
    assert!(task.scheduled.is_some());
    assert!(task.deadline.is_some());
}

#[test]
fn test_list_tasks_limit_zero() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let tasks = org_mode
        .list_tasks(None, None, None, Some(0))
        .expect("Failed to list tasks");

    assert!(tasks.is_empty(), "Limit of 0 should return no tasks");
}

#[test]
fn test_list_tasks_with_state_filter() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

    let todo_tasks = org_mode
        .list_tasks(Some(&["TODO".to_string()]), None, None, None)
        .expect("Failed to list TODO tasks");

    assert!(!todo_tasks.is_empty(), "Should have TODO tasks");
    for task in &todo_tasks {
        assert_eq!(
            task.todo_state.as_deref(),
            Some("TODO"),
            "Task '{}' should have TODO state",
            task.heading
        );
    }
    assert!(
        todo_tasks.len() >= 10,
        "Expected at least 10 TODO tasks, got {}",
        todo_tasks.len()
    );

    let done_tasks = org_mode
        .list_tasks(Some(&["DONE".to_string()]), None, None, None)
        .expect("Failed to list DONE tasks");

    assert!(done_tasks.is_empty(), "Should not have DONE tasks");
}

#[test]
#[serial]
fn test_list_tasks_with_tag_filter() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

    // Filter by "work" tag
    let work_tasks = org_mode
        .list_tasks(None, Some(&["work".to_string()]), None, None)
        .expect("Failed to list tasks with work tag");

    assert!(
        work_tasks.len() >= 2,
        "Should have at least 2 tasks with 'work' tag, got {}",
        work_tasks.len()
    );
    // Verify all returned tasks have the 'work' tag
    for task in &work_tasks {
        assert!(
            task.tags.contains(&"work".to_string()),
            "Task '{}' should have 'work' tag, has {:?}",
            task.heading,
            task.tags
        );
    }

    // Filter by "personal" tag
    let personal_tasks = org_mode
        .list_tasks(None, Some(&["personal".to_string()]), None, None)
        .expect("Failed to list tasks with personal tag");

    assert!(
        !personal_tasks.is_empty(),
        "Should have at least 1 task with 'personal' tag, got {}",
        personal_tasks.len()
    );
    // Verify all returned tasks have the 'personal' tag
    for task in &personal_tasks {
        assert!(
            task.tags.contains(&"personal".to_string()),
            "Task '{}' should have 'personal' tag, has {:?}",
            task.heading,
            task.tags
        );
    }

    // Filter by "urgent" tag (should find tasks with urgent tag)
    let urgent_tasks = org_mode
        .list_tasks(None, Some(&["urgent".to_string()]), None, None)
        .expect("Failed to list tasks with urgent tag");

    assert!(
        !urgent_tasks.is_empty(),
        "Should have at least 1 task with 'urgent' tag, got {}",
        urgent_tasks.len()
    );
    for task in &urgent_tasks {
        assert!(
            task.tags.contains(&"urgent".to_string()),
            "Task '{}' should have 'urgent' tag, has {:?}",
            task.heading,
            task.tags
        );
    }
}

#[test]
fn test_list_tasks_with_priority_filter() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

    // Test Priority A filter
    let a_tasks = org_mode
        .list_tasks(None, None, Some(Priority::A), None)
        .expect("Failed to list priority A tasks");

    assert!(
        a_tasks.len() >= 2,
        "Should have at least 2 priority A tasks, got {}",
        a_tasks.len()
    );
    for task in &a_tasks {
        assert_eq!(
            task.priority.as_deref(),
            Some("A"),
            "Task '{}' should have priority A, has {:?}",
            task.heading,
            task.priority
        );
    }

    // Test Priority B filter
    let b_tasks = org_mode
        .list_tasks(None, None, Some(Priority::B), None)
        .expect("Failed to list priority B tasks");

    assert!(
        !b_tasks.is_empty(),
        "Should have at least 1 priority B task, got {}",
        b_tasks.len()
    );
    for task in &b_tasks {
        assert_eq!(
            task.priority.as_deref(),
            Some("B"),
            "Task '{}' should have priority B, has {:?}",
            task.heading,
            task.priority
        );
    }

    // Test Priority C filter
    let c_tasks = org_mode
        .list_tasks(None, None, Some(Priority::C), None)
        .expect("Failed to list priority C tasks");

    // Note: "Update documentation" is DONE with priority C, but we filter TODO items
    // So we might have 0 or more C priority tasks depending on fixtures
    for task in &c_tasks {
        assert_eq!(
            task.priority.as_deref(),
            Some("C"),
            "Task '{}' should have priority C, has {:?}",
            task.heading,
            task.priority
        );
    }

    // Test Priority::None filter (tasks with no priority)
    let no_priority_tasks = org_mode
        .list_tasks(None, None, Some(Priority::None), None)
        .expect("Failed to list tasks with no priority");

    assert!(
        !no_priority_tasks.is_empty(),
        "Should have tasks with no priority"
    );
    for task in &no_priority_tasks {
        assert!(
            task.priority.is_none(),
            "Task '{}' should have no priority, has {:?}",
            task.heading,
            task.priority
        );
    }
}

#[test]
fn test_list_tasks_combined_filters() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

    // Combine TODO state + priority filter
    let todo_a_tasks = org_mode
        .list_tasks(Some(&["TODO".to_string()]), None, Some(Priority::A), None)
        .expect("Failed to list TODO tasks with priority A");

    assert!(
        todo_a_tasks.len() >= 2,
        "Should have TODO tasks with priority A, got {}",
        todo_a_tasks.len()
    );
    for task in &todo_a_tasks {
        assert_eq!(task.todo_state.as_deref(), Some("TODO"));
        assert_eq!(task.priority.as_deref(), Some("A"));
    }

    // Combine TODO state + tag filter
    let todo_work_tasks = org_mode
        .list_tasks(
            Some(&["TODO".to_string()]),
            Some(&["work".to_string()]),
            None,
            None,
        )
        .expect("Failed to list TODO work tasks");

    assert!(
        todo_work_tasks.len() >= 2,
        "Should have TODO work tasks, got {}",
        todo_work_tasks.len()
    );
    for task in &todo_work_tasks {
        assert_eq!(task.todo_state.as_deref(), Some("TODO"));
        assert!(task.tags.contains(&"work".to_string()));
    }

    // Combine all three filters: TODO + work + urgent
    let todo_work_urgent = org_mode
        .list_tasks(
            Some(&["TODO".to_string()]),
            Some(&["work".to_string(), "urgent".to_string()]),
            None,
            None,
        )
        .expect("Failed to list TODO work+urgent tasks");

    // This should find "Code review session" which has both work and urgent tags
    assert!(
        !todo_work_urgent.is_empty(),
        "Should have at least 1 TODO work+urgent task, got {}",
        todo_work_urgent.len()
    );
    for task in &todo_work_urgent {
        assert_eq!(task.todo_state.as_deref(), Some("TODO"));
        assert!(task.tags.contains(&"work".to_string()));
        assert!(task.tags.contains(&"urgent".to_string()));
    }
}

#[test]
fn test_list_tasks_multiple_tags() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

    // Task "Code review session" has tags ["work", "review", "urgent"]
    // Filtering by ["review", "urgent"] should find it
    let review_urgent_tasks = org_mode
        .list_tasks(
            None,
            Some(&["review".to_string(), "urgent".to_string()]),
            None,
            None,
        )
        .expect("Failed to list tasks with review+urgent tags");

    assert!(
        !review_urgent_tasks.is_empty(),
        "Should have at least 1 task with review+urgent tags, got {}",
        review_urgent_tasks.len()
    );
    for task in &review_urgent_tasks {
        assert!(task.tags.contains(&"review".to_string()));
        assert!(task.tags.contains(&"urgent".to_string()));
    }

    // Filtering by ["work", "review"] should find tasks with both tags
    let work_review_tasks = org_mode
        .list_tasks(
            None,
            Some(&["work".to_string(), "review".to_string()]),
            None,
            None,
        )
        .expect("Failed to list tasks with work+review tags");

    assert!(
        !work_review_tasks.is_empty(),
        "Should have tasks with work+review tags, got {}",
        work_review_tasks.len()
    );
    for task in &work_review_tasks {
        assert!(task.tags.contains(&"work".to_string()));
        assert!(task.tags.contains(&"review".to_string()));
    }
}

#[test]
fn test_list_tasks_no_match_filters() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

    // Filter by non-existent tag
    let nonexistent_tag_tasks = org_mode
        .list_tasks(None, Some(&["nonexistent".to_string()]), None, None)
        .expect("Failed to list tasks with nonexistent tag");

    assert!(
        nonexistent_tag_tasks.is_empty(),
        "Should have no tasks with nonexistent tag, got {}",
        nonexistent_tag_tasks.len()
    );

    // Filter by combination that doesn't exist (DONE + Priority A)
    // In our fixtures, Priority A tasks are all TODO
    let done_a_tasks = org_mode
        .list_tasks(Some(&["DONE".to_string()]), None, Some(Priority::A), None)
        .expect("Failed to list DONE priority A tasks");

    assert!(
        done_a_tasks.is_empty(),
        "Should have no DONE tasks with priority A, got {}",
        done_a_tasks.len()
    );

    // Filter by impossible combination (personal + work tags together on same task)
    let personal_work_tasks = org_mode
        .list_tasks(
            None,
            Some(&["personal".to_string(), "work".to_string()]),
            None,
            None,
        )
        .expect("Failed to list tasks with personal+work tags");

    assert!(
        personal_work_tasks.is_empty(),
        "Should have no tasks with both personal and work tags, got {}",
        personal_work_tasks.len()
    );
}

#[test]
fn test_get_agenda_view_today() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let view = org_mode
        .get_agenda_view(AgendaViewType::Today, None, None)
        .expect("Failed to get today's agenda view");

    assert!(
        view.start_date.is_some(),
        "Today view should have start_date"
    );
    assert!(view.end_date.is_some(), "Today view should have end_date");

    // With dynamic dates, we should find tasks scheduled for today
    // Based on our fixture: "@TODAY@" tasks include "Review pull requests" and "Team standup"
    assert!(
        view.items.len() >= 2,
        "Should find at least 2 tasks scheduled for today, found {}",
        view.items.len()
    );

    // Verify we're finding the expected tasks
    let has_review_prs = view
        .items
        .iter()
        .any(|item| item.heading.contains("Review pull requests"));
    let has_standup = view
        .items
        .iter()
        .any(|item| item.heading.contains("Team standup"));

    assert!(
        has_review_prs || has_standup,
        "Should find at least one of today's scheduled tasks"
    );
}

#[test]
fn test_get_agenda_view_current_week() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let view = org_mode
        .get_agenda_view(AgendaViewType::CurrentWeek, None, None)
        .expect("Failed to get current week agenda view");

    assert!(
        view.start_date.is_some(),
        "Week view should have start_date"
    );
    assert!(view.end_date.is_some(), "Week view should have end_date");

    // Week view should find at least some tasks
    // Number varies depending on which day of the week "today" is
    assert!(
        !view.items.is_empty(),
        "Should find at least 1 task in current week, found {}",
        view.items.len()
    );

    // Verify we're finding tasks scheduled for the current week
    // At minimum, @TODAY@ tasks should be included
    let task_names: Vec<String> = view.items.iter().map(|i| i.heading.clone()).collect();
    let has_weekly_task = task_names
        .iter()
        .any(|name| name.contains("Review pull requests") || name.contains("Team standup"));

    assert!(
        has_weekly_task,
        "Should find tasks from the current week (at least @TODAY@ tasks)"
    );
}

#[test]
fn test_get_agenda_view_custom_week() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let view = org_mode
        .get_agenda_view(AgendaViewType::Week(9), None, None)
        .expect("Failed to get current week agenda view");

    assert!(
        view.start_date.is_some(),
        "Week view should have start_date"
    );
    assert!(view.end_date.is_some(), "Week view should have end_date");

    // TODO: complete once filters are implemented
}

#[test]
fn test_get_agenda_view_custom_range() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

    // Test with a custom range from TODAY+1 to TODAY+6 (should match several fixture tasks)
    let today = Local::now();
    let from = today.checked_add_days(Days::new(1)).unwrap();
    let to = today.checked_add_days(Days::new(6)).unwrap();

    let view = org_mode
        .get_agenda_view(AgendaViewType::Custom { from, to }, None, None)
        .expect("Failed to get custom range agenda view");

    assert!(
        view.start_date.is_some(),
        "Custom view should have start_date"
    );
    assert!(view.end_date.is_some(), "Custom view should have end_date");

    // Should find tasks scheduled in this range
    // Based on fixtures: @TODAY+1@, @TODAY+2@, @TODAY+3@, @TODAY+4@, @TODAY+5@, @TODAY+6@
    assert!(
        view.items.len() >= 3,
        "Should find at least 3 tasks in custom range, found {}",
        view.items.len()
    );

    // Verify we find expected tasks in this range
    let has_quarterly_report = view
        .items
        .iter()
        .any(|item| item.heading.contains("Complete quarterly report"));

    assert!(
        has_quarterly_report,
        "Should find 'Complete quarterly report' task scheduled for TODAY+1"
    );
}

#[test]
fn test_get_agenda_view_with_filters() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

    let _view = org_mode
        .get_agenda_view(
            AgendaViewType::Today,
            Some(&["TODO".to_string()]),
            Some(&["work".to_string()]),
        )
        .expect("Failed to get filtered agenda view");

    // TODO: complete once filters are implemented
}

#[test]
fn test_get_agenda_view_empty_results() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();

    let from = Local.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap();
    let to = Local.with_ymd_and_hms(2030, 1, 7, 23, 59, 59).unwrap();

    let view = org_mode
        .get_agenda_view(AgendaViewType::Custom { from, to }, None, None)
        .expect("Failed to get agenda view");

    // Far future dates should have no tasks
    assert!(
        view.items.is_empty(),
        "Should have no tasks in far future, found {}",
        view.items.len()
    );
}

#[test]
fn test_agenda_today_finds_scheduled_tasks() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let view = org_mode
        .get_agenda_view(AgendaViewType::Today, None, None)
        .expect("Failed to get today's agenda");

    // Verify tasks scheduled for @TODAY@ are found
    let review_task = view
        .items
        .iter()
        .find(|item| item.heading.contains("Review pull requests"));

    assert!(
        review_task.is_some(),
        "Should find 'Review pull requests' task scheduled for today"
    );

    if let Some(task) = review_task {
        assert_eq!(task.todo_state, Some("TODO".to_string()));
        assert!(task.tags.contains(&"work".to_string()));
        assert!(task.scheduled.is_some());
    }
}

#[test]
fn test_agenda_today_excludes_future_tasks() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let view = org_mode
        .get_agenda_view(AgendaViewType::Today, None, None)
        .expect("Failed to get today's agenda");

    // Tasks scheduled for @TODAY+1@ or later should not be in today's view
    let has_future_task = view
        .items
        .iter()
        .any(|item| item.heading.contains("Buy groceries")); // Scheduled for @TODAY+2@

    assert!(
        !has_future_task,
        "Today's view should not include tasks scheduled for future dates"
    );
}

#[test]
fn test_agenda_week_includes_all_week_tasks() {
    let (org_mode, _temp_dir) = create_test_org_mode_with_agenda_files();
    let view = org_mode
        .get_agenda_view(AgendaViewType::CurrentWeek, None, None)
        .expect("Failed to get current week agenda");

    // Week should include tasks from the current week (Monday through Sunday)
    let task_headings: Vec<String> = view.items.iter().map(|i| i.heading.clone()).collect();

    // At minimum, should include @TODAY@ tasks
    assert!(
        !task_headings.is_empty(),
        "Week view should include at least one task"
    );

    // Should include today's tasks
    assert!(
        task_headings
            .iter()
            .any(|h| h.contains("Review pull requests") || h.contains("Team standup")),
        "Week view should include today's tasks (@TODAY@)"
    );

    // Note: Future tasks (@TODAY+1@, etc.) may or may not be in the current week
    // depending on which day of the week "today" is. If today is Sunday,
    // then @TODAY+1@ tasks are in next week, not this week.
}

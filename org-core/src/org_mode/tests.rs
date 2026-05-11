use super::*;
use crate::OrgModeError;
use crate::config::OrgConfig;
use orgize::Org;
use orgize::export::{Container, Event, from_fn};
use std::fs;

fn make_org_mode(temp_dir: &tempfile::TempDir) -> OrgMode {
    OrgMode::new(OrgConfig {
        org_directory: temp_dir.path().to_str().unwrap().to_string(),
        ..OrgConfig::default()
    })
    .unwrap()
}

fn capture_minimal(file: &str, title: &str) -> CaptureEntry {
    CaptureEntry {
        title: title.to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some(file.to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    }
}

#[test]
fn test_capture_append_to_new_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "New Note".to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("new_file.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry).unwrap();
    assert_eq!(result.file_path, "new_file.org");
    assert_eq!(result.level, 1);
    assert_eq!(result.heading_line, "* New Note");

    let content = fs::read_to_string(temp_dir.path().join("new_file.org")).unwrap();
    assert!(content.contains("* New Note"));
}

#[test]
fn test_capture_append_to_existing_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(
        temp_dir.path().join("existing.org"),
        "* First Heading\nSome content.\n",
    )
    .unwrap();

    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Second Heading".to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("existing.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry).unwrap();
    assert_eq!(result.heading_line, "* Second Heading");

    let content = fs::read_to_string(temp_dir.path().join("existing.org")).unwrap();
    assert!(content.contains("* First Heading"));
    assert!(content.contains("* Second Heading"));
}

#[test]
fn test_capture_append_under_target_heading() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(
        temp_dir.path().join("test.org"),
        "* Projects\nIntro text.\n* Archive\nOld stuff.\n",
    )
    .unwrap();

    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "New Project".to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: Some("Projects".to_string()),
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry).unwrap();
    assert_eq!(result.level, 2);
    assert_eq!(result.under_target, Some("Projects".to_string()));

    let content = fs::read_to_string(temp_dir.path().join("test.org")).unwrap();
    let proj_pos = content.find("* Projects").unwrap();
    let new_pos = content.find("** New Project").unwrap();
    let archive_pos = content.find("* Archive").unwrap();
    assert!(new_pos > proj_pos);
    assert!(new_pos < archive_pos);
}

#[test]
fn test_capture_uses_default_notes_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Default Note".to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: None,
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry).unwrap();
    assert_eq!(result.file_path, "notes.org");

    let content = fs::read_to_string(temp_dir.path().join("notes.org")).unwrap();
    assert!(content.contains("* Default Note"));
}

#[test]
fn test_capture_with_body() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Note with Body".to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: None,
        body: Some("This is the body content.".to_string()),
        file: Some("body_test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry).unwrap();
    assert_eq!(result.heading_line, "* Note with Body");

    let content = fs::read_to_string(temp_dir.path().join("body_test.org")).unwrap();
    assert!(content.contains("* Note with Body"));
    assert!(content.contains("This is the body content."));
}

#[test]
fn test_capture_invalid_todo_keyword() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Task".to_string(),
        level: None,
        todo_state: Some("INVALID".to_string()),
        tags: None,
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry);
    assert!(result.is_err());
    match result.unwrap_err() {
        OrgModeError::InvalidTodoKeyword(kw) => assert_eq!(kw, "INVALID"),
        e => panic!("Expected InvalidTodoKeyword, got: {e:?}"),
    }
}

#[test]
fn test_capture_invalid_priority() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Task".to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: Some("X".to_string()),
        body: None,
        file: Some("test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry);
    assert!(result.is_err());
    match result.unwrap_err() {
        OrgModeError::InvalidPriority(p) => assert_eq!(p, "X"),
        e => panic!("Expected InvalidPriority, got: {e:?}"),
    }
}

#[test]
fn test_capture_creates_missing_heading_path() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "My Task".to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: Some("Projects/Work".to_string()),
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry).unwrap();
    assert_eq!(result.level, 3);
    assert_eq!(result.under_target, Some("Projects/Work".to_string()));

    let content = fs::read_to_string(temp_dir.path().join("test.org")).unwrap();
    assert!(content.contains("* Projects"));
    assert!(content.contains("** Work"));
    assert!(content.contains("*** My Task"));
}

#[test]
fn test_capture_creates_partial_heading_path() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(
        temp_dir.path().join("test.org"),
        "* Projects\nIntro text.\n",
    )
    .unwrap();

    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "My Task".to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: Some("Projects/Work".to_string()),
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry).unwrap();
    assert_eq!(result.level, 3);

    let content = fs::read_to_string(temp_dir.path().join("test.org")).unwrap();
    assert!(content.contains("* Projects"));
    assert!(content.contains("** Work"));
    assert!(content.contains("*** My Task"));
}

#[test]
fn test_capture_creates_heading_path_with_explicit_level() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "My Task".to_string(),
        level: Some(4),
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: Some("A/B".to_string()),
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry).unwrap();
    assert_eq!(result.level, 4);

    let content = fs::read_to_string(temp_dir.path().join("test.org")).unwrap();
    assert!(content.contains("** A"));
    assert!(content.contains("*** B"));
    assert!(content.contains("**** My Task"));
}

#[test]
fn test_capture_creates_missing_with_existing_parent_and_explicit_level() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(temp_dir.path().join("test.org"), "** A\nContent.\n").unwrap();

    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "My Task".to_string(),
        level: Some(4),
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: Some("A/B".to_string()),
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry).unwrap();
    assert_eq!(result.level, 4);

    let content = fs::read_to_string(temp_dir.path().join("test.org")).unwrap();
    assert!(content.contains("** A"));
    assert!(content.contains("*** B"));
    assert!(content.contains("**** My Task"));
}

#[test]
fn test_capture_with_explicit_level() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Deep Heading".to_string(),
        level: Some(3),
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("level_test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry).unwrap();
    assert_eq!(result.level, 3);
    assert_eq!(result.heading_line, "*** Deep Heading");
}

#[test]
fn test_capture_full_heading() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Important Task".to_string(),
        level: Some(2),
        todo_state: Some("TODO".to_string()),
        tags: Some(vec!["work".to_string(), "urgent".to_string()]),
        priority: Some("A".to_string()),
        body: Some("Deadline is tomorrow.".to_string()),
        file: Some("full_test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry).unwrap();
    assert_eq!(
        result.heading_line,
        "** TODO [#A] Important Task :work:urgent:"
    );

    let content = fs::read_to_string(temp_dir.path().join("full_test.org")).unwrap();
    assert!(content.contains("** TODO [#A] Important Task :work:urgent:"));
    assert!(content.contains("Deadline is tomorrow."));
}

#[test]
fn test_capture_target_heading_does_not_match_wrong_parent() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(temp_dir.path().join("test.org"), "* A\n* B\n** Work\n").unwrap();

    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Item".to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: Some("A/Work".to_string()),
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry).unwrap();
    let content = fs::read_to_string(temp_dir.path().join("test.org")).unwrap();

    assert_eq!(
        result.level, 3,
        "Item must end up at level 3 (A>Work>Item), got file:\n{content}"
    );
    assert!(
        content.contains("* B\n** Work"),
        "B's ** Work must remain intact:\n{content}"
    );
    let a_pos = content.find("* A").unwrap();
    let b_pos = content.find("* B").unwrap();
    let item_pos = content.find("*** Item").expect("Item must be at level 3");
    assert!(
        item_pos > a_pos && item_pos < b_pos,
        "Item must be inserted under A (between A and B), got:\n{content}"
    );
}

#[test]
fn test_capture_rejects_empty_title() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "".to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry);
    assert!(matches!(result, Err(OrgModeError::InvalidTitle(_))));
}

#[test]
fn test_capture_rejects_whitespace_title() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "   ".to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry);
    assert!(matches!(result, Err(OrgModeError::InvalidTitle(_))));
}

#[test]
fn test_capture_rejects_newline_in_title() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Line1\nLine2".to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry);
    assert!(matches!(result, Err(OrgModeError::InvalidTitle(_))));
}

#[test]
fn test_capture_rejects_carriage_return_in_title() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Line1\rLine2".to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry);
    assert!(matches!(result, Err(OrgModeError::InvalidTitle(_))));
}

#[test]
fn test_capture_rejects_level_zero() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Title".to_string(),
        level: Some(0),
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry);
    assert!(matches!(result, Err(OrgModeError::InvalidLevel(0))));
}

#[test]
fn test_capture_rejects_level_too_deep() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Title".to_string(),
        level: Some(100),
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry);
    assert!(matches!(result, Err(OrgModeError::InvalidLevel(100))));
}

#[test]
fn test_capture_rejects_tag_with_space() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Title".to_string(),
        level: None,
        todo_state: None,
        tags: Some(vec!["bad tag".to_string()]),
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry);
    assert!(matches!(result, Err(OrgModeError::InvalidTag(_))));
}

#[test]
fn test_capture_rejects_tag_with_colon() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Title".to_string(),
        level: None,
        todo_state: None,
        tags: Some(vec!["with:colon".to_string()]),
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry);
    assert!(matches!(result, Err(OrgModeError::InvalidTag(_))));
}

#[test]
fn test_capture_rejects_empty_tag() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Title".to_string(),
        level: None,
        todo_state: None,
        tags: Some(vec!["".to_string()]),
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    let result = org_mode.capture_append(entry);
    assert!(matches!(result, Err(OrgModeError::InvalidTag(_))));
}

#[test]
fn test_capture_accepts_valid_tag_chars() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Title".to_string(),
        level: None,
        todo_state: None,
        tags: Some(vec!["work_2025".to_string(), "@home".to_string()]),
        priority: None,
        body: None,
        file: Some("test.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    org_mode.capture_append(entry).unwrap();
}

#[test]
fn test_capture_cleans_up_lock_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let entry = CaptureEntry {
        title: "Note".to_string(),
        level: None,
        todo_state: None,
        tags: None,
        priority: None,
        body: None,
        file: Some("notes.org".to_string()),
        target_heading: None,
        scheduled: None,
        deadline: None,
        closed: None,
        properties: None,
        datetree: false,
        datetree_date: None,
    };

    org_mode.capture_append(entry).unwrap();

    let lock_path = temp_dir.path().join(".notes.org.lock");
    assert!(
        !lock_path.exists(),
        "lockfile must be removed after capture: {lock_path:?}"
    );
    assert!(temp_dir.path().join("notes.org").exists());
}

#[test]
fn test_capture_concurrent_writes_preserve_all_entries() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = Arc::new(make_org_mode(&temp_dir));
    let n = 20;

    let handles: Vec<_> = (0..n)
        .map(|i| {
            let om = Arc::clone(&org_mode);
            thread::spawn(move || {
                let entry = CaptureEntry {
                    title: format!("Note {i}"),
                    level: None,
                    todo_state: None,
                    tags: None,
                    priority: None,
                    body: None,
                    file: Some("concurrent.org".to_string()),
                    target_heading: None,
                    scheduled: None,
                    deadline: None,
                    closed: None,
                    properties: None,
                    datetree: false,
                    datetree_date: None,
                };
                om.capture_append(entry).unwrap();
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }

    let content = fs::read_to_string(temp_dir.path().join("concurrent.org")).unwrap();
    for i in 0..n {
        assert!(
            content.contains(&format!("* Note {i}")),
            "Note {i} missing from concurrent.org:\n{content}"
        );
    }
}

#[test]
fn test_capture_rejects_path_traversal_via_dotdot() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let mut entry = capture_minimal("../outside/foo.org", "Escape");
    entry.file = Some("../outside/foo.org".to_string());
    let err = org_mode.capture_append(entry).unwrap_err();
    assert!(matches!(err, OrgModeError::InvalidDirectory(_)));

    let parent_of_org = temp_dir.path().parent().unwrap();
    assert!(
        !parent_of_org.join("outside").exists(),
        "create_dir_all must not have escaped org_directory"
    );
}

#[test]
fn test_capture_rejects_absolute_file_path() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let mut entry = capture_minimal("/tmp/somewhere/foo.org", "Abs");
    entry.file = Some("/tmp/somewhere/foo.org".to_string());
    let err = org_mode.capture_append(entry).unwrap_err();
    assert!(matches!(err, OrgModeError::InvalidDirectory(_)));
}

#[test]
fn test_capture_bumps_level_on_fully_matched_target() {
    let temp_dir = tempfile::tempdir().unwrap();
    fs::write(temp_dir.path().join("bump.org"), "* Parent\n** Sub\n").unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let mut entry = capture_minimal("bump.org", "Child");
    entry.target_heading = Some("Parent/Sub".to_string());
    entry.level = Some(1);
    let result = org_mode.capture_append(entry).unwrap();
    assert_eq!(result.level, 3, "level must be bumped to parent_level + 1");

    let content = fs::read_to_string(temp_dir.path().join("bump.org")).unwrap();
    assert!(
        content.contains("*** Child"),
        "Child must be at level 3:\n{content}"
    );
}

#[test]
fn test_capture_rejects_empty_target_heading_segments() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    for bad in ["A//B", "/A", "A/", "  /B"] {
        let mut entry = capture_minimal("test.org", "X");
        entry.target_heading = Some(bad.to_string());
        let err = org_mode.capture_append(entry).unwrap_err();
        assert!(
            matches!(err, OrgModeError::InvalidHeadingPath(_)),
            "expected InvalidHeadingPath for '{bad}', got {err:?}"
        );
    }
}

#[test]
fn test_capture_with_scheduled() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let mut entry = capture_minimal("planning.org", "Plan stuff");
    entry.scheduled = Some("2026-05-15".to_string());
    org_mode.capture_append(entry).unwrap();

    let content = fs::read_to_string(temp_dir.path().join("planning.org")).unwrap();
    assert!(
        content.contains("SCHEDULED: <2026-05-15 Fri>"),
        "missing SCHEDULED line:\n{content}"
    );
}

#[test]
fn test_capture_with_deadline_and_time() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let mut entry = capture_minimal("planning.org", "Ship");
    entry.deadline = Some("2026-05-20 17:00".to_string());
    org_mode.capture_append(entry).unwrap();

    let content = fs::read_to_string(temp_dir.path().join("planning.org")).unwrap();
    assert!(
        content.contains("DEADLINE: <2026-05-20 Wed 17:00>"),
        "missing DEADLINE line:\n{content}"
    );
}

#[test]
fn test_capture_with_closed_inactive_brackets() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let mut entry = capture_minimal("planning.org", "Done thing");
    entry.closed = Some("2026-05-10".to_string());
    org_mode.capture_append(entry).unwrap();

    let content = fs::read_to_string(temp_dir.path().join("planning.org")).unwrap();
    assert!(
        content.contains("CLOSED: [2026-05-10 Sun]"),
        "missing CLOSED line:\n{content}"
    );
}

#[test]
fn test_capture_with_all_planning_fields() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let mut entry = capture_minimal("planning.org", "Triple");
    entry.scheduled = Some("2026-05-15".to_string());
    entry.deadline = Some("2026-05-20 17:00".to_string());
    entry.closed = Some("2026-05-10".to_string());
    org_mode.capture_append(entry).unwrap();

    let content = fs::read_to_string(temp_dir.path().join("planning.org")).unwrap();
    let want =
        "SCHEDULED: <2026-05-15 Fri> DEADLINE: <2026-05-20 Wed 17:00> CLOSED: [2026-05-10 Sun]";
    assert!(
        content.contains(want),
        "missing combined planning line:\n{content}\nwanted: {want}"
    );
}

#[test]
fn test_capture_rejects_invalid_scheduled() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let mut entry = capture_minimal("planning.org", "Bad");
    entry.scheduled = Some("tomorrow".to_string());
    let err = org_mode.capture_append(entry).unwrap_err();
    match err {
        OrgModeError::InvalidTimestamp { field, .. } => assert_eq!(field, "scheduled"),
        other => panic!("expected InvalidTimestamp for scheduled, got {other:?}"),
    }
}

#[test]
fn test_capture_with_properties() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_with_no_auto = OrgConfig {
        org_directory: temp_dir.path().to_str().unwrap().to_string(),
        org_auto_created_property: false,
        ..OrgConfig::default()
    };
    let org_mode = OrgMode::new(config_with_no_auto).unwrap();

    let mut entry = capture_minimal("p.org", "T");
    entry.properties = Some(vec![
        PropertyPair {
            key: "CATEGORY".into(),
            value: "project".into(),
        },
        PropertyPair {
            key: "EFFORT".into(),
            value: "1h".into(),
        },
    ]);
    org_mode.capture_append(entry).unwrap();

    let content = fs::read_to_string(temp_dir.path().join("p.org")).unwrap();
    let drawer_start = content.find(":PROPERTIES:").expect("drawer start");
    let cat_pos = content.find(":CATEGORY: project").expect("category");
    let eff_pos = content.find(":EFFORT: 1h").expect("effort");
    let drawer_end = content.find(":END:").expect("drawer end");
    assert!(drawer_start < cat_pos);
    assert!(cat_pos < eff_pos, "keys must preserve order");
    assert!(eff_pos < drawer_end);
}

#[test]
fn test_capture_empty_properties_omits_drawer() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = OrgConfig {
        org_directory: temp_dir.path().to_str().unwrap().to_string(),
        org_auto_created_property: false,
        ..OrgConfig::default()
    };
    let org_mode = OrgMode::new(config).unwrap();

    let mut entry = capture_minimal("p.org", "T");
    entry.properties = Some(vec![]);
    org_mode.capture_append(entry).unwrap();

    let content = fs::read_to_string(temp_dir.path().join("p.org")).unwrap();
    assert!(!content.contains(":PROPERTIES:"));
}

#[test]
fn test_capture_rejects_property_key_with_colon() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let mut entry = capture_minimal("p.org", "T");
    entry.properties = Some(vec![PropertyPair {
        key: "BAD:KEY".into(),
        value: "v".into(),
    }]);
    let err = org_mode.capture_append(entry).unwrap_err();
    assert!(matches!(err, OrgModeError::InvalidPropertyKey(_)));
}

#[test]
fn test_capture_rejects_property_value_with_newline() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let mut entry = capture_minimal("p.org", "T");
    entry.properties = Some(vec![PropertyPair {
        key: "K".into(),
        value: "line1\nline2".into(),
    }]);
    let err = org_mode.capture_append(entry).unwrap_err();
    assert!(matches!(err, OrgModeError::InvalidPropertyValue { .. }));
}

#[test]
fn test_capture_rejects_duplicate_property_keys() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let mut entry = capture_minimal("p.org", "T");
    entry.properties = Some(vec![
        PropertyPair {
            key: "K".into(),
            value: "v1".into(),
        },
        PropertyPair {
            key: "K".into(),
            value: "v2".into(),
        },
    ]);
    let err = org_mode.capture_append(entry).unwrap_err();
    assert!(matches!(err, OrgModeError::DuplicatePropertyKey(_)));
}

#[test]
fn test_capture_auto_created_default_on() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);
    let entry = capture_minimal("c.org", "Note");
    org_mode.capture_append(entry).unwrap();

    let content = fs::read_to_string(temp_dir.path().join("c.org")).unwrap();
    let drawer = content.find(":PROPERTIES:").expect("drawer present");
    let created = content.find(":CREATED:").expect("CREATED line present");
    assert!(drawer < created);
    let re =
        regex::Regex::new(r":CREATED: \[\d{4}-\d{2}-\d{2} [A-Z][a-z]{2} \d{2}:\d{2}\]").unwrap();
    assert!(re.is_match(&content), "CREATED format wrong:\n{content}");
}

#[test]
fn test_capture_auto_created_disabled_omits_drawer_when_no_user_properties() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = OrgMode::new(OrgConfig {
        org_directory: temp_dir.path().to_str().unwrap().to_string(),
        org_auto_created_property: false,
        ..OrgConfig::default()
    })
    .unwrap();

    let entry = capture_minimal("c.org", "Note");
    org_mode.capture_append(entry).unwrap();

    let content = fs::read_to_string(temp_dir.path().join("c.org")).unwrap();
    assert!(!content.contains(":PROPERTIES:"));
}

#[test]
fn test_capture_auto_created_user_wins() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);
    let mut entry = capture_minimal("c.org", "Note");
    entry.properties = Some(vec![PropertyPair {
        key: "CREATED".into(),
        value: "[2025-01-01 Wed]".into(),
    }]);
    org_mode.capture_append(entry).unwrap();

    let content = fs::read_to_string(temp_dir.path().join("c.org")).unwrap();
    assert!(content.contains(":CREATED: [2025-01-01 Wed]"));
    let count = content.matches(":CREATED:").count();
    assert_eq!(count, 1, "expected exactly 1 CREATED line, got {count}");
}

#[test]
fn test_capture_auto_created_case_insensitive_match() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);
    let mut entry = capture_minimal("c.org", "Note");
    entry.properties = Some(vec![PropertyPair {
        key: "created".into(),
        value: "manual".into(),
    }]);
    org_mode.capture_append(entry).unwrap();

    let content = fs::read_to_string(temp_dir.path().join("c.org")).unwrap();
    let count = content.to_lowercase().matches(":created:").count();
    assert_eq!(count, 1, "expected exactly 1 CREATED line, got {count}");
    assert!(content.contains(":created: manual"));
}

// Datetree integration tests
#[test]
fn test_datetree_creates_year_month_day_no_target() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = OrgMode::new(OrgConfig {
        org_directory: temp_dir.path().to_str().unwrap().to_string(),
        org_auto_created_property: false,
        ..OrgConfig::default()
    })
    .unwrap();

    let mut entry = capture_minimal("dt.org", "Item");
    entry.datetree = true;
    entry.datetree_date = Some("2026-05-10".to_string());
    let result = org_mode.capture_append(entry).unwrap();
    assert_eq!(result.level, 4);

    let content = fs::read_to_string(temp_dir.path().join("dt.org")).unwrap();
    assert!(content.contains("* 2026"));
    assert!(content.contains("** 2026-05 May"));
    assert!(content.contains("*** 2026-05-10 Sunday"));
    assert!(content.contains("**** Item"));
}

#[test]
fn test_datetree_under_target_heading() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = OrgMode::new(OrgConfig {
        org_directory: temp_dir.path().to_str().unwrap().to_string(),
        org_auto_created_property: false,
        ..OrgConfig::default()
    })
    .unwrap();
    fs::write(temp_dir.path().join("logs.org"), "* Logs\n").unwrap();

    let mut entry = capture_minimal("logs.org", "Standup");
    entry.target_heading = Some("Logs".to_string());
    entry.datetree = true;
    entry.datetree_date = Some("2026-05-10".to_string());
    let result = org_mode.capture_append(entry).unwrap();
    assert_eq!(result.level, 5);

    let content = fs::read_to_string(temp_dir.path().join("logs.org")).unwrap();
    assert!(content.contains("** 2026"));
    assert!(content.contains("*** 2026-05 May"));
    assert!(content.contains("**** 2026-05-10 Sunday"));
    assert!(content.contains("***** Standup"));
}

#[test]
fn test_datetree_reuses_existing_day() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = OrgMode::new(OrgConfig {
        org_directory: temp_dir.path().to_str().unwrap().to_string(),
        org_auto_created_property: false,
        ..OrgConfig::default()
    })
    .unwrap();

    let mut e1 = capture_minimal("dt.org", "First");
    e1.datetree = true;
    e1.datetree_date = Some("2026-05-10".to_string());
    org_mode.capture_append(e1).unwrap();

    let mut e2 = capture_minimal("dt.org", "Second");
    e2.datetree = true;
    e2.datetree_date = Some("2026-05-10".to_string());
    org_mode.capture_append(e2).unwrap();

    let content = fs::read_to_string(temp_dir.path().join("dt.org")).unwrap();
    assert_eq!(
        content.matches("* 2026\n").count(),
        1,
        "year heading must be unique:\n{content}"
    );
    assert_eq!(content.matches("** 2026-05 May").count(), 1);
    assert_eq!(content.matches("*** 2026-05-10 Sunday").count(), 1);
    assert!(content.contains("**** First"));
    assert!(content.contains("**** Second"));
}

#[test]
fn test_datetree_rejects_date_without_flag() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let mut entry = capture_minimal("dt.org", "x");
    entry.datetree = false;
    entry.datetree_date = Some("2026-05-10".to_string());
    let err = org_mode.capture_append(entry).unwrap_err();
    assert!(matches!(err, OrgModeError::DatetreeDateWithoutFlag));
}

#[test]
fn test_datetree_rejects_bad_date_format() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let mut entry = capture_minimal("dt.org", "x");
    entry.datetree = true;
    entry.datetree_date = Some("tomorrow".to_string());
    let err = org_mode.capture_append(entry).unwrap_err();
    assert!(matches!(err, OrgModeError::InvalidDatetreeDate(_)));
}

#[test]
fn test_datetree_rejects_date_with_time() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = make_org_mode(&temp_dir);

    let mut entry = capture_minimal("dt.org", "x");
    entry.datetree = true;
    entry.datetree_date = Some("2026-05-10 14:00".to_string());
    let err = org_mode.capture_append(entry).unwrap_err();
    assert!(matches!(err, OrgModeError::InvalidDatetreeDate(_)));
}

#[test]
fn test_capture_with_planning_and_properties() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = OrgMode::new(OrgConfig {
        org_directory: temp_dir.path().to_str().unwrap().to_string(),
        org_auto_created_property: false,
        ..OrgConfig::default()
    })
    .unwrap();

    let mut entry = capture_minimal("combo.org", "Combo");
    entry.scheduled = Some("2026-05-15".to_string());
    entry.deadline = Some("2026-05-20 17:00".to_string());
    entry.properties = Some(vec![PropertyPair {
        key: "CATEGORY".into(),
        value: "demo".into(),
    }]);
    org_mode.capture_append(entry).unwrap();

    let content = fs::read_to_string(temp_dir.path().join("combo.org")).unwrap();
    let heading = content.find("* Combo").expect("heading");
    let planning = content.find("SCHEDULED:").expect("planning");
    let drawer = content.find(":PROPERTIES:").expect("drawer");
    let category = content.find(":CATEGORY: demo").expect("category");
    let drawer_end = content.find(":END:").expect("drawer end");
    assert!(heading < planning, "planning must follow heading");
    assert!(planning < drawer, "drawer must follow planning");
    assert!(
        drawer < category && category < drawer_end,
        "drawer must contain CATEGORY"
    );
}

#[test]
fn test_capture_with_everything() {
    let temp_dir = tempfile::tempdir().unwrap();
    let org_mode = OrgMode::new(OrgConfig {
        org_directory: temp_dir.path().to_str().unwrap().to_string(),
        org_auto_created_property: false,
        ..OrgConfig::default()
    })
    .unwrap();

    let mut entry = capture_minimal("everything.org", "Full");
    entry.todo_state = Some("TODO".to_string());
    entry.priority = Some("A".to_string());
    entry.tags = Some(vec!["work".to_string()]);
    entry.scheduled = Some("2026-05-15".to_string());
    entry.deadline = Some("2026-05-20 17:00".to_string());
    entry.closed = Some("2026-05-10".to_string());
    entry.properties = Some(vec![
        PropertyPair {
            key: "CATEGORY".into(),
            value: "demo".into(),
        },
        PropertyPair {
            key: "EFFORT".into(),
            value: "1h".into(),
        },
    ]);
    entry.body = Some("Body text.".to_string());
    org_mode.capture_append(entry).unwrap();

    let content = fs::read_to_string(temp_dir.path().join("everything.org")).unwrap();

    let org = Org::parse(&content);
    let mut found = None;
    let mut handler = from_fn(|event| {
        if let Event::Enter(Container::Headline(ref h)) = event
            && found.is_none()
            && h.title_raw().trim() == "Full"
        {
            found = Some(h.clone());
        }
    });
    org.traverse(&mut handler);
    let h = found.expect("could not parse heading");

    assert!(h.scheduled().is_some(), "scheduled should round-trip");
    assert!(h.deadline().is_some(), "deadline should round-trip");

    let props = h.properties().expect("properties drawer should be present");
    let map = props.to_hash_map();
    assert_eq!(
        map.get("CATEGORY").map(|s| s.to_string()),
        Some("demo".to_string())
    );
    assert_eq!(
        map.get("EFFORT").map(|s| s.to_string()),
        Some("1h".to_string())
    );

    assert!(content.contains("CLOSED: [2026-05-10 Sun]"));
    assert!(content.contains("* TODO [#A] Full :work:"));
    assert!(content.contains("Body text."));
}

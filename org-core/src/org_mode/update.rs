use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::OrgModeError;
use crate::org_mode::capture::ParsedTimestamp;
use crate::org_mode::{ClearField, OrgMode, UpdateEntry};

static PLANNING_LINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(?:SCHEDULED|DEADLINE|CLOSED):").unwrap());
static PLANNING_KV_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(SCHEDULED|DEADLINE|CLOSED):\s*(\[[^\]]*\]|<[^>]*>)").unwrap());

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct PlanningValues {
    pub scheduled: Option<String>,
    pub deadline: Option<String>,
    pub closed: Option<String>,
}

#[derive(Debug)]
pub(crate) struct PlanningBlock {
    pub first_line: usize,
    pub line_count: usize,
    pub values: PlanningValues,
}

pub(crate) fn parse_planning_block(lines: &[String], headline_line: usize) -> PlanningBlock {
    let mut values = PlanningValues::default();
    let mut line_count = 0;
    for line in lines.iter().skip(headline_line + 1) {
        if !PLANNING_LINE_RE.is_match(line) {
            break;
        }
        line_count += 1;
        for cap in PLANNING_KV_RE.captures_iter(line) {
            let raw = cap[2].to_string();
            match &cap[1] {
                "SCHEDULED" => values.scheduled = Some(raw),
                "DEADLINE" => values.deadline = Some(raw),
                "CLOSED" => values.closed = Some(raw),
                _ => unreachable!("regex only matches the three keywords"),
            }
        }
    }
    PlanningBlock {
        first_line: headline_line + 1,
        line_count,
        values,
    }
}

pub(crate) fn render_planning_line(values: &PlanningValues) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(s) = &values.scheduled {
        parts.push(format!("SCHEDULED: {s}"));
    }
    if let Some(d) = &values.deadline {
        parts.push(format!("DEADLINE: {d}"));
    }
    if let Some(c) = &values.closed {
        parts.push(format!("CLOSED: {c}"));
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ResolvedUpdate {
    pub file_rel: Option<String>,
    pub scheduled: Option<ParsedTimestamp>,
    pub deadline: Option<ParsedTimestamp>,
    pub closed: Option<ParsedTimestamp>,
}

impl OrgMode {
    #[allow(dead_code)]
    pub(crate) fn validate_update(
        &self,
        entry: &UpdateEntry,
    ) -> Result<ResolvedUpdate, OrgModeError> {
        // Targeting: id OR (file + heading_path); id wins when both are given.
        if entry.file.is_some() != entry.heading_path.is_some() {
            return Err(OrgModeError::InvalidUpdate(
                "file and heading_path must be given together".to_string(),
            ));
        }
        if entry.id.is_none() && entry.file.is_none() {
            return Err(OrgModeError::InvalidUpdate(
                "target required: pass id or file + heading_path".to_string(),
            ));
        }
        if let Some(ref id) = entry.id
            && id.trim().is_empty()
        {
            return Err(OrgModeError::InvalidUpdate(
                "id must not be empty".to_string(),
            ));
        }
        if let Some(ref path) = entry.heading_path {
            for segment in path.split('/') {
                if segment.trim().is_empty() {
                    return Err(OrgModeError::InvalidHeadingPath(format!(
                        "heading_path contains an empty or whitespace-only segment: '{path}'"
                    )));
                }
            }
        }

        // At least one mutation.
        if entry.todo_state.is_none()
            && entry.priority.is_none()
            && entry.tags.is_none()
            && entry.scheduled.is_none()
            && entry.deadline.is_none()
            && entry.closed.is_none()
            && entry.clear.is_empty()
        {
            return Err(OrgModeError::InvalidUpdate("nothing to update".to_string()));
        }

        // No field may be both set and cleared; no duplicate clear entries.
        let mut seen: HashSet<ClearField> = HashSet::new();
        for field in &entry.clear {
            if !seen.insert(*field) {
                return Err(OrgModeError::InvalidUpdate(format!(
                    "duplicate clear entry '{field}'"
                )));
            }
            let conflict = match field {
                ClearField::TodoState => entry.todo_state.is_some(),
                ClearField::Priority => entry.priority.is_some(),
                ClearField::Tags => entry.tags.is_some(),
                ClearField::Scheduled => entry.scheduled.is_some(),
                ClearField::Deadline => entry.deadline.is_some(),
                ClearField::Closed => entry.closed.is_some(),
            };
            if conflict {
                return Err(OrgModeError::InvalidUpdate(format!(
                    "field '{field}' is both set and cleared"
                )));
            }
        }

        if let Some(ref kw) = entry.todo_state {
            let valid_keywords: Vec<&str> = self
                .config
                .org_todo_keywords
                .iter()
                .filter(|k| k.as_str() != "|")
                .map(|k| k.as_str())
                .collect();
            if !valid_keywords.contains(&kw.as_str()) {
                return Err(OrgModeError::InvalidTodoKeyword(kw.clone()));
            }
        }

        if let Some(ref p) = entry.priority
            && !matches!(p.as_str(), "A" | "B" | "C")
        {
            return Err(OrgModeError::InvalidPriority(p.clone()));
        }

        if let Some(ref tags) = entry.tags {
            for tag in tags {
                if !super::capture::is_valid_tag(tag) {
                    return Err(OrgModeError::InvalidTag(tag.clone()));
                }
            }
        }

        let scheduled = entry
            .scheduled
            .as_deref()
            .map(|v| Self::parse_iso_timestamp("scheduled", v))
            .transpose()?;
        let deadline = entry
            .deadline
            .as_deref()
            .map(|v| Self::parse_iso_timestamp("deadline", v))
            .transpose()?;
        let closed = entry
            .closed
            .as_deref()
            .map(|v| Self::parse_iso_timestamp("closed", v))
            .transpose()?;

        let file_rel = match entry.file {
            Some(ref f) => {
                Self::validate_relative_file_path(f)?;
                Some(f.clone())
            }
            None => None,
        };

        Ok(ResolvedUpdate {
            file_rel,
            scheduled,
            deadline,
            closed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OrgConfig;

    fn make_org_mode(temp_dir: &tempfile::TempDir) -> OrgMode {
        OrgMode::new(OrgConfig {
            org_directory: temp_dir.path().to_str().unwrap().to_string(),
            ..OrgConfig::default()
        })
        .unwrap()
    }

    fn entry_by_path() -> UpdateEntry {
        UpdateEntry {
            id: None,
            file: Some("notes.org".to_string()),
            heading_path: Some("Daily Tasks/Buy groceries".to_string()),
            todo_state: Some("DONE".to_string()),
            priority: None,
            tags: None,
            scheduled: None,
            deadline: None,
            closed: None,
            clear: vec![],
        }
    }

    #[test]
    fn test_validate_rejects_missing_target() {
        let temp_dir = tempfile::tempdir().unwrap();
        let org_mode = make_org_mode(&temp_dir);
        let mut e = entry_by_path();
        e.file = None;
        e.heading_path = None;
        let err = org_mode.validate_update(&e).unwrap_err();
        assert!(matches!(err, OrgModeError::InvalidUpdate(_)));
    }

    #[test]
    fn test_validate_rejects_file_without_heading_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let org_mode = make_org_mode(&temp_dir);
        let mut e = entry_by_path();
        e.heading_path = None;
        let err = org_mode.validate_update(&e).unwrap_err();
        assert!(matches!(err, OrgModeError::InvalidUpdate(_)));
    }

    #[test]
    fn test_validate_rejects_noop() {
        let temp_dir = tempfile::tempdir().unwrap();
        let org_mode = make_org_mode(&temp_dir);
        let mut e = entry_by_path();
        e.todo_state = None;
        let err = org_mode.validate_update(&e).unwrap_err();
        match err {
            OrgModeError::InvalidUpdate(msg) => assert_eq!(msg, "nothing to update"),
            other => panic!("expected InvalidUpdate, got {other:?}"),
        }
    }

    #[test]
    fn test_validate_rejects_set_and_clear_same_field() {
        let temp_dir = tempfile::tempdir().unwrap();
        let org_mode = make_org_mode(&temp_dir);
        let mut e = entry_by_path();
        e.todo_state = None;
        e.scheduled = Some("2026-05-15".to_string());
        e.clear = vec![ClearField::Scheduled];
        let err = org_mode.validate_update(&e).unwrap_err();
        match err {
            OrgModeError::InvalidUpdate(msg) => assert!(msg.contains("scheduled")),
            other => panic!("expected InvalidUpdate, got {other:?}"),
        }
    }

    #[test]
    fn test_validate_rejects_invalid_keyword_priority_tag() {
        let temp_dir = tempfile::tempdir().unwrap();
        let org_mode = make_org_mode(&temp_dir);

        let mut e = entry_by_path();
        e.todo_state = Some("MAYBE".to_string());
        assert!(matches!(
            org_mode.validate_update(&e).unwrap_err(),
            OrgModeError::InvalidTodoKeyword(_)
        ));

        let mut e = entry_by_path();
        e.todo_state = None;
        e.priority = Some("Z".to_string());
        assert!(matches!(
            org_mode.validate_update(&e).unwrap_err(),
            OrgModeError::InvalidPriority(_)
        ));

        let mut e = entry_by_path();
        e.todo_state = None;
        e.tags = Some(vec!["bad tag".to_string()]);
        assert!(matches!(
            org_mode.validate_update(&e).unwrap_err(),
            OrgModeError::InvalidTag(_)
        ));
    }

    #[test]
    fn test_validate_rejects_invalid_timestamp_and_traversal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let org_mode = make_org_mode(&temp_dir);

        let mut e = entry_by_path();
        e.todo_state = None;
        e.scheduled = Some("tomorrow".to_string());
        assert!(matches!(
            org_mode.validate_update(&e).unwrap_err(),
            OrgModeError::InvalidTimestamp { .. }
        ));

        let mut e = entry_by_path();
        e.file = Some("../escape.org".to_string());
        assert!(matches!(
            org_mode.validate_update(&e).unwrap_err(),
            OrgModeError::InvalidDirectory(_)
        ));
    }

    #[test]
    fn test_validate_accepts_clear_only_update() {
        let temp_dir = tempfile::tempdir().unwrap();
        let org_mode = make_org_mode(&temp_dir);
        let mut e = entry_by_path();
        e.todo_state = None;
        e.clear = vec![ClearField::Priority];
        assert!(org_mode.validate_update(&e).is_ok());
    }

    fn lines_of(content: &str) -> Vec<String> {
        content.lines().map(String::from).collect()
    }

    #[test]
    fn test_parse_planning_single_line() {
        let lines = lines_of(
            "* TODO Task\nSCHEDULED: <2026-05-15 Fri ++1w> DEADLINE: \
            <2026-05-20 Wed>\nbody\n",
        );
        let block = parse_planning_block(&lines, 0);
        assert_eq!(block.first_line, 1);
        assert_eq!(block.line_count, 1);
        assert_eq!(
            block.values.scheduled.as_deref(),
            Some("<2026-05-15 Fri ++1w>")
        );
        assert_eq!(block.values.deadline.as_deref(), Some("<2026-05-20 Wed>"));
        assert_eq!(block.values.closed, None);
    }

    #[test]
    fn test_parse_planning_multi_line_and_closed() {
        let lines = lines_of(
            "* DONE Task\nCLOSED: [2026-05-16 Sat 10:00]\nSCHEDULED: <2026-05-15 Fri>\nbody\n",
        );
        let block = parse_planning_block(&lines, 0);
        assert_eq!(block.line_count, 2);
        assert_eq!(
            block.values.closed.as_deref(),
            Some("[2026-05-16 Sat 10:00]")
        );
        assert_eq!(block.values.scheduled.as_deref(), Some("<2026-05-15 Fri>"));
    }

    #[test]
    fn test_parse_planning_absent() {
        let lines = lines_of("* TODO Task\n:PROPERTIES:\n:END:\n");
        let block = parse_planning_block(&lines, 0);
        assert_eq!(block.line_count, 0);
        assert_eq!(block.values, PlanningValues::default());
    }

    #[test]
    fn test_parse_planning_stops_at_body() {
        let lines = lines_of(
            "* TODO Task\nSCHEDULED: <2026-05-15 Fri>\nA note\nDEADLINE: \
            <2026-05-20 Wed>\n",
        );
        let block = parse_planning_block(&lines, 0);
        assert_eq!(block.line_count, 1);
        assert_eq!(block.values.deadline, None);
    }

    #[test]
    fn test_render_planning_line_order_and_none() {
        let values = PlanningValues {
            scheduled: Some("<2026-05-15 Fri>".to_string()),
            deadline: None,
            closed: Some("[2026-05-16 Sat]".to_string()),
        };
        assert_eq!(
            render_planning_line(&values).as_deref(),
            Some("SCHEDULED: <2026-05-15 Fri> CLOSED: [2026-05-16 Sat]")
        );
        assert_eq!(render_planning_line(&PlanningValues::default()), None);
    }
}

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use orgize::export::{Container, Event, from_fn, from_fn_with_ctx};
use orgize::{Org, ParseConfig};

use crate::OrgModeError;
use crate::org_mode::capture::ParsedTimestamp;
use crate::org_mode::{ClearField, OrgMode, UpdateEntry, UpdateResult};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct PlanningValues {
    pub scheduled: Option<String>,
    pub deadline: Option<String>,
    pub closed: Option<String>,
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
pub(crate) struct ResolvedUpdate {
    pub scheduled: Option<ParsedTimestamp>,
    pub deadline: Option<ParsedTimestamp>,
    pub closed: Option<ParsedTimestamp>,
}

#[derive(Debug)]
struct TargetHeadline {
    line_idx: usize,
    level: usize,
    title: String,
    keyword: Option<String>,
    priority: Option<String>,
    tags: Vec<String>,
    ambiguity: Option<String>,
    planning_first_line: usize,
    planning_line_count: usize,
    planning_values: PlanningValues,
}

fn line_index_at(content: &str, byte_offset: usize) -> usize {
    content[..byte_offset]
        .bytes()
        .filter(|b| *b == b'\n')
        .count()
}

fn extract_planning(h: &orgize::ast::Headline, content: &str) -> (usize, usize, PlanningValues) {
    match h.planning() {
        None => (
            line_index_at(content, h.start().into()) + 1,
            0,
            PlanningValues::default(),
        ),
        Some(p) => {
            let p_start: usize = p.start().into();
            let p_end: usize = p.end().into();
            let first_line = line_index_at(content, p_start);
            // orgize's planning span ends before the trailing '\n' only at EOF; max(1) guards that.
            let ast_line_count = content[p_start..p_end]
                .bytes()
                .filter(|&b| b == b'\n')
                .count()
                .max(1);
            let mut values = PlanningValues {
                scheduled: p.scheduled().map(|ts| ts.raw()),
                deadline: p.deadline().map(|ts| ts.raw()),
                closed: p.closed().map(|ts| ts.raw()),
            };
            // Extend line_count over any additional consecutive planning lines that
            // orgize does not include in its span (non-standard but written by some tools).
            // Also extract any timestamps from those extra lines so they survive the splice.
            let all_lines: Vec<&str> = content.lines().collect();
            let mut extra = 0;
            let mut next = first_line + ast_line_count;
            while let Some(line) = all_lines.get(next) {
                let t = line.trim_start();
                let keyword = if t.starts_with("SCHEDULED:") {
                    Some("SCHEDULED")
                } else if t.starts_with("DEADLINE:") {
                    Some("DEADLINE")
                } else if t.starts_with("CLOSED:") {
                    Some("CLOSED")
                } else {
                    None
                };
                if let Some(kw) = keyword {
                    // Extract the raw timestamp (everything after "KEYWORD: ").
                    let raw = t[kw.len()..].trim_start_matches(':').trim().to_string();
                    match kw {
                        "SCHEDULED" if values.scheduled.is_none() => {
                            values.scheduled = Some(raw);
                        }
                        "DEADLINE" if values.deadline.is_none() => {
                            values.deadline = Some(raw);
                        }
                        "CLOSED" if values.closed.is_none() => {
                            values.closed = Some(raw);
                        }
                        _ => {}
                    }
                    extra += 1;
                    next += 1;
                } else {
                    break;
                }
            }
            (first_line, ast_line_count + extra, values)
        }
    }
}

impl OrgMode {
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
                ClearField::Body => entry.body.is_some(),
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

        if let Some(ref f) = entry.file {
            Self::validate_relative_file_path(f)?;
        }

        Ok(ResolvedUpdate {
            scheduled,
            deadline,
            closed,
        })
    }

    pub fn update_todo(&self, entry: UpdateEntry) -> Result<UpdateResult, OrgModeError> {
        let resolved = self.validate_update(&entry)?;
        let (file_rel, full_path) = self.resolve_target_file(&entry)?;

        let lock_path = Self::lock_path_for(&full_path)?;
        let lock_file = Self::acquire_capture_lock(&lock_path)?;

        let result = self.apply_update(&file_rel, &full_path, &entry, &resolved);

        // Same lock-release dance as capture_append.
        #[cfg(unix)]
        let _ = fs::remove_file(&lock_path);
        drop(lock_file);
        #[cfg(not(unix))]
        let _ = fs::remove_file(&lock_path);

        result
    }

    fn resolve_target_file(&self, entry: &UpdateEntry) -> Result<(String, PathBuf), OrgModeError> {
        if let Some(ref id) = entry.id {
            let mut matches: Vec<String> = Vec::new();
            for path in self.list_files(None, None)? {
                let content = self.read_file(&path)?;
                if Self::file_contains_id(&content, id) {
                    matches.push(path);
                }
            }
            return match matches.len() {
                0 => Err(OrgModeError::HeadingNotFound(id.clone())),
                1 => {
                    let file_rel = matches.pop().unwrap();
                    let full_path = self.prepare_target_path(&file_rel)?;
                    Ok((file_rel, full_path))
                }
                _ => Err(OrgModeError::AmbiguousTarget(id.clone())),
            };
        }

        let file_rel = entry.file.clone().unwrap();
        let full_path = PathBuf::from(&self.config.org_directory).join(&file_rel);
        if !full_path.is_file() {
            return Err(OrgModeError::HeadingNotFound(format!(
                "{file_rel} (file does not exist)"
            )));
        }
        let full_path = self.prepare_target_path(&file_rel)?;
        Ok((file_rel, full_path))
    }

    fn file_contains_id(content: &str, id: &str) -> bool {
        let mut found = false;
        let mut handler = from_fn_with_ctx(|event, ctx| {
            if let Event::Enter(Container::Headline(ref h)) = event
                && h.properties()
                    .and_then(|props| {
                        props
                            .to_hash_map()
                            .into_iter()
                            .find(|(k, v)| k.to_lowercase() == "id" && v == id)
                    })
                    .is_some()
            {
                found = true;
                ctx.stop();
            }
        });
        Org::parse(content).traverse(&mut handler);
        found
    }

    fn apply_update(
        &self,
        file_rel: &str,
        full_path: &PathBuf,
        entry: &UpdateEntry,
        resolved: &ResolvedUpdate,
    ) -> Result<UpdateResult, OrgModeError> {
        let content = fs::read_to_string(full_path).map_err(OrgModeError::IoError)?;

        let parse_config = ParseConfig {
            todo_keywords: (
                self.config.unfinished_keywords(),
                self.config.finished_keywords(),
            ),
            ..Default::default()
        };
        let org = parse_config.parse(&content);

        let target = match self.locate_headline(&org, &content, entry)? {
            Some(t) if t.ambiguity.is_none() => t,
            Some(t) => return Err(OrgModeError::AmbiguousTarget(t.ambiguity.unwrap())),
            None => {
                let shown = entry
                    .id
                    .clone()
                    .unwrap_or_else(|| entry.heading_path.clone().unwrap_or_default());
                return Err(OrgModeError::HeadingNotFound(shown));
            }
        };

        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        // Resulting field values: explicit set > clear > existing.
        let cleared = |f: ClearField| entry.clear.contains(&f);

        let new_keyword = if cleared(ClearField::TodoState) {
            None
        } else {
            entry.todo_state.clone().or(target.keyword.clone())
        };
        let new_priority = if cleared(ClearField::Priority) {
            None
        } else {
            entry.priority.clone().or(target.priority.clone())
        };
        let new_tags = if cleared(ClearField::Tags) {
            Vec::new()
        } else {
            entry.tags.clone().unwrap_or_else(|| target.tags.clone())
        };
        let new_scheduled = if cleared(ClearField::Scheduled) {
            None
        } else {
            resolved
                .scheduled
                .as_ref()
                .map(|ts| Self::format_org_timestamp(ts, true))
                .or(target.planning_values.scheduled.clone())
        };
        let new_deadline = if cleared(ClearField::Deadline) {
            None
        } else {
            resolved
                .deadline
                .as_ref()
                .map(|ts| Self::format_org_timestamp(ts, true))
                .or(target.planning_values.deadline.clone())
        };

        let is_done = new_keyword
            .as_ref()
            .map(|k| self.config.finished_keywords().contains(k))
            .unwrap_or(false);
        let new_closed = if cleared(ClearField::Closed) {
            None
        } else if let Some(ts) = &resolved.closed {
            Some(Self::format_org_timestamp(ts, false))
        } else if is_done {
            match target.planning_values.closed.clone() {
                Some(existing) => Some(existing),
                None if self.config.org_auto_closed_timestamp => {
                    let now = chrono::Local::now();
                    Some(Self::format_org_timestamp(
                        &ParsedTimestamp {
                            date: now.date_naive(),
                            time: Some(now.time()),
                            repeater: None,
                            warning: None,
                        },
                        false,
                    ))
                }
                None => None,
            }
        } else if self.config.org_auto_closed_timestamp {
            // Reactivated or keyword-less heading: drop the stale CLOSED.
            None
        } else {
            target.planning_values.closed.clone()
        };

        let new_headline = Self::format_heading(
            target.level,
            new_keyword.as_deref(),
            new_priority.as_deref(),
            &target.title,
            Some(&new_tags),
        );
        let new_planning = render_planning_line(&PlanningValues {
            scheduled: new_scheduled.clone(),
            deadline: new_deadline.clone(),
            closed: new_closed.clone(),
        });

        // Splice: headline line replaced; planning block (0..n lines) replaced by 0..1 lines.
        lines[target.line_idx] = new_headline.clone();
        let replacement: Vec<String> = new_planning.into_iter().collect();
        lines.splice(
            target.planning_first_line..target.planning_first_line + target.planning_line_count,
            replacement,
        );

        let newline = if content.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let mut out = lines.join(newline);
        if content.ends_with('\n') {
            out.push_str(newline);
        }
        Self::atomic_write(full_path, out.as_bytes())?;

        let mut changes = Vec::new();
        Self::push_change(
            &mut changes,
            "todo_state",
            target.keyword.as_deref(),
            new_keyword.as_deref(),
        );
        Self::push_change(
            &mut changes,
            "priority",
            target.priority.as_deref(),
            new_priority.as_deref(),
        );
        let old_tags = target.tags.join(":");
        let new_tags_joined = new_tags.join(":");
        Self::push_change(
            &mut changes,
            "tags",
            (!old_tags.is_empty()).then_some(old_tags.as_str()),
            (!new_tags_joined.is_empty()).then_some(new_tags_joined.as_str()),
        );
        Self::push_change(
            &mut changes,
            "scheduled",
            target.planning_values.scheduled.as_deref(),
            new_scheduled.as_deref(),
        );
        Self::push_change(
            &mut changes,
            "deadline",
            target.planning_values.deadline.as_deref(),
            new_deadline.as_deref(),
        );
        Self::push_change(
            &mut changes,
            "closed",
            target.planning_values.closed.as_deref(),
            new_closed.as_deref(),
        );

        Ok(UpdateResult {
            file_path: file_rel.to_string(),
            heading_line: new_headline,
            changes,
        })
    }

    fn push_change(changes: &mut Vec<String>, name: &str, old: Option<&str>, new: Option<&str>) {
        if old != new {
            changes.push(format!(
                "{name}: {} -> {}",
                old.unwrap_or("<none>"),
                new.unwrap_or("<none>")
            ));
        }
    }

    fn locate_headline(
        &self,
        org: &Org,
        content: &str,
        entry: &UpdateEntry,
    ) -> Result<Option<TargetHeadline>, OrgModeError> {
        let mut matches: Vec<TargetHeadline> = Vec::new();

        if let Some(ref id) = entry.id {
            let mut handler = from_fn_with_ctx(|event, ctx| {
                if let Event::Enter(Container::Headline(ref h)) = event {
                    let has_id = h
                        .properties()
                        .and_then(|props| {
                            props
                                .to_hash_map()
                                .into_iter()
                                .find(|(k, v)| k.to_lowercase() == "id" && v == id)
                        })
                        .is_some();
                    if has_id {
                        let (planning_first_line, planning_line_count, planning_values) =
                            extract_planning(h, content);
                        matches.push(TargetHeadline {
                            line_idx: line_index_at(content, h.start().into()),
                            level: h.level(),
                            title: h.title_raw().trim_end().to_string(),
                            keyword: h.todo_keyword().map(|t| t.to_string()),
                            priority: h.priority().map(|p| p.to_string()),
                            tags: h.tags().map(|s| s.to_string()).collect(),
                            ambiguity: None,
                            planning_first_line,
                            planning_line_count,
                            planning_values,
                        });
                        // Two matches suffice to report ambiguity; stop early.
                        if matches.len() == 2 {
                            ctx.stop();
                        }
                    }
                }
            });
            org.traverse(&mut handler);
        } else {
            let path = entry.heading_path.clone().unwrap_or_default();
            let parts: Vec<&str> = path.split('/').collect();
            let mut stack: Vec<(usize, String)> = Vec::new();
            let mut handler = from_fn(|event| {
                if let Event::Enter(Container::Headline(ref h)) = event {
                    let level = h.level();
                    while stack.last().map(|(l, _)| *l >= level).unwrap_or(false) {
                        stack.pop();
                    }
                    // orgize's title_raw() keeps the title/tags separator as a
                    // trailing space once tags are present; trim so user-supplied
                    // path segments match the bare title.
                    stack.push((level, h.title_raw().trim_end().to_string()));
                    if stack.len() == parts.len()
                        && stack
                            .iter()
                            .map(|(_, t)| t.as_str())
                            .eq(parts.iter().copied())
                    {
                        let (planning_first_line, planning_line_count, planning_values) =
                            extract_planning(h, content);
                        matches.push(TargetHeadline {
                            line_idx: line_index_at(content, h.start().into()),
                            level,
                            title: h.title_raw().trim_end().to_string(),
                            keyword: h.todo_keyword().map(|t| t.to_string()),
                            priority: h.priority().map(|p| p.to_string()),
                            tags: h.tags().map(|s| s.to_string()).collect(),
                            ambiguity: None,
                            planning_first_line,
                            planning_line_count,
                            planning_values,
                        });
                    }
                }
            });
            org.traverse(&mut handler);
        }

        match matches.len() {
            0 => Ok(None),
            1 => Ok(matches.pop().map(Some).unwrap()),
            _ => {
                let shown = entry
                    .id
                    .clone()
                    .unwrap_or_else(|| entry.heading_path.clone().unwrap_or_default());
                Ok(Some(TargetHeadline {
                    line_idx: 0,
                    level: 0,
                    title: String::new(),
                    keyword: None,
                    priority: None,
                    tags: vec![],
                    ambiguity: Some(shown),
                    planning_first_line: 0,
                    planning_line_count: 0,
                    planning_values: PlanningValues::default(),
                }))
            }
        }
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
            title: None,
            body: None,
            properties: None,
            remove_properties: None,
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
    fn test_validate_rejects_empty_heading_path_segment() {
        let temp_dir = tempfile::tempdir().unwrap();
        let org_mode = make_org_mode(&temp_dir);
        let mut e = entry_by_path();
        e.heading_path = Some("Daily Tasks//Buy groceries".to_string());
        assert!(matches!(
            org_mode.validate_update(&e).unwrap_err(),
            OrgModeError::InvalidHeadingPath(_)
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

    use crate::org_mode::UpdateResult;
    use std::fs;

    const FIXTURE: &str = "\
* Daily Tasks
:PROPERTIES:
:ID: daily-tasks-123
:END:
** TODO Buy groceries
:PROPERTIES:
:ID: task-groceries-456
:END:
** DONE Read book
CLOSED: [2026-05-01 Fri 09:00]
:PROPERTIES:
:ID: task-book-789
:END:
* Projects
** Work
*** TODO Refactor API
SCHEDULED: <2026-05-15 Fri ++1w>
Body line must survive.
";

    fn setup_fixture(temp_dir: &tempfile::TempDir) {
        fs::write(temp_dir.path().join("notes.org"), FIXTURE).unwrap();
    }

    fn update_by_id(id: &str) -> UpdateEntry {
        UpdateEntry {
            id: Some(id.to_string()),
            file: None,
            heading_path: None,
            todo_state: None,
            priority: None,
            tags: None,
            scheduled: None,
            deadline: None,
            closed: None,
            clear: vec![],
            title: None,
            body: None,
            properties: None,
            remove_properties: None,
        }
    }

    #[test]
    fn test_update_state_to_done_stamps_closed_by_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        setup_fixture(&temp_dir);
        let org_mode = make_org_mode(&temp_dir);

        let mut e = update_by_id("task-groceries-456");
        e.todo_state = Some("DONE".to_string());
        let result: UpdateResult = org_mode.update_todo(e).unwrap();

        assert_eq!(result.file_path, "notes.org");
        assert_eq!(result.heading_line, "** DONE Buy groceries");
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let content = fs::read_to_string(temp_dir.path().join("notes.org")).unwrap();
        assert!(content.contains("** DONE Buy groceries"));
        assert!(
            content.contains(&format!("CLOSED: [{today}")),
            "expected auto CLOSED stamp with today's date:\n{content}"
        );
        // property drawer must survive directly under the planning line
        assert!(content.contains(":ID: task-groceries-456"));
    }

    #[test]
    fn test_update_done_to_todo_removes_closed_by_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        setup_fixture(&temp_dir);
        let org_mode = make_org_mode(&temp_dir);

        let mut e = UpdateEntry {
            id: None,
            file: Some("notes.org".to_string()),
            heading_path: Some("Daily Tasks/Read book".to_string()),
            todo_state: Some("TODO".to_string()),
            priority: None,
            tags: None,
            scheduled: None,
            deadline: None,
            closed: None,
            clear: vec![],
            title: None,
            body: None,
            properties: None,
            remove_properties: None,
        };
        let result = org_mode.update_todo(e.clone()).unwrap();
        assert_eq!(result.heading_line, "** TODO Read book");
        let content = fs::read_to_string(temp_dir.path().join("notes.org")).unwrap();
        assert!(
            !content.contains("CLOSED:"),
            "CLOSED must be removed:\n{content}"
        );
        assert!(content.contains(":ID: task-book-789"));

        // id wins when both targeting forms are given
        e.id = Some("task-groceries-456".to_string());
        e.todo_state = Some("DONE".to_string());
        let result = org_mode.update_todo(e).unwrap();
        assert_eq!(result.heading_line, "** DONE Buy groceries");
    }

    #[test]
    fn test_update_explicit_closed_wins_over_auto_stamp() {
        let temp_dir = tempfile::tempdir().unwrap();
        setup_fixture(&temp_dir);
        let org_mode = make_org_mode(&temp_dir);

        let mut e = update_by_id("task-groceries-456");
        e.todo_state = Some("DONE".to_string());
        e.closed = Some("2026-05-17 12:00".to_string());
        org_mode.update_todo(e).unwrap();

        let content = fs::read_to_string(temp_dir.path().join("notes.org")).unwrap();
        assert!(content.contains("CLOSED: [2026-05-17 Sun 12:00]"));
    }

    #[test]
    fn test_update_metadata_preserves_body_and_repeater() {
        let temp_dir = tempfile::tempdir().unwrap();
        setup_fixture(&temp_dir);
        let org_mode = make_org_mode(&temp_dir);

        let mut e = UpdateEntry {
            id: None,
            file: Some("notes.org".to_string()),
            heading_path: Some("Projects/Work/Refactor API".to_string()),
            todo_state: None,
            priority: Some("A".to_string()),
            tags: Some(vec!["backend".to_string()]),
            scheduled: None,
            deadline: Some("2026-05-20 17:00".to_string()),
            closed: None,
            clear: vec![],
            title: None,
            body: None,
            properties: None,
            remove_properties: None,
        };
        let result = org_mode.update_todo(e.clone()).unwrap();
        assert_eq!(result.heading_line, "*** TODO [#A] Refactor API :backend:");
        let content = fs::read_to_string(temp_dir.path().join("notes.org")).unwrap();
        // untouched scheduled keeps its raw text including the repeater
        assert!(content.contains("SCHEDULED: <2026-05-15 Fri ++1w>"));
        assert!(content.contains("DEADLINE: <2026-05-20 Wed 17:00>"));
        assert!(content.contains("Body line must survive."));

        // everything outside the headline+planning span is byte-identical
        e.priority = Some("B".to_string());
        e.tags = None;
        e.deadline = None;
        org_mode.update_todo(e).unwrap();
        let after = fs::read_to_string(temp_dir.path().join("notes.org")).unwrap();
        assert!(after.contains("*** TODO [#B] Refactor API :backend:"));
        assert!(after.ends_with("Body line must survive.\n"));
        assert!(after.starts_with("* Daily Tasks\n:PROPERTIES:\n:ID: daily-tasks-123"));
    }

    #[test]
    fn test_update_target_errors() {
        let temp_dir = tempfile::tempdir().unwrap();
        setup_fixture(&temp_dir);
        let org_mode = make_org_mode(&temp_dir);

        let mut e = update_by_id("no-such-id");
        e.todo_state = Some("DONE".to_string());
        assert!(matches!(
            org_mode.update_todo(e.clone()).unwrap_err(),
            OrgModeError::HeadingNotFound(_)
        ));

        e.id = None;
        e.file = Some("notes.org".to_string());
        e.heading_path = Some("No/Such/Path".to_string());
        assert!(matches!(
            org_mode.update_todo(e.clone()).unwrap_err(),
            OrgModeError::HeadingNotFound(_)
        ));

        e.heading_path = Some("Daily Tasks".to_string());
        // Non-leaf paths are valid targets: "Daily Tasks" has children but is a
        // unique full-path match. Guards the behavior flipped by PR review.
        let result = org_mode.update_todo(e).unwrap();
        assert_eq!(result.heading_line, "* DONE Daily Tasks");
        let content = fs::read_to_string(temp_dir.path().join("notes.org")).unwrap();
        assert!(content.contains("* DONE Daily Tasks"));
    }

    #[test]
    fn test_update_preserves_crlf_line_endings() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(
            temp_dir.path().join("crlf.org"),
            "* TODO Task\r\nSCHEDULED: <2026-05-15 Fri>\r\nbody\r\n",
        )
        .unwrap();
        let org_mode = make_org_mode(&temp_dir);

        let mut e = update_by_id("irrelevant");
        e.id = None;
        e.file = Some("crlf.org".to_string());
        e.heading_path = Some("Task".to_string());
        e.todo_state = Some("DONE".to_string());
        org_mode.update_todo(e).unwrap();

        let content = fs::read_to_string(temp_dir.path().join("crlf.org")).unwrap();
        assert!(content.contains("* DONE Task\r\n"));
        assert!(content.contains("\r\nbody\r\n"));
        assert!(content.contains("CLOSED: ["));
        assert!(
            !content.replace("\r\n", "").contains('\n'),
            "found bare LF after CRLF update:\n{content}"
        );
    }

    #[test]
    fn test_update_ambiguous_path_rejected() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(
            temp_dir.path().join("dup.org"),
            "* A\n** TODO Dup\n* B\n** TODO Dup\n",
        )
        .unwrap();
        let org_mode = make_org_mode(&temp_dir);

        let mut e = update_by_id("irrelevant");
        e.id = None;
        e.file = Some("dup.org".to_string());
        e.heading_path = Some("B/Dup".to_string());
        e.todo_state = Some("DONE".to_string());
        // unique under B: works
        org_mode.update_todo(e.clone()).unwrap();
        // now make the remaining one ambiguous by checking a duplicated full path
        fs::write(
            temp_dir.path().join("dup2.org"),
            "* X\n** TODO Same\n* X\n** TODO Same\n",
        )
        .unwrap();
        e.file = Some("dup2.org".to_string());
        e.heading_path = Some("X/Same".to_string());
        assert!(matches!(
            org_mode.update_todo(e).unwrap_err(),
            OrgModeError::AmbiguousTarget(_)
        ));
    }

    #[test]
    fn test_update_by_id_preserves_single_spacing_with_tags() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(
            temp_dir.path().join("ids.org"),
            "* TODO Tagged task :a:b:\n:PROPERTIES:\n:ID: tagged-1\n:END:\nbody\n",
        )
        .unwrap();
        let org_mode = make_org_mode(&temp_dir);

        let mut e = update_by_id("tagged-1");
        e.priority = Some("A".to_string());
        let result = org_mode.update_todo(e).unwrap();
        assert_eq!(result.heading_line, "* TODO [#A] Tagged task :a:b:");
        let content = fs::read_to_string(temp_dir.path().join("ids.org")).unwrap();
        assert!(content.contains("* TODO [#A] Tagged task :a:b:\n"));
        assert!(content.contains("\nbody\n"));
    }

    #[test]
    fn test_clear_scheduled_and_priority() {
        let temp_dir = tempfile::tempdir().unwrap();
        setup_fixture(&temp_dir);
        let org_mode = make_org_mode(&temp_dir);

        let e = UpdateEntry {
            id: None,
            file: Some("notes.org".to_string()),
            heading_path: Some("Projects/Work/Refactor API".to_string()),
            todo_state: None,
            priority: None,
            tags: None,
            scheduled: None,
            deadline: None,
            closed: None,
            clear: vec![ClearField::Scheduled, ClearField::Priority],
            title: None,
            body: None,
            properties: None,
            remove_properties: None,
        };
        let result = org_mode.update_todo(e).unwrap();
        assert_eq!(result.heading_line, "*** TODO Refactor API");
        let content = fs::read_to_string(temp_dir.path().join("notes.org")).unwrap();
        assert!(
            !content.contains("SCHEDULED:"),
            "planning line removed:\n{content}"
        );
        assert!(content.contains("Body line must survive."));
        assert!(
            result
                .changes
                .iter()
                .any(|c| c.starts_with("scheduled: ") && c.ends_with("-> <none>")),
            "expected a scheduled removal entry, got {:?}",
            result.changes
        );
    }

    #[test]
    fn test_clear_todo_state_strips_keyword_and_closed() {
        let temp_dir = tempfile::tempdir().unwrap();
        setup_fixture(&temp_dir);
        let org_mode = make_org_mode(&temp_dir);

        let mut e = update_by_id("task-book-789");
        e.clear = vec![ClearField::TodoState];
        let result = org_mode.update_todo(e).unwrap();
        assert_eq!(result.heading_line, "** Read book");
        let content = fs::read_to_string(temp_dir.path().join("notes.org")).unwrap();
        assert!(
            !content.contains("CLOSED:"),
            "CLOSED removed with keyword:\n{content}"
        );
    }

    #[test]
    fn test_clear_tags_only() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(
            temp_dir.path().join("tags.org"),
            "* TODO Tagged task :a:b:\nbody\n",
        )
        .unwrap();
        let org_mode = make_org_mode(&temp_dir);

        let e = UpdateEntry {
            id: None,
            file: Some("tags.org".to_string()),
            heading_path: Some("Tagged task".to_string()),
            todo_state: None,
            priority: None,
            tags: None,
            scheduled: None,
            deadline: None,
            closed: None,
            clear: vec![ClearField::Tags],
            title: None,
            body: None,
            properties: None,
            remove_properties: None,
        };
        let result = org_mode.update_todo(e).unwrap();
        assert_eq!(result.heading_line, "* TODO Tagged task");
        let content = fs::read_to_string(temp_dir.path().join("tags.org")).unwrap();
        assert_eq!(content, "* TODO Tagged task\nbody\n");
    }

    #[test]
    fn test_planning_lands_before_property_drawer() {
        let temp_dir = tempfile::tempdir().unwrap();
        setup_fixture(&temp_dir);
        let org_mode = make_org_mode(&temp_dir);

        let mut e = update_by_id("task-groceries-456");
        e.scheduled = Some("2026-05-18".to_string());
        org_mode.update_todo(e).unwrap();

        let content = fs::read_to_string(temp_dir.path().join("notes.org")).unwrap();
        let block_start = content.find("** TODO Buy groceries").unwrap();
        let rest = &content[block_start..];
        let planning_pos = rest.find("SCHEDULED: <2026-05-18 Mon>").unwrap();
        let drawer_pos = rest.find(":PROPERTIES:").unwrap();
        assert!(
            planning_pos < drawer_pos,
            "planning line must come before the property drawer:\n{rest}"
        );
    }

    #[test]
    fn test_auto_closed_disabled_keeps_stale_closed() {
        let temp_dir = tempfile::tempdir().unwrap();
        setup_fixture(&temp_dir);
        let org_mode = OrgMode::new(OrgConfig {
            org_directory: temp_dir.path().to_str().unwrap().to_string(),
            org_auto_closed_timestamp: false,
            ..OrgConfig::default()
        })
        .unwrap();

        let mut e = update_by_id("task-book-789");
        e.todo_state = Some("TODO".to_string());
        org_mode.update_todo(e).unwrap();
        let content = fs::read_to_string(temp_dir.path().join("notes.org")).unwrap();
        assert!(content.contains("** TODO Read book"));
        assert!(
            content.contains("CLOSED: [2026-05-01 Fri 09:00]"),
            "config off keeps existing CLOSED:\n{content}"
        );

        // and no auto-stamp on a fresh DONE either
        let mut e2 = update_by_id("task-groceries-456");
        e2.todo_state = Some("DONE".to_string());
        org_mode.update_todo(e2).unwrap();
        let content = fs::read_to_string(temp_dir.path().join("notes.org")).unwrap();
        let groceries = &content[content.find("** DONE Buy groceries").unwrap()..];
        assert!(
            !groceries.lines().nth(1).unwrap_or("").contains("CLOSED:"),
            "config off must not stamp CLOSED:\n{groceries}"
        );
    }

    #[test]
    fn test_lock_file_cleaned_up() {
        let temp_dir = tempfile::tempdir().unwrap();
        setup_fixture(&temp_dir);
        let org_mode = make_org_mode(&temp_dir);

        let mut e = update_by_id("task-groceries-456");
        e.priority = Some("B".to_string());
        org_mode.update_todo(e).unwrap();

        assert!(!temp_dir.path().join(".notes.org.lock").exists());
    }

    #[test]
    fn test_update_planning_at_eof_no_trailing_newline() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(
            temp_dir.path().join("eof.org"),
            "* TODO Task\nSCHEDULED: <2026-05-15 Fri>", // no trailing newline
        )
        .unwrap();
        let org_mode = make_org_mode(&temp_dir);
        let e = UpdateEntry {
            id: None,
            file: Some("eof.org".to_string()),
            heading_path: Some("Task".to_string()),
            todo_state: Some("DONE".to_string()),
            priority: None,
            tags: None,
            scheduled: None,
            deadline: None,
            closed: None,
            clear: vec![],
            title: None,
            body: None,
            properties: None,
            remove_properties: None,
        };
        org_mode.update_todo(e).unwrap();
        let content = fs::read_to_string(temp_dir.path().join("eof.org")).unwrap();
        // Must not contain a duplicate SCHEDULED line
        assert_eq!(
            content.matches("SCHEDULED:").count(),
            1,
            "duplicate planning line:\n{content}"
        );
        assert!(content.contains("* DONE Task"));
    }

    #[test]
    fn test_update_multi_line_planning_no_orphan() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(
            temp_dir.path().join("multi.org"),
            "* TODO Task\nSCHEDULED: <2026-05-15 Fri>\nDEADLINE: <2026-05-20 Wed>\nbody\n",
        )
        .unwrap();
        let org_mode = make_org_mode(&temp_dir);
        let e = UpdateEntry {
            id: None,
            file: Some("multi.org".to_string()),
            heading_path: Some("Task".to_string()),
            todo_state: Some("DONE".to_string()),
            priority: None,
            tags: None,
            scheduled: None,
            deadline: None,
            closed: None,
            clear: vec![],
            title: None,
            body: None,
            properties: None,
            remove_properties: None,
        };
        org_mode.update_todo(e).unwrap();
        let content = fs::read_to_string(temp_dir.path().join("multi.org")).unwrap();
        assert_eq!(
            content.matches("SCHEDULED:").count(),
            1,
            "duplicate SCHEDULED:\n{content}"
        );
        assert_eq!(
            content.matches("DEADLINE:").count(),
            1,
            "duplicate DEADLINE:\n{content}"
        );
        assert!(
            content.contains("body"),
            "body line must survive:\n{content}"
        );
    }
}

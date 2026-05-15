use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::{fs, io, path::PathBuf};

use chrono::NaiveDate;
use orgize::export::{Container, Event, from_fn_with_ctx};
use orgize::{Org, ParseConfig, TextRange, TextSize};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use crate::OrgModeError;
use crate::org_mode::{CaptureEntry, CaptureResult, OrgMode, PropertyPair};

const MAX_HEADING_LEVEL: usize = 19;

struct HeadingSearchResult {
    insert_pos: TextSize,
    matched_depth: usize,
    last_matched_level: usize,
    remaining_parts: Vec<String>,
}

fn is_valid_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '@')
}

fn is_valid_property_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedTimestamp {
    pub(crate) date: NaiveDate,
    pub(crate) time: Option<chrono::NaiveTime>,
    pub(crate) repeater: Option<String>,
    pub(crate) warning: Option<String>,
}

impl OrgMode {
    pub fn capture_append(&self, entry: CaptureEntry) -> Result<CaptureResult, OrgModeError> {
        let file_rel = entry
            .file
            .as_deref()
            .unwrap_or(&self.config.org_default_notes_file);

        if entry.title.trim().is_empty() {
            return Err(OrgModeError::InvalidTitle(
                "title must not be empty or whitespace-only".to_string(),
            ));
        }
        if entry.title.contains('\n') || entry.title.contains('\r') {
            return Err(OrgModeError::InvalidTitle(
                "title must not contain newline or carriage return characters".to_string(),
            ));
        }

        if let Some(level) = entry.level
            && !(1..=MAX_HEADING_LEVEL).contains(&level)
        {
            return Err(OrgModeError::InvalidLevel(level));
        }

        Self::validate_relative_file_path(file_rel)?;

        if let Some(ref target) = entry.target_heading {
            for segment in target.split('/') {
                if segment.trim().is_empty() {
                    return Err(OrgModeError::InvalidHeadingPath(format!(
                        "target_heading contains an empty or whitespace-only segment: '{target}'"
                    )));
                }
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
                if !is_valid_tag(tag) {
                    return Err(OrgModeError::InvalidTag(tag.clone()));
                }
            }
        }

        let scheduled_ts = entry
            .scheduled
            .as_deref()
            .map(|v| Self::parse_iso_timestamp("scheduled", v))
            .transpose()?;
        let deadline_ts = entry
            .deadline
            .as_deref()
            .map(|v| Self::parse_iso_timestamp("deadline", v))
            .transpose()?;
        let closed_ts = entry
            .closed
            .as_deref()
            .map(|v| Self::parse_iso_timestamp("closed", v))
            .transpose()?;

        if entry.datetree_date.is_some() && !entry.datetree {
            return Err(OrgModeError::DatetreeDateWithoutFlag);
        }
        let datetree_date: Option<NaiveDate> = if entry.datetree {
            match entry.datetree_date.as_deref() {
                Some(s) => {
                    if s.contains(char::is_whitespace) {
                        return Err(OrgModeError::InvalidDatetreeDate(s.to_string()));
                    }
                    Some(
                        NaiveDate::parse_from_str(s, "%Y-%m-%d")
                            .map_err(|_| OrgModeError::InvalidDatetreeDate(s.to_string()))?,
                    )
                }
                None => Some(chrono::Local::now().date_naive()),
            }
        } else {
            None
        };

        let user_properties: Vec<PropertyPair> = match entry.properties {
            Some(ref ps) => {
                let mut seen: HashSet<String> = HashSet::new();
                for p in ps {
                    if !is_valid_property_key(&p.key) {
                        return Err(OrgModeError::InvalidPropertyKey(p.key.clone()));
                    }
                    if p.value.contains('\n') || p.value.contains('\r') {
                        return Err(OrgModeError::InvalidPropertyValue {
                            key: p.key.clone(),
                            reason: "value must not contain newline or carriage return".to_string(),
                        });
                    }
                    if !seen.insert(p.key.to_uppercase()) {
                        return Err(OrgModeError::DuplicatePropertyKey(p.key.clone()));
                    }
                }
                ps.clone()
            }
            None => Vec::new(),
        };

        let org_dir = PathBuf::from(&self.config.org_directory);
        let full_path = org_dir.join(file_rel);

        let canonical_org_dir = org_dir.canonicalize().map_err(|e| {
            OrgModeError::InvalidDirectory(format!(
                "Cannot canonicalize org directory '{}': {e}",
                self.config.org_directory
            ))
        })?;

        if full_path.exists() {
            let canonical_file = full_path.canonicalize().map_err(OrgModeError::IoError)?;
            if !canonical_file.starts_with(&canonical_org_dir) {
                return Err(OrgModeError::InvalidDirectory(format!(
                    "Path is outside org directory: {file_rel}"
                )));
            }
        } else if let Some(parent) = full_path.parent() {
            if parent.exists() {
                let canonical_parent = parent.canonicalize().map_err(OrgModeError::IoError)?;
                if !canonical_parent.starts_with(&canonical_org_dir) {
                    return Err(OrgModeError::InvalidDirectory(format!(
                        "Path is outside org directory: {file_rel}"
                    )));
                }
            } else {
                fs::create_dir_all(parent).map_err(OrgModeError::IoError)?;
                let canonical_parent = parent.canonicalize().map_err(OrgModeError::IoError)?;
                if !canonical_parent.starts_with(&canonical_org_dir) {
                    return Err(OrgModeError::InvalidDirectory(format!(
                        "Path is outside org directory: {file_rel}"
                    )));
                }
            }
        }

        let lock_path = Self::lock_path_for(&full_path)?;
        let lock_file = Self::acquire_capture_lock(&lock_path)?;

        let result: Result<CaptureResult, OrgModeError> = (|| {
            let content = if full_path.exists() {
                fs::read_to_string(&full_path).map_err(OrgModeError::IoError)?
            } else {
                String::new()
            };

            let parse_config = ParseConfig {
                todo_keywords: (
                    self.config.unfinished_keywords(),
                    self.config.finished_keywords(),
                ),
                ..Default::default()
            };
            let mut org = parse_config.parse(&content);

            let mut effective_target_parts: Vec<String> = Vec::new();
            if let Some(ref target) = entry.target_heading {
                effective_target_parts.extend(target.split('/').map(|s| s.trim().to_string()));
            }
            if let Some(d) = datetree_date {
                effective_target_parts.extend(Self::datetree_segments(d));
            }
            let effective_target = if effective_target_parts.is_empty() {
                None
            } else {
                Some(effective_target_parts.join("/"))
            };

            let (insert_pos, prefix_text, parent_level, under_target) =
                if let Some(ref target) = effective_target {
                    let search = self.find_heading_path(&org, target, content.len() as u32);

                    if search.remaining_parts.is_empty() {
                        (
                            search.insert_pos,
                            String::new(),
                            search.last_matched_level,
                            Some(target.clone()),
                        )
                    } else {
                        let base_level = if let Some(explicit_level) = entry.level {
                            let from_explicit =
                                explicit_level.saturating_sub(search.remaining_parts.len());
                            if search.matched_depth > 0 {
                                from_explicit.max(search.last_matched_level + 1)
                            } else {
                                from_explicit.max(1)
                            }
                        } else if search.matched_depth > 0 {
                            search.last_matched_level + 1
                        } else {
                            1
                        };

                        let mut prefix = String::new();
                        let mut last_level = search.last_matched_level;
                        for (i, part) in search.remaining_parts.iter().enumerate() {
                            let hlevel = (base_level + i).min(MAX_HEADING_LEVEL);
                            prefix.push_str(&"*".repeat(hlevel));
                            prefix.push(' ');
                            prefix.push_str(part);
                            prefix.push('\n');
                            last_level = hlevel;
                        }

                        (search.insert_pos, prefix, last_level, Some(target.clone()))
                    }
                } else {
                    let end = TextSize::from(content.len() as u32);
                    (end, String::new(), 0usize, None)
                };

            let level = match entry.level {
                Some(l) if under_target.is_some() => l.max(parent_level + 1).min(MAX_HEADING_LEVEL),
                Some(l) => l,
                None if under_target.is_some() => (parent_level + 1).min(MAX_HEADING_LEVEL),
                None => 1,
            };

            let heading_line = Self::format_heading(
                level,
                entry.todo_state.as_deref(),
                entry.priority.as_deref(),
                &entry.title,
                entry.tags.as_deref(),
            );

            let mut insert_text = String::new();
            if !content.is_empty() {
                insert_text.push('\n');
            }
            insert_text.push_str(&prefix_text);
            insert_text.push_str(&heading_line);
            insert_text.push('\n');

            let mut planning_parts: Vec<String> = Vec::new();
            if let Some(ref ts) = scheduled_ts {
                planning_parts.push(format!(
                    "SCHEDULED: {}",
                    Self::format_org_timestamp(ts, true)
                ));
            }
            if let Some(ref ts) = deadline_ts {
                planning_parts.push(format!(
                    "DEADLINE: {}",
                    Self::format_org_timestamp(ts, true)
                ));
            }
            if let Some(ref ts) = closed_ts {
                planning_parts.push(format!("CLOSED: {}", Self::format_org_timestamp(ts, false)));
            }
            if !planning_parts.is_empty() {
                insert_text.push_str(&planning_parts.join(" "));
                insert_text.push('\n');
            }

            let user_has_created = user_properties
                .iter()
                .any(|p| p.key.eq_ignore_ascii_case("CREATED"));
            let mut effective: Vec<PropertyPair> = Vec::new();
            if self.config.org_auto_created_property && !user_has_created {
                let now = chrono::Local::now();
                let dow = now.format("%a");
                effective.push(PropertyPair {
                    key: "CREATED".to_string(),
                    value: format!("[{} {dow} {}]", now.format("%Y-%m-%d"), now.format("%H:%M")),
                });
            }
            effective.extend(user_properties.iter().cloned());

            if !effective.is_empty() {
                insert_text.push_str(":PROPERTIES:\n");
                for pp in &effective {
                    insert_text.push_str(&format!(":{}: {}\n", pp.key, pp.value));
                }
                insert_text.push_str(":END:\n");
            }

            if let Some(ref body) = entry.body {
                insert_text.push_str(body);
                if !body.ends_with('\n') {
                    insert_text.push('\n');
                }
            }

            org.replace_range(TextRange::empty(insert_pos), &insert_text);
            let new_content = org.to_org();

            Self::atomic_write(&full_path, new_content.as_bytes())?;

            Ok(CaptureResult {
                file_path: file_rel.to_string(),
                level,
                heading_line,
                under_target,
            })
        })();

        let _ = fs::remove_file(&lock_path);
        drop(lock_file);

        result
    }

    fn lock_path_for(target: &Path) -> Result<PathBuf, OrgModeError> {
        let parent = target.parent().ok_or_else(|| {
            OrgModeError::IoError(io::Error::new(
                io::ErrorKind::InvalidInput,
                "target path has no parent directory",
            ))
        })?;
        let file_name = target.file_name().ok_or_else(|| {
            OrgModeError::IoError(io::Error::new(
                io::ErrorKind::InvalidInput,
                "target path has no file name",
            ))
        })?;
        let mut name = OsString::from(".");
        name.push(file_name);
        name.push(".lock");
        Ok(parent.join(name))
    }

    fn validate_relative_file_path(file_rel: &str) -> Result<(), OrgModeError> {
        use std::path::Component;
        let p = Path::new(file_rel);
        if p.is_absolute() {
            return Err(OrgModeError::InvalidDirectory(format!(
                "absolute path not allowed: {file_rel}"
            )));
        }
        for comp in p.components() {
            match comp {
                Component::Normal(_) | Component::CurDir => {}
                Component::ParentDir => {
                    return Err(OrgModeError::InvalidDirectory(format!(
                        "path traversal segment '..' not allowed: {file_rel}"
                    )));
                }
                Component::RootDir | Component::Prefix(_) => {
                    return Err(OrgModeError::InvalidDirectory(format!(
                        "absolute or drive-prefix path not allowed: {file_rel}"
                    )));
                }
            }
        }
        Ok(())
    }

    #[cfg(unix)]
    fn acquire_capture_lock(lock_path: &Path) -> Result<std::fs::File, OrgModeError> {
        loop {
            let fd = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(lock_path)
                .map_err(OrgModeError::IoError)?;
            fd.lock().map_err(OrgModeError::IoError)?;

            let our_ino = fd.metadata().map_err(OrgModeError::IoError)?.ino();
            match fs::metadata(lock_path) {
                Ok(m) if m.ino() == our_ino => return Ok(fd),
                _ => {
                    drop(fd);
                    continue;
                }
            }
        }
    }

    #[cfg(not(unix))]
    fn acquire_capture_lock(lock_path: &Path) -> Result<std::fs::File, OrgModeError> {
        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)
            .map_err(OrgModeError::IoError)?;
        fd.lock().map_err(OrgModeError::IoError)?;
        Ok(fd)
    }

    fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), OrgModeError> {
        let parent = target.parent().ok_or_else(|| {
            OrgModeError::IoError(io::Error::new(
                io::ErrorKind::InvalidInput,
                "target path has no parent directory",
            ))
        })?;

        let mut tmp = tempfile::Builder::new()
            .prefix(".")
            .tempfile_in(parent)
            .map_err(OrgModeError::IoError)?;

        tmp.write_all(bytes).map_err(OrgModeError::IoError)?;
        tmp.as_file().sync_all().map_err(OrgModeError::IoError)?;

        #[cfg(unix)]
        if let Ok(meta) = fs::metadata(target) {
            let _ = tmp.as_file().set_permissions(meta.permissions());
        }

        tmp.persist(target)
            .map_err(|e| OrgModeError::IoError(e.error))?;
        Ok(())
    }

    fn format_heading(
        level: usize,
        todo_state: Option<&str>,
        priority: Option<&str>,
        title: &str,
        tags: Option<&[String]>,
    ) -> String {
        let stars = "*".repeat(level);
        let mut parts = vec![stars];

        if let Some(kw) = todo_state {
            parts.push(kw.to_string());
        }
        if let Some(p) = priority {
            parts.push(format!("[#{p}]"));
        }
        parts.push(title.to_string());

        let mut line = parts.join(" ");

        if let Some(tags) = tags
            && !tags.is_empty()
        {
            let tag_str = format!(" :{}:", tags.join(":"));
            line.push_str(&tag_str);
        }

        line
    }

    pub(crate) fn parse_iso_timestamp(
        field: &'static str,
        value: &str,
    ) -> Result<ParsedTimestamp, OrgModeError> {
        let invalid = || OrgModeError::InvalidTimestamp {
            field,
            value: value.to_string(),
        };

        let mut tokens = value.split_whitespace();
        let date_tok = tokens.next().ok_or_else(invalid)?;
        let date = NaiveDate::parse_from_str(date_tok, "%Y-%m-%d").map_err(|_| invalid())?;

        let mut time: Option<chrono::NaiveTime> = None;
        let mut repeater: Option<String> = None;
        let mut warning: Option<String> = None;

        for tok in tokens {
            if time.is_none()
                && repeater.is_none()
                && warning.is_none()
                && let Ok(t) = chrono::NaiveTime::parse_from_str(tok, "%H:%M")
            {
                time = Some(t);
                continue;
            }

            if Self::is_repeater_suffix(tok) {
                if repeater.is_some() {
                    return Err(invalid());
                }
                repeater = Some(tok.to_string());
                continue;
            }
            if Self::is_warning_suffix(tok) {
                if warning.is_some() {
                    return Err(invalid());
                }
                warning = Some(tok.to_string());
                continue;
            }
            return Err(invalid());
        }

        Ok(ParsedTimestamp {
            date,
            time,
            repeater,
            warning,
        })
    }

    pub(crate) fn datetree_segments(date: NaiveDate) -> Vec<String> {
        vec![
            date.format("%Y").to_string(),
            date.format("%Y-%m %B").to_string(),
            date.format("%Y-%m-%d %A").to_string(),
        ]
    }

    pub(crate) fn format_org_timestamp(ts: &ParsedTimestamp, active: bool) -> String {
        let (open, close) = if active { ('<', '>') } else { ('[', ']') };
        let dow = ts.date.format("%a");
        let mut s = format!("{open}{} {dow}", ts.date.format("%Y-%m-%d"));
        if let Some(t) = ts.time {
            s.push_str(&format!(" {}", t.format("%H:%M")));
        }
        if let Some(r) = &ts.repeater {
            s.push(' ');
            s.push_str(r);
        }
        if let Some(w) = &ts.warning {
            s.push(' ');
            s.push_str(w);
        }
        s.push(close);
        s
    }

    fn is_repeater_suffix(tok: &str) -> bool {
        let body = if let Some(rest) = tok.strip_prefix("++") {
            rest
        } else if let Some(rest) = tok.strip_prefix(".+") {
            rest
        } else if let Some(rest) = tok.strip_prefix('+') {
            rest
        } else {
            return false;
        };
        Self::is_count_unit(body)
    }

    fn is_warning_suffix(tok: &str) -> bool {
        let body = match tok.strip_prefix('-') {
            Some(rest) => rest,
            None => return false,
        };
        Self::is_count_unit(body)
    }

    fn is_count_unit(s: &str) -> bool {
        if s.len() < 2 {
            return false;
        }
        let mut chars = s.chars();
        let unit = chars.next_back().unwrap();
        if !matches!(unit, 'h' | 'd' | 'w' | 'm' | 'y') {
            return false;
        }
        let num: String = chars.collect();
        if num.is_empty() {
            return false;
        }
        matches!(
            num.parse::<u32>(),
            Ok(n) if n > 0 && num.chars().all(|c| c.is_ascii_digit())
        )
    }

    fn find_heading_path(
        &self,
        org: &Org,
        heading_path: &str,
        content_len: u32,
    ) -> HeadingSearchResult {
        let path_parts: Vec<&str> = heading_path.split('/').collect();
        let total = path_parts.len();

        let mut open_stack: Vec<(usize, Option<usize>)> = Vec::new();
        let mut matched = 0usize;
        let mut insert_pos = TextSize::from(content_len);
        let mut last_level = 0usize;

        let mut handler = from_fn_with_ctx(|event, ctx| match event {
            Event::Enter(Container::Headline(h)) => {
                let level = h.level();

                while let Some(&(top_level, _)) = open_stack.last() {
                    if top_level >= level {
                        open_stack.pop();
                    } else {
                        break;
                    }
                }

                let mut step_matched_depth: Option<usize> = None;
                if matched < total {
                    let part = path_parts[matched];
                    let parent_ok = if matched == 0 {
                        true
                    } else {
                        open_stack
                            .last()
                            .map(|&(_, d)| d == Some(matched - 1))
                            .unwrap_or(false)
                    };
                    if parent_ok && h.title_raw() == part {
                        insert_pos = h.end();
                        last_level = level;
                        step_matched_depth = Some(matched);
                        matched += 1;
                        if matched == total {
                            ctx.stop();
                        }
                    }
                }

                open_stack.push((level, step_matched_depth));
            }
            Event::Leave(Container::Headline(h)) => {
                let level = h.level();
                while let Some(&(top_level, _)) = open_stack.last() {
                    if top_level >= level {
                        open_stack.pop();
                        if top_level == level {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
            _ => {}
        });

        org.traverse(&mut handler);

        HeadingSearchResult {
            insert_pos,
            matched_depth: matched,
            last_matched_level: last_level,
            remaining_parts: path_parts[matched..]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_format_heading_simple() {
        let result = OrgMode::format_heading(1, None, None, "My Title", None);
        assert_eq!(result, "* My Title");
    }

    #[test]
    fn test_format_heading_with_todo() {
        let result = OrgMode::format_heading(2, Some("TODO"), None, "Task", None);
        assert_eq!(result, "** TODO Task");
    }

    #[test]
    fn test_format_heading_with_priority() {
        let result = OrgMode::format_heading(1, None, Some("A"), "Important", None);
        assert_eq!(result, "* [#A] Important");
    }

    #[test]
    fn test_format_heading_with_tags() {
        let tags = vec!["work".to_string(), "urgent".to_string()];
        let result = OrgMode::format_heading(1, None, None, "Title", Some(&tags));
        assert_eq!(result, "* Title :work:urgent:");
    }

    #[test]
    fn test_format_heading_full() {
        let tags = vec!["proj".to_string()];
        let result = OrgMode::format_heading(2, Some("TODO"), Some("B"), "My Task", Some(&tags));
        assert_eq!(result, "** TODO [#B] My Task :proj:");
    }

    #[test]
    fn test_parse_iso_timestamp_date_only() {
        let ts = OrgMode::parse_iso_timestamp("scheduled", "2026-05-15").unwrap();
        assert_eq!(ts.date, NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        assert!(ts.time.is_none());
        assert!(ts.repeater.is_none());
        assert!(ts.warning.is_none());
    }

    #[test]
    fn test_parse_iso_timestamp_with_time() {
        let ts = OrgMode::parse_iso_timestamp("deadline", "2026-05-15 14:30").unwrap();
        assert_eq!(ts.date, NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
        assert_eq!(
            ts.time,
            Some(chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap())
        );
    }

    #[test]
    fn test_parse_iso_timestamp_rejects_garbage() {
        for bad in ["2026/05/15", "tomorrow", "<2026-05-15 Fri>", ""] {
            let err = OrgMode::parse_iso_timestamp("scheduled", bad).unwrap_err();
            assert!(matches!(err, OrgModeError::InvalidTimestamp { .. }));
        }
    }

    #[test]
    fn test_parse_iso_timestamp_with_repeater() {
        let ts = OrgMode::parse_iso_timestamp("scheduled", "2026-05-15 ++1w").unwrap();
        assert_eq!(ts.repeater.as_deref(), Some("++1w"));
        assert!(ts.warning.is_none());
    }

    #[test]
    fn test_parse_iso_timestamp_with_warning() {
        let ts = OrgMode::parse_iso_timestamp("deadline", "2026-05-15 -3d").unwrap();
        assert!(ts.repeater.is_none());
        assert_eq!(ts.warning.as_deref(), Some("-3d"));
    }

    #[test]
    fn test_parse_iso_timestamp_with_time_repeater_warning() {
        let ts = OrgMode::parse_iso_timestamp("scheduled", "2026-05-15 14:30 ++1w -3d").unwrap();
        assert_eq!(
            ts.time,
            Some(chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap())
        );
        assert_eq!(ts.repeater.as_deref(), Some("++1w"));
        assert_eq!(ts.warning.as_deref(), Some("-3d"));
    }

    #[test]
    fn test_parse_iso_timestamp_repeater_other_forms() {
        for r in ["+1d", "++2w", ".+3m"] {
            let raw = format!("2026-05-15 {r}");
            let ts = OrgMode::parse_iso_timestamp("scheduled", &raw).unwrap();
            assert_eq!(ts.repeater.as_deref(), Some(r));
        }
    }

    #[test]
    fn test_parse_iso_timestamp_rejects_two_repeaters() {
        let err = OrgMode::parse_iso_timestamp("scheduled", "2026-05-15 +1d ++1w").unwrap_err();
        assert!(matches!(err, OrgModeError::InvalidTimestamp { .. }));
    }

    #[test]
    fn test_parse_iso_timestamp_rejects_two_warnings() {
        let err = OrgMode::parse_iso_timestamp("scheduled", "2026-05-15 -1d -3d").unwrap_err();
        assert!(matches!(err, OrgModeError::InvalidTimestamp { .. }));
    }

    #[test]
    fn test_parse_iso_timestamp_rejects_unknown_suffix() {
        let err = OrgMode::parse_iso_timestamp("scheduled", "2026-05-15 garbage").unwrap_err();
        assert!(matches!(err, OrgModeError::InvalidTimestamp { .. }));
    }

    #[test]
    fn test_parse_iso_timestamp_rejects_zero_count() {
        for bad in ["2026-05-15 +0d", "2026-05-15 -0w", "2026-05-15 ++0m"] {
            let err = OrgMode::parse_iso_timestamp("scheduled", bad).unwrap_err();
            assert!(matches!(err, OrgModeError::InvalidTimestamp { .. }));
        }
    }

    #[test]
    fn test_parse_iso_timestamp_rejects_leading_zero_count() {
        for bad in ["2026-05-15 +00d", "2026-05-15 -000w", "2026-05-15 ++0m"] {
            let err = OrgMode::parse_iso_timestamp("scheduled", bad).unwrap_err();
            assert!(matches!(err, OrgModeError::InvalidTimestamp { .. }));
        }
    }

    #[test]
    fn test_parse_iso_timestamp_rejects_multibyte_utf8() {
        let err = OrgMode::parse_iso_timestamp("scheduled", "2026-05-15 +1\u{00F6}").unwrap_err();
        assert!(matches!(err, OrgModeError::InvalidTimestamp { .. }));
    }

    fn ts(
        date: (i32, u32, u32),
        time: Option<(u32, u32)>,
        rep: Option<&str>,
        warn: Option<&str>,
    ) -> ParsedTimestamp {
        ParsedTimestamp {
            date: NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap(),
            time: time.map(|(h, m)| chrono::NaiveTime::from_hms_opt(h, m, 0).unwrap()),
            repeater: rep.map(String::from),
            warning: warn.map(String::from),
        }
    }

    #[test]
    fn test_format_timestamp_active_date_only() {
        let s = OrgMode::format_org_timestamp(&ts((2026, 5, 15), None, None, None), true);
        assert_eq!(s, "<2026-05-15 Fri>");
    }

    #[test]
    fn test_format_timestamp_inactive_date_only() {
        let s = OrgMode::format_org_timestamp(&ts((2026, 5, 10), None, None, None), false);
        assert_eq!(s, "[2026-05-10 Sun]");
    }

    #[test]
    fn test_format_timestamp_active_with_time() {
        let s = OrgMode::format_org_timestamp(&ts((2026, 5, 15), Some((14, 30)), None, None), true);
        assert_eq!(s, "<2026-05-15 Fri 14:30>");
    }

    #[test]
    fn test_format_timestamp_inactive_with_time() {
        let s =
            OrgMode::format_org_timestamp(&ts((2026, 5, 15), Some((14, 30)), None, None), false);
        assert_eq!(s, "[2026-05-15 Fri 14:30]");
    }

    #[test]
    fn test_format_timestamp_with_repeater_and_warning() {
        let s = OrgMode::format_org_timestamp(
            &ts((2026, 5, 15), Some((14, 30)), Some("++1w"), Some("-3d")),
            true,
        );
        assert_eq!(s, "<2026-05-15 Fri 14:30 ++1w -3d>");
    }

    #[test]
    fn test_datetree_segments_format() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 10).unwrap();
        let segs = OrgMode::datetree_segments(date);
        assert_eq!(segs, vec!["2026", "2026-05 May", "2026-05-10 Sunday"]);
    }

    #[test]
    fn test_datetree_segments_january() {
        let date = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        let segs = OrgMode::datetree_segments(date);
        assert_eq!(segs, vec!["2027", "2027-01 January", "2027-01-01 Friday"]);
    }

    #[test]
    fn test_is_count_unit_rejects_short_input() {
        assert!(!OrgMode::is_count_unit(""));
        assert!(!OrgMode::is_count_unit("d"));
    }
}

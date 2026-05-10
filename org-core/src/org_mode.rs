use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::{convert::TryFrom, fs, io, path::PathBuf};

use chrono::{DateTime, Datelike, Days, Duration, Local, Months, NaiveDate, TimeZone};
use globset::{Glob, GlobSetBuilder};
use ignore::{Walk, WalkBuilder};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as NucleoConfig, Matcher};
use orgize::ast::{Headline, PropertyDrawer, Timestamp};
use orgize::export::{Container, Event, from_fn, from_fn_with_ctx};
use orgize::{Org, ParseConfig, TextRange, TextSize};
use rowan::ast::AstNode;
use serde::{Deserialize, Serialize};
use shellexpand::tilde;

use crate::OrgModeError;
use crate::config::{OrgConfig, load_org_config};
use crate::utils::tags_match;

/// Macro to convert org-mode Timestamp to chrono::NaiveDateTime
///
/// Takes a timestamp and a prefix (start/end) and automatically constructs
/// the appropriate method calls (year_start/year_end, etc.).
macro_rules! convert_timestamp {
    ($ts:expr, $prefix:ident) => {{
        pastey::paste! {
            let year = $ts.[<year_ $prefix>]()?;
            let month = $ts.[<month_ $prefix>]()?;
            let day = $ts.[<day_ $prefix>]()?;
            let hour = $ts.[<hour_ $prefix>]();
            let minute = $ts.[<minute_ $prefix>]();

            Some(chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(
                    year.parse().ok()?,
                    month.parse().ok()?,
                    day.parse().ok()?,
                )?,
                chrono::NaiveTime::from_hms_opt(
                    hour.map(|v| v.parse().unwrap_or_default())
                        .unwrap_or_default(),
                    minute
                        .map(|v| v.parse().unwrap_or_default())
                        .unwrap_or_default(),
                    0,
                )?,
            ))
        }
    }};
}

#[derive(Debug)]
pub struct OrgMode {
    config: OrgConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    pub label: String,
    pub level: usize,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<TreeNode>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file_path: String,
    pub snippet: String,
    pub score: u32,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoState {
    Todo,
    Done,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    A,
    B,
    C,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgendaItem {
    pub file_path: String,
    pub heading: String,
    pub level: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    // TODO: review type (string vs enum)
    pub todo_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    // TODO: review type (string vs enum)
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgendaView {
    pub items: Vec<AgendaItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
}

#[derive(Default)]
pub enum AgendaViewType {
    Today,
    Day(DateTime<Local>),
    #[default]
    CurrentWeek,
    Week(u8),
    CurrentMonth,
    Month(u32),
    Custom {
        from: DateTime<Local>,
        to: DateTime<Local>,
    },
}

/// Single key/value pair for the org property drawer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyPair {
    pub key: String,
    pub value: String,
}

/// Input for [`OrgMode::capture_append`]: describes a heading to append to an org file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureEntry {
    /// Heading title. Must be non-empty (after trimming) and contain no `\n`/`\r`.
    pub title: String,
    /// Heading level (1..=19). Auto-derived if omitted: `parent_level + 1` when
    /// `target_heading` is set, otherwise 1. If supplied with `target_heading`,
    /// the level is silently bumped to `parent_level + 1` if smaller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<usize>,
    /// TODO keyword (must match a configured keyword in `org_todo_keywords`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todo_state: Option<String>,
    /// Tags. Each must match `^[A-Za-z0-9_@]+$`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<Vec<String>>,
    /// Priority cookie: `A`, `B`, or `C`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    /// Body content placed beneath the heading. Trailing newline is added if missing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Relative path under `org_directory`. Defaults to `org_default_notes_file`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Slash-separated heading path the new entry should live under, e.g. `Projects/Work`.
    /// Each segment must be a direct child of the previous match — siblings of the
    /// matched chain do **not** satisfy a deeper segment. If the path doesn't exist,
    /// missing levels are created beneath the deepest existing match. The first match
    /// in document order is used when duplicates exist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_heading: Option<String>,
    /// SCHEDULED active timestamp. ISO: "YYYY-MM-DD" or "YYYY-MM-DD HH:MM",
    /// optionally followed by repeater (`+N{u}|++N{u}|.+N{u}`) and warning (`-N{u}`)
    /// tokens where `u` ∈ `{h, d, w, m, y}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled: Option<String>,
    /// DEADLINE active timestamp. Same grammar as `scheduled`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    /// CLOSED inactive timestamp. Same input grammar as `scheduled`; formatted as
    /// `[YYYY-MM-DD Day]` (inactive brackets) on output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed: Option<String>,
    /// Property drawer entries written in given order. Empty list ≡ omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Vec<PropertyPair>>,
    /// When true, expand `target_heading` with a Year/Month/Day datetree before
    /// resolution so the entry lands under the chosen day's leaf.
    #[serde(default)]
    pub datetree: bool,
    /// Optional override for the datetree day. Date-only ISO ("YYYY-MM-DD"); no
    /// time. Defaults to today (local time) when `datetree = true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datetree_date: Option<String>,
}

/// Result returned from a successful [`OrgMode::capture_append`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResult {
    /// Path the heading was written to, relative to `org_directory`.
    pub file_path: String,
    /// Final heading level used (after auto-derivation / bumping).
    pub level: usize,
    /// The full heading line written to disk (e.g., `** TODO [#A] Title :tag:`).
    pub heading_line: String,
    /// The `target_heading` argument echoed back, when one was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub under_target: Option<String>,
}

impl TreeNode {
    pub fn new(label: String) -> Self {
        Self {
            label,
            level: 0,
            children: Vec::new(),
            tags: Vec::new(),
        }
    }

    pub fn new_with_level(label: String, level: usize) -> Self {
        Self {
            label,
            level,
            children: Vec::new(),
            tags: Vec::new(),
        }
    }

    pub fn to_indented_string(&self, indent: usize) -> String {
        let mut result = String::new();
        let prefix = "  ".repeat(indent);
        result.push_str(&format!(
            "{}{} {}\n",
            prefix,
            "*".repeat(self.level),
            self.label
        ));

        for child in &self.children {
            result.push_str(&child.to_indented_string(indent + 1));
        }

        result
    }
}

impl OrgMode {
    pub fn new(config: OrgConfig) -> Result<Self, OrgModeError> {
        let config = config.validate()?;

        Ok(OrgMode { config })
    }

    pub fn with_defaults() -> Result<Self, OrgModeError> {
        Self::new(load_org_config(None, None)?)
    }

    pub fn config(&self) -> &OrgConfig {
        &self.config
    }
}

impl OrgMode {
    pub fn list_files(
        &self,
        tags: Option<&[String]>,
        limit: Option<usize>,
    ) -> Result<Vec<String>, OrgModeError> {
        Walk::new(&self.config.org_directory)
            .filter_map(|entry| match entry {
                Ok(dir_entry) => {
                    let path = dir_entry.path();

                    if path.is_file()
                        && let Some(extension) = path.extension()
                        && extension == "org"
                        && let Ok(relative_path) = path.strip_prefix(&self.config.org_directory)
                        && let Some(path_str) = relative_path.to_str()
                    {
                        Some(Ok(path_str.to_string()))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(OrgModeError::WalkError(e))),
            })
            .collect::<Result<Vec<String>, OrgModeError>>()
            .map(|files| {
                files
                    .into_iter()
                    .filter(|path| {
                        if let Some(tags) = tags {
                            let file_tags = self.tags_in_file(path).unwrap_or_default();
                            tags.iter().any(|tag| file_tags.contains(tag))
                        } else {
                            true
                        }
                    })
                    .take(limit.unwrap_or(usize::MAX))
                    .collect::<Vec<String>>()
            })
    }

    pub fn search(
        &self,
        query: &str,
        limit: Option<usize>,
        snippet_max_size: Option<usize>,
    ) -> Result<Vec<SearchResult>, OrgModeError> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }

        let mut matcher = Matcher::new(NucleoConfig::DEFAULT);
        let pattern = Pattern::new(
            query,
            CaseMatching::Ignore,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );

        let files = self.list_files(None, None)?;
        let mut all_results = Vec::new();

        for file in files {
            let content = match self.read_file(&file) {
                Ok(content) => content,
                Err(_) => continue, // Skip files that can't be read
            };

            let matches = pattern.match_list(
                content.lines().map(|s| s.to_owned()).collect::<Vec<_>>(),
                &mut matcher,
            );

            for (snippet, score) in matches {
                let snippet = Self::snippet(&snippet, snippet_max_size.unwrap_or(100));
                all_results.push(SearchResult {
                    file_path: file.clone(),
                    snippet,
                    score,
                    tags: self.tags_in_file(&file).unwrap_or_default(),
                });
            }
        }

        all_results.sort_by_key(|b| std::cmp::Reverse(b.score));
        all_results.truncate(limit.unwrap_or(all_results.len()));

        Ok(all_results)
    }

    pub fn search_with_tags(
        &self,
        query: &str,
        tags: Option<&[String]>,
        limit: Option<usize>,
        snippet_max_size: Option<usize>,
    ) -> Result<Vec<SearchResult>, OrgModeError> {
        self.search(query, None, snippet_max_size)
            .map(|results| match tags {
                Some(filter_tags) => results
                    .into_iter()
                    .filter(|r| filter_tags.iter().any(|tag| r.tags.contains(tag)))
                    .collect(),
                None => results,
            })
            .map(|mut all_results| {
                all_results.truncate(limit.unwrap_or(all_results.len()));
                all_results
            })
    }

    pub fn list_files_by_tags(&self, tags: &[String]) -> Result<Vec<String>, OrgModeError> {
        self.list_files(Some(tags), None)
    }

    pub fn read_file(&self, path: &str) -> Result<String, OrgModeError> {
        let org_dir = PathBuf::from(&self.config.org_directory);
        let full_path = org_dir.join(path);

        if !full_path.exists() {
            return Err(OrgModeError::IoError(io::Error::new(
                io::ErrorKind::NotFound,
                format!("File not found: {}", path),
            )));
        }

        if !full_path.is_file() {
            return Err(OrgModeError::IoError(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Path is not a file: {}", path),
            )));
        }

        fs::read_to_string(full_path).map_err(OrgModeError::IoError)
    }

    pub fn get_outline(&self, path: &str) -> Result<TreeNode, OrgModeError> {
        let content = self.read_file(path)?;

        let mut root = TreeNode::new("Document".into());
        let mut stack: Vec<TreeNode> = Vec::new();

        let mut handler = from_fn(|event| {
            if let Event::Enter(Container::Headline(h)) = event {
                let level = h.level();
                let label = h.title_raw();
                let tags = h.tags().map(|s| s.to_string()).collect();
                let node = TreeNode {
                    label,
                    level,
                    tags,
                    children: Vec::new(),
                };

                while let Some(n) = stack.last() {
                    if n.level < level {
                        break;
                    }
                    let finished_node = stack.pop().unwrap();
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(finished_node);
                    } else {
                        root.children.push(finished_node);
                    }
                }

                stack.push(node);
            }
        });

        Org::parse(&content).traverse(&mut handler);

        while let Some(node) = stack.pop() {
            if let Some(parent) = stack.last_mut() {
                parent.children.push(node);
            } else {
                root.children.push(node);
            }
        }

        Ok(root)
    }

    pub fn get_heading(&self, path: &str, heading: &str) -> Result<String, OrgModeError> {
        let content = self.read_file(path)?;

        let heading_path: Vec<&str> = heading.split('/').collect();
        let mut current_level = 0;
        let mut found = None;

        let mut handler = from_fn_with_ctx(|event, ctx| {
            if let Event::Enter(Container::Headline(h)) = event {
                let title = h.title_raw();
                let level = h.level();

                if let Some(part) = heading_path.get(current_level) {
                    if title == *part {
                        if level == heading_path.len() {
                            found = Some(h);
                            ctx.stop();
                        }
                        current_level += 1;
                    }
                } else {
                    ctx.stop()
                }
            }
        });

        Org::parse(&content).traverse(&mut handler);

        found
            .ok_or_else(|| OrgModeError::InvalidHeadingPath(heading.into()))
            .map(|h| h.raw())
    }

    pub fn get_element_by_id(&self, id: &str) -> Result<String, OrgModeError> {
        let files = self.list_files(None, None)?;

        let found = files.iter().find_map(|path| {
            self.read_file(path)
                .map(|content| self.search_id(content, id))
                .unwrap_or_default()
        });

        found.ok_or_else(|| OrgModeError::InvalidElementId(id.into()))
    }

    fn search_id(&self, content: String, id: &str) -> Option<String> {
        let mut found = None;
        let has_id_property = |properties: Option<PropertyDrawer>| {
            properties
                .and_then(|props| {
                    props
                        .to_hash_map()
                        .into_iter()
                        .find(|(k, v)| k.to_lowercase() == "id" && v == id)
                })
                .is_some()
        };
        let mut handler = from_fn_with_ctx(|event, ctx| {
            if let Event::Enter(Container::Headline(ref h)) = event
                && has_id_property(h.properties())
            {
                found = Some(h.raw());
                ctx.stop();
            } else if let Event::Enter(Container::Document(ref d)) = event
                && has_id_property(d.properties())
            {
                found = Some(d.raw());
                ctx.stop();
            }
        });

        Org::parse(&content).traverse(&mut handler);

        found
    }

    fn snippet(s: &str, max: usize) -> String {
        if s.chars().count() > max {
            s.chars().take(max).collect::<String>() + "..."
        } else {
            s.to_string()
        }
    }

    fn tags_in_file(&self, path: &str) -> Result<Vec<String>, OrgModeError> {
        let content = self.read_file(path)?;
        let mut tags = Vec::new();

        let mut handler = from_fn(|event| {
            if let Event::Enter(Container::Headline(h)) = event {
                tags.extend(h.tags().map(|s| s.to_string()));
            }
        });

        Org::parse(&content).traverse(&mut handler);

        Ok(tags)
    }

    fn files_in_path(&self, path: &str) -> Result<impl Iterator<Item = PathBuf>, OrgModeError> {
        let org_root = PathBuf::from(&self.config.org_directory);

        let path = tilde(&path).into_owned();
        let path = if PathBuf::from(&path).is_absolute() {
            path.to_string()
        } else {
            org_root.join(&path).to_str().unwrap_or(&path).to_string()
        };

        let path = if PathBuf::from(&path).is_dir() {
            format!("{}/**/*.org", path.trim_end_matches('/'))
        } else {
            path
        };

        let root = path
            .split_once('*')
            .map(|(prefix, _)| prefix)
            .unwrap_or(&path);

        let globset = GlobSetBuilder::new().add(Glob::new(&path)?).build()?;

        let iter = WalkBuilder::new(root)
            .build()
            .flatten()
            .filter(move |e| e.path().is_file() && globset.is_match(e.path()))
            .map(|e| e.path().to_path_buf());

        Ok(iter)
    }

    fn agenda_tasks(&self) -> impl Iterator<Item = (Headline, String)> {
        self.config
            .org_agenda_files
            .iter()
            .filter_map(|loc| self.files_in_path(loc).ok())
            .flatten()
            .collect::<HashSet<_>>()
            .into_iter()
            .flat_map(|file| {
                let config = ParseConfig {
                    todo_keywords: (
                        self.config.unfinished_keywords(),
                        self.config.finished_keywords(),
                    ),
                    ..Default::default()
                };
                // TODO: handle file read errors
                let org = config.parse(fs::read_to_string(&file).unwrap_or_default());

                let org_root = Path::new(&self.config.org_directory);

                let mut tasks = Vec::new();
                let mut handler = from_fn(|event| {
                    if let Event::Enter(container) = event
                        && let Container::Headline(headline) = container
                        && (headline.is_todo() || headline.is_done())
                    {
                        let file_path = file
                            .strip_prefix(org_root)
                            .unwrap_or(&file)
                            .to_string_lossy()
                            .to_string();
                        tasks.push((headline, file_path));
                    }
                });
                org.traverse(&mut handler);
                tasks
            })
    }

    /// List all tasks (TODO/DONE items) across all org files
    ///
    /// # Arguments
    /// * `todo_states` - Optional filter for specific TODO states (e.g., ["TODO", "DONE"])
    /// * `tags` - Optional filter by tags
    /// * `priority` - Optional filter by priority level
    /// * `limit` - Maximum number of items to return
    ///
    /// # Returns
    /// Vector of `AgendaItem` containing task information
    pub fn list_tasks(
        &self,
        todo_states: Option<&[String]>,
        tags: Option<&[String]>,
        priority: Option<Priority>,
        limit: Option<usize>,
    ) -> Result<Vec<AgendaItem>, OrgModeError> {
        let tasks = self
            .agenda_tasks()
            .filter(|(headline, _)| {
                headline.is_todo()
                    && tags
                        .map(|tags| {
                            tags_match(
                                &headline.tags().map(|s| s.to_string()).collect::<Vec<_>>(),
                                tags,
                            )
                        })
                        .unwrap_or(true)
                    && priority
                        .as_ref()
                        .map(|p| {
                            if let Some(prio) = headline.priority() {
                                let prio = prio.to_string();
                                matches!(
                                    (p, prio.as_str()),
                                    (Priority::A, "A") | (Priority::B, "B") | (Priority::C, "C")
                                )
                            } else {
                                *p == Priority::None
                            }
                        })
                        .unwrap_or(true)
                    && todo_states
                        .map(|states| {
                            headline
                                .todo_keyword()
                                .map(|kw| states.contains(&kw.to_string()))
                                .unwrap_or(false)
                        })
                        .unwrap_or(true)
            })
            .map(|(headline, file_path)| Self::headline_to_agenda_item(&headline, file_path))
            .take(limit.unwrap_or(usize::MAX))
            .collect::<Vec<_>>();

        Ok(tasks)
    }

    /// Get agenda view organized by date
    ///
    /// # Arguments
    /// * `agenda_view_type` - Type of agenda view (e.g., Today, CurrentWeek, Custom range)
    /// * `todo_states` - Optional filter for specific TODO states
    /// * `tags` - Optional filter by tags
    ///
    /// # Returns
    /// `AgendaView` containing items within the date range
    pub fn get_agenda_view(
        &self,
        agenda_view_type: AgendaViewType,
        todo_states: Option<&[String]>,
        tags: Option<&[String]>,
        limit: Option<usize>,
    ) -> Result<AgendaView, OrgModeError> {
        let mut items = self
            .agenda_tasks()
            .filter(|(headline, _)| {
                tags.map(|tags| {
                    tags_match(
                        &headline.tags().map(|s| s.to_string()).collect::<Vec<_>>(),
                        tags,
                    )
                })
                .unwrap_or(true)
                    && todo_states
                        .map(|states| {
                            headline
                                .todo_keyword()
                                .map(|kw| states.contains(&kw.to_string()))
                                .unwrap_or(false)
                        })
                        .unwrap_or(true)
                    && self.is_in_agenda_range(headline, &agenda_view_type)
            })
            .map(|(headline, file_path)| Self::headline_to_agenda_item(&headline, file_path))
            .collect::<Vec<_>>();

        if let Some(limit) = limit {
            items.truncate(limit);
        }

        Ok(AgendaView {
            items,
            start_date: Some(agenda_view_type.start_date().format("%Y-%m-%d").to_string()),
            end_date: Some(agenda_view_type.end_date().format("%Y-%m-%d").to_string()),
        })
    }

    // TODO: support recurrent tasks
    fn is_in_agenda_range(&self, headline: &Headline, agenda_view_type: &AgendaViewType) -> bool {
        let start_date = agenda_view_type.start_date();
        let end_date = agenda_view_type.end_date();

        let timestamps = headline
            .syntax()
            .children()
            .filter(|c| !Headline::can_cast(c.kind()))
            .flat_map(|node| node.descendants().filter_map(Timestamp::cast))
            .filter(|ts| ts.is_active())
            .filter(|ts| {
                headline.scheduled().map(|s| &s != ts).unwrap_or(true)
                    && headline.deadline().map(|s| &s != ts).unwrap_or(true)
            })
            .collect::<Vec<_>>();

        let is_within_range = |ts_opt: Option<Timestamp>| {
            // more info https://orgmode.org/org.html#Repeated-tasks
            if let Some(ts) = ts_opt
                && let Some(date) = OrgMode::start_to_chrono(&ts)
                && let Some(date) = match Local.from_local_datetime(&date) {
                    chrono::LocalResult::Single(t) => Some(t),
                    chrono::LocalResult::Ambiguous(t, _) => Some(t),
                    chrono::LocalResult::None => {
                        let dt_plus_1 = date + chrono::Duration::hours(1);
                        match Local.from_local_datetime(&dt_plus_1) {
                            chrono::LocalResult::Single(t) => Some(t),
                            chrono::LocalResult::Ambiguous(t, _) => Some(t),
                            chrono::LocalResult::None => None,
                        }
                    }
                }
            {
                if let Some(repeater_value) = ts.repeater_value()
                    && let Some(repeater_unit) = ts.repeater_unit()
                {
                    let value = repeater_value as u64;
                    let mut current_date =
                        OrgMode::add_repeater_duration(date, value, &repeater_unit);

                    while current_date < start_date {
                        current_date =
                            OrgMode::add_repeater_duration(current_date, value, &repeater_unit);
                    }

                    current_date >= start_date && current_date <= end_date
                } else {
                    date >= start_date && date <= end_date
                }
            } else {
                false
            }
        };

        is_within_range(headline.scheduled())
            || is_within_range(headline.deadline())
            || timestamps.into_iter().any(|ts| is_within_range(Some(ts)))
    }

    pub fn start_to_chrono(ts: &Timestamp) -> Option<chrono::NaiveDateTime> {
        convert_timestamp!(ts, start)
    }

    pub fn end_to_chrono(ts: &Timestamp) -> Option<chrono::NaiveDateTime> {
        convert_timestamp!(ts, end)
    }
}

/// Maximum heading depth supported by org-mode parsers (orgize/Emacs).
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

/// Parsed components of a capture-supplied timestamp.
#[derive(Debug, Clone)]
pub(crate) struct ParsedTimestamp {
    pub(crate) date: NaiveDate,
    pub(crate) time: Option<chrono::NaiveTime>,
    pub(crate) repeater: Option<String>,
    pub(crate) warning: Option<String>,
}

// Capture (append heading) functionality
impl OrgMode {
    /// Append a new heading (and optional body) to an org file, optionally under a target heading.
    ///
    /// # Behavior
    ///
    /// - When `entry.target_heading` is `None`, the heading is appended to the end of the file.
    /// - When `entry.target_heading` is `Some("A/B/C")`, the path is resolved against the file's
    ///   heading hierarchy. Each segment must be a direct child of the previous match (no
    ///   sibling-subtree matches). If any segment is missing, intermediate headings are created
    ///   beneath the deepest existing match.
    /// - If multiple headings match the path, the **first** match (in file order) is used.
    ///
    /// # Level resolution priority
    ///
    /// 1. Explicit `entry.level` is honored when it is structurally consistent (≥ parent + 1).
    ///    If the explicit level is *less* than `parent + 1`, it is silently bumped to `parent + 1`
    ///    so the result is still well-formed.
    /// 2. Otherwise, when `target_heading` is supplied, the level defaults to `parent_level + 1`.
    /// 3. Otherwise, level defaults to 1.
    ///
    /// # Validation
    ///
    /// - `title` must be non-empty after trimming, and must not contain `\n` or `\r`.
    /// - `level`, when provided, must satisfy `1 ≤ level ≤ 19`.
    /// - `todo_state`, when provided, must be a configured TODO keyword.
    /// - `priority`, when provided, must be `A`, `B`, or `C`.
    /// - `tags` entries must match `^[A-Za-z0-9_@]+$` (the org-mode tag character set).
    ///
    /// # Concurrency & durability
    ///
    /// The implementation takes an exclusive OS-level file lock for the duration of the
    /// read-modify-write critical section, then atomically swaps the file via temp-file +
    /// `rename`. Concurrent captures from this process or any other will not lose data, and
    /// a process crash mid-write cannot corrupt the target file.
    pub fn capture_append(&self, entry: CaptureEntry) -> Result<CaptureResult, OrgModeError> {
        let file_rel = entry
            .file
            .as_deref()
            .unwrap_or(&self.config.org_default_notes_file);

        // Validate title: non-empty, no newlines or carriage returns.
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

        // Validate explicit level
        if let Some(level) = entry.level
            && !(1..=MAX_HEADING_LEVEL).contains(&level)
        {
            return Err(OrgModeError::InvalidLevel(level));
        }

        // Validate TODO keyword against config
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

        // Validate priority
        if let Some(ref p) = entry.priority
            && !matches!(p.as_str(), "A" | "B" | "C")
        {
            return Err(OrgModeError::InvalidPriority(p.clone()));
        }

        // Validate tags
        if let Some(ref tags) = entry.tags {
            for tag in tags {
                if !is_valid_tag(tag) {
                    return Err(OrgModeError::InvalidTag(tag.clone()));
                }
            }
        }

        // Validate planning timestamps before acquiring the lock.
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

        // Datetree validation: date-only, requires flag.
        if entry.datetree_date.is_some() && !entry.datetree {
            return Err(OrgModeError::DatetreeDateWithoutFlag);
        }
        let datetree_date: Option<NaiveDate> = if entry.datetree {
            match entry.datetree_date.as_deref() {
                Some(s) => {
                    // Reject anything that isn't a bare YYYY-MM-DD (no time, no suffixes).
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

        // Validate properties.
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
                    if !seen.insert(p.key.clone()) {
                        return Err(OrgModeError::DuplicatePropertyKey(p.key.clone()));
                    }
                }
                ps.clone()
            }
            None => Vec::new(),
        };

        // Resolve full path and validate it's under org_directory.
        // Fail-closed if org_dir cannot canonicalize (issue #7).
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
            }
        }

        // Acquire a sibling .lock file as the concurrency mutex. We can't lock the
        // target itself, since atomic-rename replaces its inode and stale locks would
        // no longer guard the path. The lockfile is unlinked on release; a stat-after-
        // lock retry inside `acquire_capture_lock` keeps the unlink race-free (issue #2).
        let lock_path = Self::lock_path_for(&full_path)?;
        let lock_file = Self::acquire_capture_lock(&lock_path)?;

        // Run the rest under a closure so we always unlink the lockfile on the way out,
        // even on error paths.
        let result: Result<CaptureResult, OrgModeError> = (|| {
            // Inside the lock, read the current target content (file may not exist yet).
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

            // Compose the effective target_heading: optional user-supplied prefix,
            // then optional datetree segments (year/month/day).
            let mut effective_target_parts: Vec<String> = Vec::new();
            if let Some(ref target) = entry.target_heading {
                effective_target_parts.extend(target.split('/').map(String::from));
            }
            if let Some(d) = datetree_date {
                effective_target_parts.extend(Self::datetree_segments(d));
            }
            let effective_target = if effective_target_parts.is_empty() {
                None
            } else {
                Some(effective_target_parts.join("/"))
            };

            // Find insertion point and determine level
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
                            let hlevel = base_level + i;
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

            let level = entry.level.unwrap_or_else(|| {
                if under_target.is_some() {
                    parent_level + 1
                } else {
                    1
                }
            });

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

            // Planning line (SCHEDULED, DEADLINE, CLOSED) — order is fixed.
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

            // Property drawer: prepend auto-CREATED (when enabled and the user
            // hasn't supplied their own CREATED), then extend with user properties.
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

            // Atomic write: write to a temp file in the same directory, fsync, then rename.
            // The lock on `lock_file` is still held while we do this; rename(2) replaces the
            // target inode atomically on POSIX (issue #3).
            Self::atomic_write(&full_path, new_content.as_bytes())?;

            Ok(CaptureResult {
                file_path: file_rel.to_string(),
                level,
                heading_line,
                under_target,
            })
        })();

        // Unlink the lockfile *before* releasing the lock. Any blocked acquirer waiting
        // on this inode will, when its lock returns, observe stat(lock_path) yielding a
        // different inode (or NotFound) and will retry — picking up a fresh lock.
        let _ = fs::remove_file(&lock_path);
        drop(lock_file);

        result
    }

    /// Sibling lock file for `target`.
    ///
    /// Lives in the same directory as the target so concurrent processes (CLI and MCP
    /// server) coordinate via the same path. Cleaned up on successful capture.
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
        let mut name = std::ffi::OsString::from(".");
        name.push(file_name);
        name.push(".lock");
        Ok(parent.join(name))
    }

    /// Acquire an exclusive lock on `lock_path`, retrying if another writer recreates
    /// the lockfile under us between our open and lock.
    ///
    /// The retry loop closes a small race that arises when a previous holder unlinks
    /// the lockfile on release: another process might have already created a fresh
    /// lockfile at the same path before our `lock()` call returns. We detect this by
    /// comparing the inode of our open fd against the inode of the path; on mismatch,
    /// we drop our lock and retry.
    ///
    /// On non-unix platforms there is no inode concept; we trust the lock as-is.
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

            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let our_ino = fd.metadata().map_err(OrgModeError::IoError)?.ino();
                match fs::metadata(lock_path) {
                    Ok(m) if m.ino() == our_ino => return Ok(fd),
                    // Mismatch or NotFound: another writer recreated the lockfile
                    // under us. Release and retry.
                    _ => {
                        drop(fd);
                        continue;
                    }
                }
            }
            #[cfg(not(unix))]
            {
                return Ok(fd);
            }
        }
    }

    /// Write `bytes` to `target` atomically: write to a unique sibling temp file,
    /// fsync, then rename over the target. The temp file name embeds the process ID
    /// and a counter so concurrent writers within one process do not collide on the
    /// temp path. The caller is responsible for serializing concurrent writes via
    /// `lock_path_for`.
    fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), OrgModeError> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);

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

        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut tmp_name = std::ffi::OsString::from(".");
        tmp_name.push(file_name);
        tmp_name.push(format!(".tmp.{}.{}", std::process::id(), counter));
        let tmp_path = parent.join(&tmp_name);

        {
            let mut tmp = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_path)
                .map_err(OrgModeError::IoError)?;
            tmp.write_all(bytes).map_err(OrgModeError::IoError)?;
            tmp.sync_all().map_err(OrgModeError::IoError)?;
        }

        if let Err(e) = fs::rename(&tmp_path, target) {
            let _ = fs::remove_file(&tmp_path);
            return Err(OrgModeError::IoError(e));
        }
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

    /// Parse an ISO-form timestamp ("YYYY-MM-DD" or "YYYY-MM-DD HH:MM") with optional
    /// trailing repeater (`+N{u}` / `++N{u}` / `.+N{u}`) and warning (`-N{u}`) tokens
    /// where `u` ∈ `{h, d, w, m, y}`. At most one repeater and one warning per
    /// timestamp.
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
            // Time must appear before any suffix token; once a suffix is set,
            // a "HH:MM"-shaped token falls through to the unknown-token branch.
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

    /// Build the three datetree heading segments (year, year-month, year-month-day)
    /// for the given date. English month and weekday names match Emacs defaults.
    pub(crate) fn datetree_segments(date: NaiveDate) -> Vec<String> {
        vec![
            date.format("%Y").to_string(),
            date.format("%Y-%m %B").to_string(),
            date.format("%Y-%m-%d %A").to_string(),
        ]
    }

    /// Render a [`ParsedTimestamp`] as canonical org-mode syntax.
    /// `active = true` produces `<...>`; `false` produces `[...]`.
    pub(crate) fn format_org_timestamp(ts: &ParsedTimestamp, active: bool) -> String {
        let (open, close) = if active { ('<', '>') } else { ('[', ']') };
        let dow = ts.date.format("%a"); // English Mon..Sun
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
        // `+N{u}`, `++N{u}`, or `.+N{u}` where u ∈ {h,d,w,m,y}
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
        // `-N{u}` where u ∈ {h,d,w,m,y}
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
        let (num, unit) = s.split_at(s.len() - 1);
        if !matches!(unit, "h" | "d" | "w" | "m" | "y") {
            return false;
        }
        !num.is_empty() && num != "0" && num.chars().all(|c| c.is_ascii_digit())
    }

    /// Resolve a slash-separated heading path against `org`, requiring each segment to
    /// be a direct descendant of the previous match.
    ///
    /// Tracks open headlines via `Enter`/`Leave` events so that, e.g., `A/Work` does not
    /// match a `Work` headline that lives under sibling `B`. The next path segment can
    /// only match a headline whose immediate ancestor (on the open-headline stack) was
    /// the previously matched node.
    ///
    /// If multiple headings match the path, the **first** match in document order is
    /// returned (matching Emacs `org-capture` semantics). Once a step matches, the
    /// recorded `insert_pos` / `matched_depth` persist even after the matched ancestor
    /// closes — so a partial match correctly anchors the missing-descendants creation.
    fn find_heading_path(
        &self,
        org: &Org,
        heading_path: &str,
        content_len: u32,
    ) -> HeadingSearchResult {
        let path_parts: Vec<&str> = heading_path.split('/').collect();
        let total = path_parts.len();

        // Stack of (level, was-matched-at-this-step) frames for currently open headlines.
        // `matched`-flagged frames carry the chain forward — only their descendants can
        // satisfy the next path segment.
        let mut open_stack: Vec<(usize, bool)> = Vec::new();
        let mut matched = 0usize;
        let mut insert_pos = TextSize::from(content_len);
        let mut last_level = 0usize;

        let mut handler = from_fn_with_ctx(|event, ctx| match event {
            Event::Enter(Container::Headline(h)) => {
                let level = h.level();

                // Pop frames whose level is >= this headline's level — they're not ancestors.
                while let Some(&(top_level, _)) = open_stack.last() {
                    if top_level >= level {
                        open_stack.pop();
                    } else {
                        break;
                    }
                }

                let mut step_matched = false;
                if matched < total {
                    let part = path_parts[matched];
                    // The next path segment can only match if its parent in the document
                    // is the previously matched headline. For `matched == 0`, any
                    // top-level headline is eligible. For `matched > 0`, the top of the
                    // open stack must carry the prior match.
                    let parent_ok = if matched == 0 {
                        true
                    } else {
                        open_stack.last().map(|&(_, m)| m).unwrap_or(false)
                    };
                    if parent_ok && h.title_raw() == part {
                        insert_pos = h.end();
                        last_level = level;
                        matched += 1;
                        step_matched = true;
                        if matched == total {
                            ctx.stop();
                        }
                    }
                }

                open_stack.push((level, step_matched));
            }
            Event::Leave(Container::Headline(h)) => {
                let level = h.level();
                // Pop frames at or below this headline's level. We do NOT roll back
                // `matched`: a partial match remains valid as the anchor for descendant
                // creation, even after the matched ancestor closes.
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

// DateTime helper functions
impl OrgMode {
    /// Convert a DateTime to start of day (00:00:00)
    fn to_start_of_day(date: DateTime<Local>) -> DateTime<Local> {
        date.date_naive()
            .and_hms_opt(0, 0, 0)
            .and_then(|dt| match Local.from_local_datetime(&dt) {
                chrono::LocalResult::Single(t) => Some(t),
                chrono::LocalResult::Ambiguous(t, _) => Some(t),
                chrono::LocalResult::None => {
                    let dt_plus_1 = dt + chrono::Duration::hours(1);
                    match Local.from_local_datetime(&dt_plus_1) {
                        chrono::LocalResult::Single(t) => Some(t),
                        chrono::LocalResult::Ambiguous(t, _) => Some(t),
                        chrono::LocalResult::None => None,
                    }
                }
            })
            .unwrap_or(date)
    }

    /// Convert a DateTime to end of day (23:59:59.999)
    fn to_end_of_day(date: DateTime<Local>) -> DateTime<Local> {
        date.date_naive()
            .and_hms_opt(23, 59, 59)
            .and_then(|dt| match Local.from_local_datetime(&dt) {
                chrono::LocalResult::Single(t) => Some(t),
                chrono::LocalResult::Ambiguous(t, _) => Some(t),
                chrono::LocalResult::None => {
                    let dt_minus_1 = dt - chrono::Duration::hours(1);
                    match Local.from_local_datetime(&dt_minus_1) {
                        chrono::LocalResult::Single(t) => Some(t),
                        chrono::LocalResult::Ambiguous(t, _) => Some(t),
                        chrono::LocalResult::None => None,
                    }
                }
            })
            .unwrap_or(date)
    }

    /// Convert NaiveDate to local DateTime with specified time
    fn naive_date_to_local(
        date: NaiveDate,
        hour: u32,
        min: u32,
        sec: u32,
    ) -> Result<DateTime<Local>, OrgModeError> {
        date.and_hms_opt(hour, min, sec)
            .and_then(|dt| match Local.from_local_datetime(&dt) {
                chrono::LocalResult::Single(t) => Some(t),
                chrono::LocalResult::Ambiguous(t, _) => Some(t),
                chrono::LocalResult::None => {
                    let dt_plus_1 = dt + chrono::Duration::hours(1);
                    match Local.from_local_datetime(&dt_plus_1) {
                        chrono::LocalResult::Single(t) => Some(t),
                        chrono::LocalResult::Ambiguous(t, _) => Some(t),
                        chrono::LocalResult::None => None,
                    }
                }
            })
            .ok_or_else(|| {
                OrgModeError::InvalidAgendaViewType(format!(
                    "Could not convert date '{}' to local timezone",
                    date
                ))
            })
    }

    /// Get the last day of the month for a given date
    fn last_day_of_month(date: DateTime<Local>) -> DateTime<Local> {
        let month = date.month();
        let year = date.year();

        // Get first day of next month
        let (next_month, next_year) = if month == 12 {
            (1, year + 1)
        } else {
            (month + 1, year)
        };

        // First day of next month at midnight
        let next_month_first = Self::to_start_of_day(
            date.with_year(next_year)
                .unwrap()
                .with_day(1)
                .unwrap()
                .with_month(next_month)
                .unwrap(),
        );

        // Subtract one day to get last day of current month
        next_month_first - Duration::days(1)
    }

    /// Add a repeater duration to a date based on org-mode time units
    ///
    /// # Arguments
    /// * `date` - The starting date
    /// * `value` - The numeric value for the duration
    /// * `unit` - The time unit (Hour, Day, Week, Month, Year)
    fn add_repeater_duration(
        date: DateTime<Local>,
        value: u64,
        unit: &orgize::ast::TimeUnit,
    ) -> DateTime<Local> {
        match unit {
            orgize::ast::TimeUnit::Hour => Some(date + Duration::hours(value as i64)),
            orgize::ast::TimeUnit::Day => date.checked_add_days(Days::new(value)),
            orgize::ast::TimeUnit::Week => date.checked_add_days(Days::new(value * 7)),
            orgize::ast::TimeUnit::Month => date.checked_add_months(Months::new(value as u32)),
            orgize::ast::TimeUnit::Year => date.checked_add_months(Months::new(value as u32 * 12)),
        }
        .unwrap_or(date)
    }

    /// Convert a Headline to an AgendaItem
    ///
    /// # Arguments
    /// * `headline` - The org-mode headline to convert
    /// * `file_path` - The file path containing the headline
    fn headline_to_agenda_item(headline: &Headline, file_path: String) -> AgendaItem {
        AgendaItem {
            file_path,
            heading: headline.title_raw(),
            level: headline.level(),
            todo_state: headline.todo_keyword().map(|t| t.to_string()),
            priority: headline.priority().map(|p| p.to_string()),
            deadline: headline.deadline().map(|d| d.raw()),
            scheduled: headline.scheduled().map(|d| d.raw()),
            tags: headline.tags().map(|s| s.to_string()).collect(),
            position: Some(Position {
                start: headline.start().into(),
                end: headline.end().into(),
            }),
        }
    }

    /// Parse a date string in YYYY-MM-DD format with contextual error messages
    ///
    /// # Arguments
    /// * `date_str` - The date string to parse
    /// * `context` - Context for error messages (e.g., "from date", "to date")
    fn parse_date_string(date_str: &str, context: &str) -> Result<NaiveDate, OrgModeError> {
        NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_| {
            OrgModeError::InvalidAgendaViewType(format!(
                "Invalid {context} '{date_str}', expected YYYY-MM-DD"
            ))
        })
    }
}

// TODO: improve date management
impl AgendaViewType {
    pub fn start_date(&self) -> DateTime<Local> {
        let date = match self {
            AgendaViewType::Today => Local::now(),
            AgendaViewType::Day(d) => *d,
            AgendaViewType::CurrentWeek => {
                let now = Local::now();
                let weekday = now.weekday().num_days_from_monday();
                now - Duration::days(weekday as i64)
            }
            AgendaViewType::Week(week_num) => {
                let now = Local::now();
                let year_start =
                    OrgMode::to_start_of_day(now.with_month(1).unwrap().with_day(1).unwrap());
                year_start + Duration::weeks(*week_num as i64)
            }
            AgendaViewType::CurrentMonth => {
                let now = Local::now();
                now.with_day(1).unwrap()
            }
            AgendaViewType::Month(month) => {
                // FIXME: improve error handling
                let now = Local::now().with_day(1).unwrap_or(Local::now());
                now.with_month(*month).unwrap_or(now).with_day(1).unwrap()
            }
            AgendaViewType::Custom { from, .. } => *from,
        };
        OrgMode::to_start_of_day(date)
    }

    pub fn end_date(&self) -> DateTime<Local> {
        let date = match self {
            AgendaViewType::Today => Local::now(),
            AgendaViewType::Day(d) => *d,
            AgendaViewType::CurrentWeek => {
                let now = Local::now();
                let weekday = now.weekday().num_days_from_monday();
                let start = now - Duration::days(weekday as i64);
                start + Duration::days(6)
            }
            AgendaViewType::Week(week_num) => {
                let now = Local::now();
                let year_start =
                    OrgMode::to_start_of_day(now.with_month(1).unwrap().with_day(1).unwrap());
                let target_week_start = year_start + Duration::weeks(*week_num as i64);
                target_week_start + Duration::days(6)
            }
            AgendaViewType::CurrentMonth => {
                let now = Local::now();
                OrgMode::last_day_of_month(now)
            }
            AgendaViewType::Month(month) => {
                // FIXME: improve error handling
                let now = Local::now().with_day(1).unwrap_or(Local::now());
                let date_in_month = now.with_month(*month).unwrap_or(now);
                OrgMode::last_day_of_month(date_in_month)
            }
            AgendaViewType::Custom { to, .. } => *to,
        };
        OrgMode::to_end_of_day(date)
    }
}

/// Possible values to convert
/// "": default
/// "today": Today
/// "day/YYYY-MM-DD": specific day
/// "week": current week
/// "week/N": week number N
/// "month": current month
/// "month/N": month number N
/// "query/from/YYYY-MM-DD/to/YYYY-MM-DD": custom range
impl TryFrom<&str> for AgendaViewType {
    type Error = OrgModeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Ok(AgendaViewType::default());
        }

        match value {
            "today" => Ok(AgendaViewType::Today),
            "week" => Ok(AgendaViewType::CurrentWeek),
            "month" => Ok(AgendaViewType::CurrentMonth),
            _ => {
                // Try to parse more complex patterns
                let parts: Vec<&str> = value.split('/').collect();

                match parts.as_slice() {
                    ["day", date_str] => {
                        // Parse YYYY-MM-DD format
                        let parsed_date = OrgMode::parse_date_string(date_str, "date format")?;
                        let datetime = OrgMode::naive_date_to_local(parsed_date, 0, 0, 0)?;
                        Ok(AgendaViewType::Day(datetime))
                    }
                    ["week", week_str] => {
                        // Parse week number
                        let week_num = week_str.parse::<u8>().map_err(|_| {
                            OrgModeError::InvalidAgendaViewType(format!(
                                "Invalid week number '{}', expected 0-53",
                                week_str
                            ))
                        })?;
                        if week_num > 53 {
                            return Err(OrgModeError::InvalidAgendaViewType(format!(
                                "Week number {} out of range, expected 0-53",
                                week_num
                            )));
                        }
                        Ok(AgendaViewType::Week(week_num))
                    }
                    ["month", month_str] => {
                        // Parse month number
                        let month_num = month_str.parse::<u32>().map_err(|_| {
                            OrgModeError::InvalidAgendaViewType(format!(
                                "Invalid month number '{}', expected 1-12",
                                month_str
                            ))
                        })?;
                        if !(1..=12).contains(&month_num) {
                            return Err(OrgModeError::InvalidAgendaViewType(format!(
                                "Month number {} out of range, expected 1-12",
                                month_num
                            )));
                        }
                        Ok(AgendaViewType::Month(month_num))
                    }
                    ["query", "from", from_str, "to", to_str] => {
                        // Parse custom date range
                        let from_date = OrgMode::parse_date_string(from_str, "from date")?;
                        let to_date = OrgMode::parse_date_string(to_str, "to date")?;

                        let from_datetime = OrgMode::naive_date_to_local(from_date, 0, 0, 0)?;
                        let to_datetime = OrgMode::naive_date_to_local(to_date, 23, 59, 59)?;

                        if from_datetime > to_datetime {
                            return Err(OrgModeError::InvalidAgendaViewType(
                                "From date must be before to date".into(),
                            ));
                        }
                        Ok(AgendaViewType::Custom {
                            from: from_datetime,
                            to: to_datetime,
                        })
                    }
                    _ => Err(OrgModeError::InvalidAgendaViewType(format!(
                        "Unknown agenda view type format: '{}'",
                        value
                    ))),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;
    use orgize::ast::TimeUnit;

    #[test]
    fn test_add_repeater_duration_hour() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 14, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 2, &TimeUnit::Hour);

        assert_eq!(result.hour(), 16);
        assert_eq!(result.day(), 15);
    }

    #[test]
    fn test_add_repeater_duration_day() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 5, &TimeUnit::Day);

        assert_eq!(result.day(), 20);
        assert_eq!(result.month(), 6);
    }

    #[test]
    fn test_add_repeater_duration_week() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 2, &TimeUnit::Week);

        assert_eq!(result.day(), 29);
        assert_eq!(result.month(), 6);
    }

    #[test]
    fn test_add_repeater_duration_month() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 3, &TimeUnit::Month);

        assert_eq!(result.month(), 9);
        assert_eq!(result.day(), 15);
        assert_eq!(result.year(), 2025);
    }

    #[test]
    fn test_add_repeater_duration_year() {
        let date = Local.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 2, &TimeUnit::Year);

        assert_eq!(result.year(), 2027);
        assert_eq!(result.month(), 6);
        assert_eq!(result.day(), 15);
    }

    #[test]
    fn test_add_repeater_duration_month_boundary() {
        let date = Local.with_ymd_and_hms(2025, 10, 15, 12, 0, 0).unwrap();
        let result = OrgMode::add_repeater_duration(date, 3, &TimeUnit::Month);

        assert_eq!(result.year(), 2026);
        assert_eq!(result.month(), 1);
        assert_eq!(result.day(), 15);
    }

    #[test]
    fn test_last_day_of_month_from_day_31() {
        // This tests the bug fix: when on day 31, getting last day of a month
        // with fewer days (like February) should work correctly
        let date = Local.with_ymd_and_hms(2025, 1, 31, 12, 0, 0).unwrap();
        let result = OrgMode::last_day_of_month(date);

        assert_eq!(result.month(), 1);
        assert_eq!(result.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_february() {
        let date = Local.with_ymd_and_hms(2025, 2, 15, 12, 0, 0).unwrap();
        let result = OrgMode::last_day_of_month(date);

        assert_eq!(result.month(), 2);
        assert_eq!(result.day(), 28);
    }

    #[test]
    fn test_last_day_of_month_leap_year() {
        let date = Local.with_ymd_and_hms(2024, 2, 15, 12, 0, 0).unwrap();
        let result = OrgMode::last_day_of_month(date);

        assert_eq!(result.month(), 2);
        assert_eq!(result.day(), 29);
    }

    // --- Capture tests ---

    fn make_org_mode(temp_dir: &tempfile::TempDir) -> OrgMode {
        use crate::config::OrgConfig;
        OrgMode::new(OrgConfig {
            org_directory: temp_dir.path().to_str().unwrap().to_string(),
            ..OrgConfig::default()
        })
        .unwrap()
    }

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
        // "New Project" should appear between "Projects" and "Archive"
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

    // Issue #1: target_heading must respect parent/child hierarchy.
    // A path "A/Work" must NOT match Work that lives under a sibling B.
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

        // Expected: "Work" is created as a child of A (since the existing
        // ** Work is under B, not A); the new item lands under A/Work.
        assert_eq!(
            result.level, 3,
            "Item must end up at level 3 (A>Work>Item), got file:\n{content}"
        );
        // The original ** Work under B must be unchanged.
        assert!(
            content.contains("* B\n** Work"),
            "B's ** Work must remain intact:\n{content}"
        );
        // A new ** Work must exist under A.
        let a_pos = content.find("* A").unwrap();
        let b_pos = content.find("* B").unwrap();
        let item_pos = content.find("*** Item").expect("Item must be at level 3");
        assert!(
            item_pos > a_pos && item_pos < b_pos,
            "Item must be inserted under A (between A and B), got:\n{content}"
        );
    }

    // Issue #4: empty title should be rejected.
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

    // Issue #4: whitespace-only title should be rejected.
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

    // Issue #4: titles containing newlines break heading structure.
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

    // Issue #4: titles with carriage returns break heading structure.
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

    // Issue #4: level=0 produces output with no leading stars (not a heading).
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

    // Issue #4: ridiculously deep level should be rejected.
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

    // Issue #5: tags with spaces produce malformed :tag: blocks.
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

    // Issue #5: tags with colons split into multiple tags silently.
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

    // Issue #5: empty tags should be rejected.
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

    // Issue #5: tags matching ^[A-Za-z0-9_@]+$ remain accepted.
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

    // The lockfile must not remain in the org directory after a successful capture.
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

    // Issue #2: concurrent captures from multiple threads must not lose data.
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
        // Format check: ":CREATED: [YYYY-MM-DD Day HH:MM]"
        let re = regex::Regex::new(r":CREATED: \[\d{4}-\d{2}-\d{2} [A-Z][a-z]{2} \d{2}:\d{2}\]")
            .unwrap();
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
        // No second CREATED line
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
        // Year/month/day should appear ONCE.
        assert_eq!(
            content.matches("* 2026\n").count(),
            1,
            "year heading must be unique:\n{content}"
        );
        assert_eq!(content.matches("** 2026-05 May").count(), 1);
        assert_eq!(content.matches("*** 2026-05-10 Sunday").count(), 1);
        // Both items present at level 4.
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
        // Canonical order: heading → planning line → drawer.
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

        // Round-trip via orgize.
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

        // Validate orgize sees the planning markers.
        assert!(h.scheduled().is_some(), "scheduled should round-trip");
        assert!(h.deadline().is_some(), "deadline should round-trip");

        // Property drawer round-trips with the user-supplied keys (case-sensitive).
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

        // Substring assertions for the parts orgize doesn't expose directly.
        assert!(content.contains("CLOSED: [2026-05-10 Sun]"));
        assert!(content.contains("* TODO [#A] Full :work:"));
        assert!(content.contains("Body text."));
    }
}

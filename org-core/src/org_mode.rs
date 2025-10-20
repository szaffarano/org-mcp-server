use std::collections::HashSet;
use std::path::Path;
use std::{convert::TryFrom, fs, io, path::PathBuf};

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Timelike};
use globset::{Glob, GlobSetBuilder};
use ignore::{Walk, WalkBuilder};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as NucleoConfig, Matcher};
use orgize::ast::PropertyDrawer;
use orgize::export::{Container, Event, from_fn, from_fn_with_ctx};
use orgize::{Org, ParseConfig};
use serde::{Deserialize, Serialize};
use shellexpand::tilde;

use crate::OrgModeError;
use crate::config::{OrgConfig, load_org_config};

#[cfg(test)]
#[path = "org_mode_tests.rs"]
mod org_mode_tests;

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
    // TODO: review type
    pub line_number: Option<usize>,
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

        all_results.sort_by(|a, b| b.score.cmp(&a.score));
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
            org_root.join(&path).to_str().unwrap().to_string()
        };

        let root = path
            .split_once('*')
            .map(|(prefix, _)| prefix)
            .unwrap_or(&path);

        let globset = GlobSetBuilder::new()
            .add(Glob::new(&path)?)
            .build()
            .unwrap();

        let iter = WalkBuilder::new(root)
            .build()
            .flatten()
            .filter(move |e| e.path().is_file() && globset.is_match(e.path()))
            .map(|e| e.path().to_path_buf());

        Ok(iter)
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
        _todo_states: Option<&[String]>,
        _tags: Option<&[String]>,
        _priority: Option<Priority>,
        _limit: Option<usize>,
    ) -> Result<Vec<AgendaItem>, OrgModeError> {
        // 1. Iterate through all org files using list_files()
        // 2. Parse each file with orgize to find headlines with TODO keywords
        // 3. Extract todo_state, priority, scheduled, deadline, tags from each headline
        // 4. TODO: Filter based on todo_states, tags, and priority parameters
        // 5. Apply limit if specified

        let files = self
            .config
            .org_agenda_files
            .iter()
            .filter_map(|loc| self.files_in_path(loc).ok())
            .flatten()
            .collect::<HashSet<_>>();

        let tasks = files
            .iter()
            .flat_map(|file| {
                let config = ParseConfig {
                    todo_keywords: (
                        self.config.unfinished_keywords(),
                        self.config.finished_keywords(),
                    ),
                    ..Default::default()
                };
                let org = config.parse(fs::read_to_string(file).unwrap_or_default());

                let org_root = Path::new(&self.config.org_directory);

                let mut tasks = Vec::new();
                let mut handler = from_fn(|event| {
                    if let Event::Enter(container) = event
                        && let Container::Headline(headline) = container
                        && headline.is_todo()
                    {
                        let task = AgendaItem {
                            file_path: file
                                .strip_prefix(org_root)
                                .unwrap_or(file)
                                .to_string_lossy()
                                .to_string(),
                            heading: headline.title_raw(),
                            level: headline.level(),
                            todo_state: headline.todo_keyword().map(|t| t.to_string()),
                            priority: headline.priority().map(|p| p.to_string()),
                            deadline: headline.deadline().map(|d| d.raw()),
                            scheduled: headline.scheduled().map(|d| d.raw()),
                            tags: vec![],
                            line_number: Some(8),
                        };
                        tasks.push(task);
                    }
                });
                org.traverse(&mut handler);
                tasks
            })
            .take(_limit.unwrap_or(usize::MAX))
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
        _todo_states: Option<&[String]>,
        _tags: Option<&[String]>,
    ) -> Result<AgendaView, OrgModeError> {
        // TODO: Implement orgize parsing logic for agenda view
        // 1. Get all tasks using list_tasks()
        // 2. Filter by SCHEDULED and DEADLINE dates within start_date to end_date range
        // 3. Organize items chronologically
        // 4. Return AgendaView with sorted items

        // Mock implementation for testing
        let items = vec![AgendaItem {
            file_path: "project.org".to_string(),
            heading: "Set up development environment".to_string(),
            level: 3,
            todo_state: Some("TODO".to_string()),
            priority: Some("B".to_string()),
            scheduled: Some("2025-10-19".to_string()),
            deadline: None,
            tags: vec![],
            line_number: Some(18),
        }];

        Ok(AgendaView {
            items,
            start_date: Some(agenda_view_type.start_date().format("%Y-%m-%d").to_string()),
            end_date: Some(agenda_view_type.end_date().format("%Y-%m-%d").to_string()),
        })
    }
}

impl AgendaViewType {
    pub fn start_date(&self) -> DateTime<Local> {
        match self {
            AgendaViewType::Today => Local::now(),
            AgendaViewType::Day(d) => *d,
            AgendaViewType::CurrentWeek => {
                let now = Local::now();
                let weekday = now.weekday().num_days_from_monday();
                now - Duration::days(weekday as i64)
            }
            AgendaViewType::Week(week_num) => {
                let now = Local::now();
                let year_start = now
                    .with_month(1)
                    .unwrap()
                    .with_day(1)
                    .unwrap()
                    .with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap();
                year_start + Duration::weeks(*week_num as i64)
            }
            AgendaViewType::CurrentMonth => Local::now()
                .with_day(1)
                .unwrap()
                .with_hour(0)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap(),
            AgendaViewType::Month(month) => {
                let now = Local::now();
                now.with_month(*month)
                    .unwrap_or(now)
                    .with_day(1)
                    .unwrap()
                    .with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap()
            }
            AgendaViewType::Custom { from, .. } => *from,
        }
    }

    pub fn end_date(&self) -> DateTime<Local> {
        match self {
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
                let year_start = now
                    .with_month(1)
                    .unwrap()
                    .with_day(1)
                    .unwrap()
                    .with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap();
                let target_week_start = year_start + Duration::weeks(*week_num as i64);
                target_week_start + Duration::days(6)
            }
            AgendaViewType::CurrentMonth => {
                let now = Local::now();
                let month = now.month();
                let start = now
                    .with_month(month)
                    .unwrap_or(now)
                    .with_day(1)
                    .unwrap()
                    .with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap();

                let next_month = if month == 12 {
                    start
                        .with_year(start.year() + 1)
                        .unwrap()
                        .with_month(1)
                        .unwrap()
                } else {
                    start.with_month(month + 1).unwrap()
                };
                next_month - Duration::days(1)
            }
            AgendaViewType::Month(month) => {
                let now = Local::now();
                let start = now
                    .with_month(*month)
                    .unwrap_or(now)
                    .with_day(1)
                    .unwrap()
                    .with_hour(0)
                    .unwrap()
                    .with_minute(0)
                    .unwrap()
                    .with_second(0)
                    .unwrap();

                // Get last day of month by going to next month and subtracting a day
                let next_month = if *month == 12 {
                    start
                        .with_year(start.year() + 1)
                        .unwrap()
                        .with_month(1)
                        .unwrap()
                } else {
                    start.with_month(*month + 1).unwrap()
                };
                next_month - Duration::days(1)
            }
            AgendaViewType::Custom { to, .. } => *to,
        }
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
                        let parsed_date =
                            NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_| {
                                OrgModeError::InvalidAgendaViewType(format!(
                                    "Invalid date format '{}', expected YYYY-MM-DD",
                                    date_str
                                ))
                            })?;

                        let datetime = Local
                            .from_local_datetime(
                                &parsed_date.and_hms_opt(0, 0, 0).unwrap_or_default(),
                            )
                            .single()
                            .ok_or_else(|| {
                                OrgModeError::InvalidAgendaViewType(format!(
                                    "Could not convert date '{}' to local timezone",
                                    date_str
                                ))
                            })?;
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
                        let from_date =
                            NaiveDate::parse_from_str(from_str, "%Y-%m-%d").map_err(|_| {
                                OrgModeError::InvalidAgendaViewType(format!(
                                    "Invalid from date '{}', expected YYYY-MM-DD",
                                    from_str
                                ))
                            })?;
                        let to_date =
                            NaiveDate::parse_from_str(to_str, "%Y-%m-%d").map_err(|_| {
                                OrgModeError::InvalidAgendaViewType(format!(
                                    "Invalid to date '{}', expected YYYY-MM-DD",
                                    to_str
                                ))
                            })?;

                        let from_datetime = Local
                            .from_local_datetime(
                                &from_date.and_hms_opt(0, 0, 0).unwrap_or_default(),
                            )
                            .single()
                            .ok_or_else(|| {
                                OrgModeError::InvalidAgendaViewType(format!(
                                    "Could not convert from date '{}' to local timezone",
                                    from_str
                                ))
                            })?;
                        let to_datetime = Local
                            .from_local_datetime(
                                &to_date.and_hms_opt(23, 59, 59).unwrap_or_default(),
                            )
                            .single()
                            .ok_or_else(|| {
                                OrgModeError::InvalidAgendaViewType(format!(
                                    "Could not convert to date '{}' to local timezone",
                                    to_str
                                ))
                            })?;

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

use std::collections::HashSet;
use std::path::Path;
use std::{convert::TryFrom, fs, io, path::PathBuf};

use chrono::{DateTime, Datelike, Days, Duration, Local, Months, NaiveDate, TimeZone};
use globset::{Glob, GlobSetBuilder};
use ignore::{Walk, WalkBuilder};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as NucleoConfig, Matcher};
use orgize::ast::{Headline, PropertyDrawer, Timestamp};
use orgize::export::{Container, Event, from_fn, from_fn_with_ctx};
use orgize::{Org, ParseConfig};
use rowan::ast::AstNode;
use serde::{Deserialize, Serialize};
use shellexpand::tilde;

use crate::OrgModeError;
use crate::config::{OrgConfig, load_org_config};
use crate::utils::tags_match;

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
            org_root.join(&path).to_str().unwrap_or(&path).to_string()
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
    ) -> Result<AgendaView, OrgModeError> {
        let items = self
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
                && let Some(date) = Local.from_local_datetime(&date).single()
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

    /// Convert a Timestamp to NaiveDateTime
    ///
    /// # Arguments
    /// * `ts` - The timestamp to convert
    /// * `use_start` - If true, use start date/time; if false, use end date/time
    fn timestamp_to_chrono(ts: &Timestamp, use_start: bool) -> Option<chrono::NaiveDateTime> {
        let (year, month, day, hour, minute) = if use_start {
            (
                ts.year_start()?,
                ts.month_start()?,
                ts.day_start()?,
                ts.hour_start(),
                ts.minute_start(),
            )
        } else {
            (
                ts.year_end()?,
                ts.month_end()?,
                ts.day_end()?,
                ts.hour_end(),
                ts.minute_end(),
            )
        };

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

    pub fn start_to_chrono(ts: &Timestamp) -> Option<chrono::NaiveDateTime> {
        Self::timestamp_to_chrono(ts, true)
    }

    pub fn end_to_chrono(ts: &Timestamp) -> Option<chrono::NaiveDateTime> {
        Self::timestamp_to_chrono(ts, false)
    }
}

// DateTime helper functions
impl OrgMode {
    /// Convert a DateTime to start of day (00:00:00)
    fn to_start_of_day(date: DateTime<Local>) -> DateTime<Local> {
        date.date_naive()
            .and_hms_opt(0, 0, 0)
            .and_then(|dt| Local.from_local_datetime(&dt).single())
            .unwrap_or(date)
    }

    /// Convert a DateTime to end of day (23:59:59.999)
    fn to_end_of_day(date: DateTime<Local>) -> DateTime<Local> {
        date.date_naive()
            .succ_opt()
            .and_then(|tomorrow| tomorrow.and_hms_opt(0, 0, 0))
            .and_then(|dt| Local.from_local_datetime(&dt).single())
            .map(|tomorrow| tomorrow - Duration::milliseconds(1))
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
            .and_then(|dt| Local.from_local_datetime(&dt).single())
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
                .with_month(next_month)
                .unwrap()
                .with_day(1)
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
            orgize::ast::TimeUnit::Hour => date + Duration::hours(value as i64),
            orgize::ast::TimeUnit::Day => date.checked_add_days(Days::new(value)).unwrap(),
            orgize::ast::TimeUnit::Week => date.checked_add_days(Days::new(value * 7)).unwrap(),
            orgize::ast::TimeUnit::Month => {
                date.checked_add_months(Months::new(value as u32)).unwrap()
            }
            orgize::ast::TimeUnit::Year => date
                .checked_add_months(Months::new(value as u32 * 12))
                .unwrap(),
        }
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
                let now = Local::now();
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
                let now = Local::now();
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

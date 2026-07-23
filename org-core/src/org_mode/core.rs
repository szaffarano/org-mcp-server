use std::collections::HashSet;
use std::path::Path;
use std::{fs, io, path::PathBuf};

use chrono::{Local, TimeZone};
use globset::{Glob, GlobSetBuilder};
use ignore::{Walk, WalkBuilder};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as NucleoConfig, Matcher};
use orgize::ast::{Headline, PropertyDrawer, Timestamp};
use orgize::export::{Container, Event, from_fn, from_fn_with_ctx};
use orgize::{Org, ParseConfig};
use rowan::ast::AstNode;

use crate::OrgModeError;
use crate::config::OrgConfig;
use crate::org_mode::{
    AgendaItem, AgendaView, AgendaViewType, OrgMode, Position, Priority, SearchResult, TreeNode,
};
use crate::utils::tags_match;

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

impl OrgMode {
    pub fn new(config: OrgConfig) -> Result<Self, OrgModeError> {
        let config = config.validate()?;
        Ok(OrgMode { config })
    }

    pub fn with_defaults() -> Result<Self, OrgModeError> {
        Self::new(crate::config::load_org_config(None, None)?)
    }

    pub fn config(&self) -> &OrgConfig {
        &self.config
    }

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
                Err(_) => continue,
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

        let path = shellexpand::tilde(&path).into_owned();
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
            days_overdue: None,
        }
    }
}

use std::{fs, io, path::PathBuf};

use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as NucleoConfig, Matcher};
use orgize::Org;
use orgize::ast::PropertyDrawer;
use orgize::export::{Container, Event, from_fn, from_fn_with_ctx};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

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
        WalkDir::new(&self.config.org_directory)
            .into_iter()
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
                Err(e) => Some(Err(OrgModeError::WalkDirError(e))),
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
}

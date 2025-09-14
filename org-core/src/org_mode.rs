use std::{fs, io, path::PathBuf};

use orgize::Org;
use orgize::ast::PropertyDrawer;
use orgize::export::{Container, Event, from_fn, from_fn_with_ctx};
use shellexpand::tilde;
use walkdir::WalkDir;

use crate::OrgModeError;

#[cfg(test)]
#[path = "org_mode_tests.rs"]
mod org_mode_tests;

#[derive(Debug)]
pub struct OrgMode {
    org_dir: PathBuf,
    #[allow(unused)]
    org_agenda_files: Vec<String>,
    #[allow(unused)]
    org_agenda_text_search_extra_files: Vec<String>,
    #[allow(unused)]
    org_default_notes_file: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TreeNode {
    pub label: String,
    pub level: usize,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new(label: String) -> Self {
        Self {
            label,
            level: 0,
            children: Vec::new(),
        }
    }

    pub fn new_with_level(label: String, level: usize) -> Self {
        Self {
            label,
            level,
            children: Vec::new(),
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
    pub fn new(org_dir: &str) -> Result<Self, OrgModeError> {
        let expanded_dir = tilde(org_dir);
        let base_path = PathBuf::from(expanded_dir.as_ref());

        if !base_path.exists() {
            return Err(OrgModeError::InvalidDirectory(format!(
                "Directory does not exist: {expanded_dir}"
            )));
        }

        if !base_path.is_dir() {
            return Err(OrgModeError::InvalidDirectory(format!(
                "Path is not a directory: {expanded_dir}"
            )));
        }

        match fs::read_dir(&base_path) {
            Ok(_) => {}
            Err(e) => {
                if e.kind() == io::ErrorKind::PermissionDenied {
                    return Err(OrgModeError::InvalidDirectory(format!(
                        "Permission denied accessing directory: {expanded_dir}"
                    )));
                }
                return Err(OrgModeError::IoError(e));
            }
        }

        Ok(OrgMode {
            org_dir: base_path,
            org_agenda_files: vec![String::from("agenda.org")],
            org_default_notes_file: String::from("notes.org"),
            org_agenda_text_search_extra_files: vec![],
        })
    }

    pub fn with_defaults() -> Result<Self, OrgModeError> {
        Self::new("~/org/")
    }
}

impl OrgMode {
    pub fn list_files(&self) -> Result<Vec<String>, OrgModeError> {
        WalkDir::new(&self.org_dir)
            .into_iter()
            .filter_map(|entry| match entry {
                Ok(dir_entry) => {
                    let path = dir_entry.path();

                    if path.is_file()
                        && let Some(extension) = path.extension()
                        && extension == "org"
                        && let Ok(relative_path) = path.strip_prefix(&self.org_dir)
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
    }

    pub fn read_file(&self, path: &str) -> Result<String, OrgModeError> {
        let path_buf = PathBuf::from(path);
        let full_path = if path_buf.is_absolute() {
            path_buf
        } else {
            self.org_dir.join(path)
        };

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
                let title = h.title_raw();
                let node = TreeNode::new_with_level(title, level);

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
        let files = self.list_files()?;

        let found = files.iter().find_map(|path| {
            self.read_file(path)
                .map(|content| self.search_id(content, id))
                .unwrap_or_default()
        });

        found.ok_or_else(|| OrgModeError::InvalidElementId(id.into()))
    }

    pub fn search_id(&self, content: String, id: &str) -> Option<String> {
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
}

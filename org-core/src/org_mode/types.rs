use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::config::OrgConfig;

#[derive(Debug)]
pub struct OrgMode {
    pub(crate) config: OrgConfig,
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
    pub todo_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_overdue: Option<i64>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyPair {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureEntry {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todo_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_heading: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Vec<PropertyPair>>,
    #[serde(default)]
    pub datetree: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datetree_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResult {
    pub file_path: String,
    pub level: usize,
    pub heading_line: String,
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

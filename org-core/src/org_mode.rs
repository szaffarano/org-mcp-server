use std::{fs, io, path::PathBuf};

use orgize::Org;
use orgize::ast::PropertyDrawer;
use orgize::export::{Container, Event, from_fn, from_fn_with_ctx};
use shellexpand::tilde;
use walkdir::WalkDir;

use crate::OrgModeError;

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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_org_mode() -> OrgMode {
        let test_dir = "tests/fixtures";
        OrgMode::new(test_dir).expect("Failed to create test OrgMode")
    }

    #[test]
    fn test_get_outline_simple() {
        let org_mode = create_test_org_mode();
        let tree = org_mode
            .get_outline("simple.org")
            .expect("Failed to get outline");

        assert_eq!(tree.label, "Document");
        assert_eq!(tree.children.len(), 3);
        assert_eq!(tree.children[0].label, "First Heading");
        assert_eq!(tree.children[0].level, 1);
        assert_eq!(tree.children[1].label, "Second Heading");
        assert_eq!(tree.children[1].level, 1);
        assert_eq!(tree.children[2].label, "Third Heading");
        assert_eq!(tree.children[2].level, 1);
    }

    #[test]
    fn test_get_outline_nested() {
        let org_mode = create_test_org_mode();
        let tree = org_mode
            .get_outline("sample.org")
            .expect("Failed to get outline");

        assert_eq!(tree.label, "Document");
        assert_eq!(tree.children.len(), 2);
        assert_eq!(tree.children[0].label, "Top Level Heading");
        assert_eq!(tree.children[0].level, 1);
        assert_eq!(tree.children[0].children.len(), 2);
        assert_eq!(tree.children[0].children[0].label, "Second Level A");
        assert_eq!(tree.children[0].children[0].level, 2);
        assert_eq!(tree.children[0].children[0].children.len(), 1);
        assert_eq!(
            tree.children[0].children[0].children[0].label,
            "Third Level"
        );
        assert_eq!(tree.children[0].children[0].children[0].level, 3);
    }

    #[test]
    fn test_get_outline_nonexistent_file() {
        let org_mode = create_test_org_mode();
        let result = org_mode.get_outline("nonexistent.org");
        assert!(result.is_err());
    }

    #[test]
    fn test_tree_node_creation() {
        let node = TreeNode::new("Test Heading".to_string());
        assert_eq!(node.label, "Test Heading");
        assert_eq!(node.level, 0);
        assert!(node.children.is_empty());

        let node_with_level = TreeNode::new_with_level("Test Heading".to_string(), 2);
        assert_eq!(node_with_level.label, "Test Heading");
        assert_eq!(node_with_level.level, 2);
        assert!(node_with_level.children.is_empty());
    }

    #[test]
    fn test_tree_node_indented_string() {
        let mut parent = TreeNode::new_with_level("Parent".to_string(), 1);
        let child = TreeNode::new_with_level("Child".to_string(), 2);
        parent.children.push(child);

        let result = parent.to_indented_string(0);
        let expected = "* Parent\n  ** Child\n";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_tree_node_serialization() {
        let mut root = TreeNode::new("Document".to_string());
        let mut parent = TreeNode::new_with_level("Parent".to_string(), 1);
        let child = TreeNode::new_with_level("Child".to_string(), 2);
        parent.children.push(child);
        root.children.push(parent);

        let json = serde_json::to_string(&root).expect("Failed to serialize");
        let deserialized: TreeNode = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.label, "Document");
        assert_eq!(deserialized.level, 0);
        assert_eq!(deserialized.children.len(), 1);
        assert_eq!(deserialized.children[0].label, "Parent");
        assert_eq!(deserialized.children[0].level, 1);
        assert_eq!(deserialized.children[0].children.len(), 1);
        assert_eq!(deserialized.children[0].children[0].label, "Child");
        assert_eq!(deserialized.children[0].children[0].level, 2);
    }

    #[test]
    fn test_get_heading_simple() {
        let org_mode = create_test_org_mode();
        let result = org_mode
            .get_heading("simple.org", "First Heading")
            .expect("Failed to get heading");

        assert!(result.contains("* First Heading"));
        assert!(result.contains("This is content under the first heading."));
    }

    #[test]
    fn test_get_heading_nested_path() {
        let org_mode = create_test_org_mode();
        let result = org_mode
            .get_heading("nested.org", "Project Planning/Phase 1/Setup Tasks")
            .expect("Failed to get nested heading");

        assert!(result.contains("*** Setup Tasks"));
        assert!(result.contains("Install dependencies"));
        assert!(result.contains("Configure environment"));
    }

    #[test]
    fn test_get_heading_nonexistent() {
        let org_mode = create_test_org_mode();
        let result = org_mode.get_heading("simple.org", "Nonexistent Heading");

        match result {
            Err(OrgModeError::InvalidHeadingPath(path)) => {
                assert_eq!(path, "Nonexistent Heading");
            }
            _ => panic!("Expected InvalidHeadingPath error"),
        }
    }

    #[test]
    fn test_get_heading_invalid_path() {
        let org_mode = create_test_org_mode();
        let result = org_mode.get_heading("nested.org", "Project Planning/Nonexistent/Deep");

        match result {
            Err(OrgModeError::InvalidHeadingPath(path)) => {
                assert_eq!(path, "Project Planning/Nonexistent/Deep");
            }
            _ => panic!("Expected InvalidHeadingPath error"),
        }
    }

    #[test]
    fn test_get_heading_second_level() {
        let org_mode = create_test_org_mode();
        let result = org_mode
            .get_heading("nested.org", "Project Planning/Phase 2")
            .expect("Failed to get second level heading");

        assert!(result.contains("** Phase 2"));
        assert!(result.contains("Implementation phase."));
        assert!(result.contains("*** Development"));
        assert!(result.contains("*** Testing"));
    }

    #[test]
    fn test_get_element_by_id_simple_heading() {
        let org_mode = create_test_org_mode();
        let result = org_mode
            .get_element_by_id("simple-123")
            .expect("Failed to get element by simple ID");

        assert!(result.contains("* Heading with Simple ID"));
        assert!(result.contains(":ID: simple-123"));
        assert!(result.contains("This heading has a simple ID for testing."));
        assert!(result.contains("** Subheading"));
    }

    #[test]
    fn test_get_element_by_id_uuid_heading() {
        let org_mode = create_test_org_mode();
        let result = org_mode
            .get_element_by_id("550e8400-e29b-41d4-a716-446655440000")
            .expect("Failed to get element by UUID ID");

        assert!(result.contains("* Heading with UUID"));
        assert!(result.contains(":ID: 550e8400-e29b-41d4-a716-446655440000"));
        assert!(result.contains("This heading has a UUID-style ID."));
    }

    #[test]
    fn test_get_element_by_id_document_level() {
        let org_mode = create_test_org_mode();
        let result = org_mode
            .get_element_by_id("document-level-id-456")
            .expect("Failed to get document-level element by ID");

        assert!(result.contains(":ID: document-level-id-456"));
        assert!(result.contains(":TITLE: Document with ID"));
        assert!(result.contains("This is a document with a document-level ID property."));
    }

    #[test]
    fn test_get_element_by_id_nested_heading() {
        let org_mode = create_test_org_mode();
        let result = org_mode
            .get_element_by_id("nested-abc")
            .expect("Failed to get nested element by ID");

        assert!(result.contains("** Nested Heading with ID"));
        assert!(result.contains(":ID: nested-abc"));
        assert!(result.contains("This is a nested heading with an ID."));
    }

    #[test]
    fn test_get_element_by_id_with_multiple_properties() {
        let org_mode = create_test_org_mode();
        let result = org_mode
            .get_element_by_id("multi-prop-id")
            .expect("Failed to get element with multiple properties");

        assert!(result.contains("* Multiple Properties Heading"));
        assert!(result.contains(":ID: multi-prop-id"));
        assert!(result.contains(":CREATED: [2023-01-01 Mon]"));
        assert!(result.contains(":CATEGORY: test"));
        assert!(result.contains("This heading has multiple properties including an ID."));
    }

    #[test]
    fn test_get_element_by_id_nonexistent() {
        let org_mode = create_test_org_mode();
        let result = org_mode.get_element_by_id("nonexistent-id");

        match result {
            Err(OrgModeError::InvalidElementId(id)) => {
                assert_eq!(id, "nonexistent-id");
            }
            _ => panic!("Expected InvalidElementId error"),
        }
    }

    #[test]
    fn test_get_element_by_id_case_sensitive() {
        let org_mode = create_test_org_mode();

        let result = org_mode.get_element_by_id("CaseSensitiveID");
        assert!(result.is_ok());

        let result = org_mode.get_element_by_id("casesensitiveid");
        assert!(result.is_err());

        let result = org_mode.get_element_by_id("CASESENSITIVEID");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_element_by_id_special_characters() {
        let org_mode = create_test_org_mode();
        let result = org_mode
            .get_element_by_id("special-chars!@#$%")
            .expect("Failed to get element with special characters in ID");

        assert!(result.contains("* Heading with Special Characters"));
        assert!(result.contains(":ID: special-chars!@#$%"));
    }

    #[test]
    fn test_get_element_by_id_whitespace_handling() {
        let org_mode = create_test_org_mode();

        let result = org_mode.get_element_by_id("spaces-around");
        assert!(
            result.is_ok(),
            "Should find ID despite whitespace in org file"
        );
    }

    #[test]
    fn test_get_element_by_id_multi_file_search() {
        let org_mode = create_test_org_mode();

        let result_a = org_mode.get_element_by_id("project-alpha-001");
        assert!(result_a.is_ok());
        assert!(result_a.unwrap().contains("* Project Alpha"));

        let result_b = org_mode.get_element_by_id("project-beta-002");
        assert!(result_b.is_ok());
        assert!(result_b.unwrap().contains("* Project Beta"));
    }

    #[test]
    fn test_get_element_by_id_duplicate_id_precedence() {
        let org_mode = create_test_org_mode();

        let result = org_mode.get_element_by_id("shared-id-test");
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(content.contains("* Shared ID Test"));
        assert!(content.contains(":ID: shared-id-test"));
    }

    #[test]
    fn test_search_id_heading_found() {
        let org_mode = create_test_org_mode();
        let content = org_mode
            .read_file("with_ids.org")
            .expect("Failed to read test file");

        let result = org_mode.search_id(content, "simple-123");
        assert!(result.is_some());
        let found = result.unwrap();
        assert!(found.contains("* Heading with Simple ID"));
        assert!(found.contains(":ID: simple-123"));
    }

    #[test]
    fn test_search_id_document_found() {
        let org_mode = create_test_org_mode();
        let content = org_mode
            .read_file("doc_with_id.org")
            .expect("Failed to read test file");

        let result = org_mode.search_id(content, "document-level-id-456");
        assert!(result.is_some());
        let found = result.unwrap();
        assert!(found.contains(":ID: document-level-id-456"));
        assert!(found.contains(":TITLE: Document with ID"));
    }

    #[test]
    fn test_search_id_not_found() {
        let org_mode = create_test_org_mode();
        let content = org_mode
            .read_file("with_ids.org")
            .expect("Failed to read test file");

        let result = org_mode.search_id(content, "nonexistent-id");
        assert!(result.is_none());
    }

    #[test]
    fn test_search_id_empty_content() {
        let org_mode = create_test_org_mode();
        let result = org_mode.search_id(String::new(), "any-id");
        assert!(result.is_none());
    }

    #[test]
    fn test_search_id_case_sensitivity() {
        let org_mode = create_test_org_mode();
        let content = org_mode
            .read_file("edge_cases.org")
            .expect("Failed to read test file");

        let result = org_mode.search_id(content.clone(), "CaseSensitiveID");
        assert!(result.is_some());

        let result = org_mode.search_id(content, "casesensitiveid");
        assert!(result.is_none());
    }
}

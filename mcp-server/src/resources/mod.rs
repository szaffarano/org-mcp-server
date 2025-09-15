mod org_file;
mod org_file_list;
mod org_heading;
mod org_id;
mod org_outline;
mod utils;

#[cfg(test)]
mod resource_tests;

use rmcp::model::{
    AnnotateAble, Implementation, InitializeRequestParam, InitializeResult,
    ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParam, RawResource,
    RawResourceTemplate, ReadResourceRequestParam, ReadResourceResult,
};
use rmcp::service::RequestContext;
use rmcp::{
    ErrorData as McpError,
    model::{ServerCapabilities, ServerInfo},
    tool_handler,
};

use rmcp::{RoleServer, ServerHandler};
use serde_json::json;

use crate::core::OrgModeRouter;

pub enum OrgResource {
    OrgFiles,
    Org { path: String },
    OrgOutline { path: String },
    OrgHeading { path: String, heading: String },
    OrgId { id: String },
}

#[tool_handler]
impl ServerHandler for OrgModeRouter {
    fn get_info(&self) -> ServerInfo {
        const INSTRUCTIONS: &str = concat!(
            "This server provides org-mode tools and resources.\n\n",
            "Tools:\n",
            "- org-file-list\n",
            "Resources:\n",
            "- org:// (List all org-mode files in the configured directory tree)\n",
            "- org://{file} (Access the raw content of an allowed Org file)\n",
            "- org-outline://{file} (Get the hierarchical structure of an Org file)\n",
            "- org-heading://{file}#{heading} (Access the content of a specific headline by its path)\n",
            "- org-id://{uuid} (Access Org node content by its unique ID property)\n",
        );

        ServerInfo {
            instructions: Some(INSTRUCTIONS.into()),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_completions()
                .build(),
            server_info: Implementation::from_build_env(),
            ..Default::default()
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                RawResource {
                    uri: "org://".to_string(),
                    name: "org".to_string(),
                    description: Some(
                        "List all org-mode files in the configured directory tree".to_string(),
                    ),
                    size: None,
                    mime_type: Some("application/json".to_string()),
                }
                .no_annotation(),
            ],
            next_cursor: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: vec![
                RawResourceTemplate {
                    uri_template: "org://{file}".to_string(),
                    name: "org-file".to_string(),
                    description: Some(
                        "Access the raw content of an org-mode file by its path".to_string(),
                    ),
                    mime_type: Some("text/org".to_string()),
                }
                .no_annotation(),
                RawResourceTemplate {
                    uri_template: "org-outline://{file}".to_string(),
                    name: "org-outline-file".to_string(),
                    description: Some(
                        "Get the hierarchical outline structure of an org-mode file as JSON"
                            .to_string(),
                    ),
                    mime_type: Some("application/json".to_string()),
                }
                .no_annotation(),
                RawResourceTemplate {
                    uri_template: "org-heading://{file}#{heading}".to_string(),
                    name: "org-heading-file".to_string(),
                    description: Some(
                        "Access the content of a specific heading within an org-mode file"
                            .to_string(),
                    ),
                    mime_type: Some("text/org".to_string()),
                }
                .no_annotation(),
                RawResourceTemplate {
                    uri_template: "org-id://{id}".to_string(),
                    name: "org-element-by-id".to_string(),
                    description: Some(
                        "Access the content of any org-mode element by its unique ID property"
                            .to_string(),
                    ),
                    mime_type: Some("text/org".to_string()),
                }
                .no_annotation(),
            ],
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        match OrgModeRouter::parse_resource(uri.clone()) {
            Some(OrgResource::OrgFiles) => self.list_files(uri).await,
            Some(OrgResource::Org { path }) => self.read_file(uri, path).await,
            Some(OrgResource::OrgOutline { path }) => self.outline(uri, path).await,
            Some(OrgResource::OrgHeading { path, heading }) => {
                self.heading(uri, path, heading).await
            }
            Some(OrgResource::OrgId { id }) => self.id(uri, id).await,

            None => Err(McpError::resource_not_found(
                format!("Invalid resource URI format: {}", uri),
                Some(json!({"uri": uri})),
            )),
        }
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        Ok(self.get_info())
    }
}

impl OrgModeRouter {
    fn parse_resource(uri: String) -> Option<OrgResource> {
        let uri = Self::decode_uri_path(&uri);

        if uri == "org://" {
            Some(OrgResource::OrgFiles)
        } else if let Some(path) = uri.strip_prefix("org://")
            && !path.is_empty()
        {
            Some(OrgResource::Org {
                path: path.to_string(),
            })
        } else if let Some(id) = uri.strip_prefix("org-id://")
            && !id.is_empty()
        {
            Some(OrgResource::OrgId { id: id.to_string() })
        } else if let Some(path) = uri.strip_prefix("org-outline://")
            && !path.is_empty()
        {
            Some(OrgResource::OrgOutline {
                path: path.to_string(),
            })
        } else if let Some(remainder) = uri.strip_prefix("org-heading://")
            && !remainder.is_empty()
            && let Some((path, heading)) = remainder.split_once('#')
            && !path.is_empty()
            && !heading.is_empty()
        {
            Some(OrgResource::OrgHeading {
                path: path.to_string(),
                heading: heading.to_string(),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{core::OrgModeRouter, resources::OrgResource};

    #[test]
    fn test_org_files_list_resource() {
        let result = OrgModeRouter::parse_resource("org://".to_string());
        assert!(matches!(result, Some(OrgResource::OrgFiles)));
    }

    #[test]
    fn test_org_resource_parsing() {
        let cases = vec![
            ("org://simple.org", "simple.org"),
            ("org://path/to/file.org", "path/to/file.org"),
            (
                "org://deep/nested/path/document.org",
                "deep/nested/path/document.org",
            ),
            (
                "org://file_with_underscores.org",
                "file_with_underscores.org",
            ),
            ("org://file-with-dashes.org", "file-with-dashes.org"),
        ];

        for (uri, expected_path) in cases {
            let result = OrgModeRouter::parse_resource(uri.to_string());
            match result {
                Some(OrgResource::Org { path }) => {
                    assert_eq!(path, expected_path, "Failed for URI: {}", uri);
                }
                _ => {
                    unreachable!("Expected Org resource for URI: {}", uri);
                }
            }
        }
    }

    #[test]
    fn test_org_outline_resource_parsing() {
        let cases = vec![
            ("org-outline://simple.org", "simple.org"),
            ("org-outline://path/to/file.org", "path/to/file.org"),
            (
                "org-outline://deep/nested/path/document.org",
                "deep/nested/path/document.org",
            ),
        ];

        for (uri, expected_path) in cases {
            let result = OrgModeRouter::parse_resource(uri.to_string());
            match result {
                Some(OrgResource::OrgOutline { path }) => {
                    assert_eq!(path, expected_path, "Failed for URI: {}", uri);
                }
                _ => {
                    unreachable!("Expected OrgOutline resource for URI: {}", uri);
                }
            }
        }
    }

    #[test]
    fn test_org_heading_resource_parsing() {
        let cases = vec![
            (
                "org-heading://file.org#Introduction",
                "file.org",
                "Introduction",
            ),
            (
                "org-heading://path/to/file.org#My Heading",
                "path/to/file.org",
                "My Heading",
            ),
            (
                "org-heading://notes/tasks.org#Project Planning",
                "notes/tasks.org",
                "Project Planning",
            ),
            (
                "org-heading://complex/path#Heading with Multiple Words",
                "complex/path",
                "Heading with Multiple Words",
            ),
            (
                "org-heading://file.org#Section 1.2.3",
                "file.org",
                "Section 1.2.3",
            ),
        ];

        for (uri, expected_path, expected_heading) in cases {
            let result = OrgModeRouter::parse_resource(uri.to_string());
            match result {
                Some(OrgResource::OrgHeading { path, heading }) => {
                    assert_eq!(path, expected_path, "Path failed for URI: {}", uri);
                    assert_eq!(heading, expected_heading, "Heading failed for URI: {}", uri);
                }
                _ => {
                    unreachable!("Expected OrgHeading resource for URI: {}", uri);
                }
            }
        }
    }

    #[test]
    fn test_uri_decoding() {
        let cases = vec![
            ("path%2Fto%2Ffile.org", "path/to/file.org"),
            ("file%20with%20spaces.org", "file with spaces.org"),
            ("deep%2Fnested%2Fpath.org", "deep/nested/path.org"),
            (
                "path%2Fto%2Ffile.org%23Special%20Heading",
                "path/to/file.org#Special Heading",
            ),
        ];

        for (encoded_path, expected_decoded) in cases {
            let decoded = OrgModeRouter::decode_uri_path(encoded_path);
            assert_eq!(
                decoded, expected_decoded,
                "URI decoding failed for: {}",
                encoded_path
            );
        }
    }

    #[test]
    fn test_uri_decoding_in_parsing() {
        let result = OrgModeRouter::parse_resource("org://path%2Fto%2Ffile.org".to_string());
        match result {
            Some(OrgResource::Org { path }) => {
                assert_eq!(path, "path/to/file.org");
            }
            _ => {
                unreachable!("Failed to parse URL-encoded org URI");
            }
        }

        let result = OrgModeRouter::parse_resource(
            "org-heading://notes%2Ftasks.org%23Project%20Planning".to_string(),
        );
        match result {
            Some(OrgResource::OrgHeading { path, heading }) => {
                assert_eq!(path, "notes/tasks.org");
                assert_eq!(heading, "Project Planning");
            }
            _ => {
                unreachable!("Failed to parse URL-encoded org-heading URI");
            }
        }
    }

    #[test]
    fn test_invalid_uris() {
        let invalid_cases = vec![
            ("", "empty string"),
            ("invalid://path", "invalid scheme"),
            ("org-outline://", "empty outline path"),
            ("org-heading://", "empty heading URI"),
            ("org-heading://path", "missing heading separator"),
            ("org-heading://path#", "empty heading"),
            ("org-heading://#heading", "empty path"),
            ("random-string", "no scheme"),
            ("org", "incomplete scheme"),
            ("org:/", "incomplete scheme"),
        ];

        for (uri, description) in invalid_cases {
            let result = OrgModeRouter::parse_resource(uri.to_string());
            assert!(
                result.is_none(),
                "Expected None for {} (URI: '{}')",
                description,
                uri
            );
        }
    }

    #[test]
    fn test_boundary_cases() {
        assert!(matches!(
            OrgModeRouter::parse_resource("org://a".to_string()),
            Some(OrgResource::Org { path }) if path == "a"
        ));

        assert!(matches!(
            OrgModeRouter::parse_resource("org-heading://a#b".to_string()),
            Some(OrgResource::OrgHeading { path, heading }) if path == "a" && heading == "b"
        ));

        let result =
            OrgModeRouter::parse_resource("org-heading://file.org#Heading: With Colon".to_string());
        match result {
            Some(OrgResource::OrgHeading { path, heading }) => {
                assert_eq!(path, "file.org");
                assert_eq!(heading, "Heading: With Colon");
            }
            _ => {
                unreachable!("Failed to parse heading with special characters");
            }
        }

        let result =
            OrgModeRouter::parse_resource("org-heading://file.org#Section#Subsection".to_string());
        match result {
            Some(OrgResource::OrgHeading { path, heading }) => {
                assert_eq!(path, "file.org");
                assert_eq!(heading, "Section#Subsection");
            }
            _ => {
                unreachable!("Failed to parse heading with multiple # characters");
            }
        }
    }

    #[test]
    fn test_case_sensitivity() {
        let invalid_cases = vec![
            "ORG://path/to/file",
            "Org://path/to/file",
            "org-OUTLINE://path/to/file",
            "ORG-HEADING://path#heading",
        ];

        for uri in invalid_cases {
            let result = OrgModeRouter::parse_resource(uri.to_string());
            assert!(
                result.is_none(),
                "Expected case-sensitive scheme rejection for: {}",
                uri
            );
        }
    }
}

use org_core::OrgModeError;
use rmcp::{
    ErrorData as McpError,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ErrorCode},
    schemars, tool, tool_router,
};

use crate::core::OrgModeRouter;

#[derive(Debug, schemars::JsonSchema, serde::Deserialize)]
pub struct SearchRequest {
    #[schemars(description = "Search query string to find in org file content")]
    pub query: String,
    #[schemars(description = "Maximum number of search results to return (optional)")]
    pub limit: Option<usize>,
    #[schemars(description = "Maximum snippet size in characters (optional, default: 100)")]
    pub snippet_max_size: Option<usize>,
    #[schemars(
        description = "Filter results by tags (optional, matches any of the provided tags)"
    )]
    pub tags: Option<Vec<String>>,
}

#[tool_router(router = "tool_router_search", vis = "pub(crate)")]
impl OrgModeRouter {
    #[tool(
        name = "org-search",
        description = "Search for text content across all org files using fuzzy matching",
        annotations(title = "org-search tool")
    )]
    async fn tool_search(
        &self,
        Parameters(SearchRequest {
            query,
            limit,
            snippet_max_size,
            tags,
        }): Parameters<SearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let org_mode = self.org_mode.lock().await;

        let results = if tags.is_some() {
            org_mode.search_with_tags(&query, tags.as_deref(), limit, snippet_max_size)
        } else {
            org_mode.search(&query, limit, snippet_max_size)
        };

        match results {
            Ok(results) => match Content::json(results) {
                Ok(serialized) => Ok(CallToolResult::success(vec![serialized])),
                Err(e) => Err(McpError {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: format!("Failed to serialize search results: {e}").into(),
                    data: None,
                }),
            },
            Err(e) => {
                let error_code = match &e {
                    OrgModeError::InvalidDirectory(_) => ErrorCode::INVALID_PARAMS,
                    OrgModeError::WalkDirError(_) => ErrorCode::INTERNAL_ERROR,
                    OrgModeError::IoError(_) => ErrorCode::INTERNAL_ERROR,
                    _ => ErrorCode::INTERNAL_ERROR,
                };
                Err(McpError {
                    code: error_code,
                    message: format!("Failed to search: {e}").into(),
                    data: None,
                })
            }
        }
    }
}

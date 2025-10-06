use org_core::OrgModeError;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, Content, ErrorCode},
    tool, tool_router,
};

use crate::core::OrgModeRouter;

#[tool_router(router = "tool_router_list_files", vis = "pub(crate)")]
impl OrgModeRouter {
    #[tool(
        name = "org-file-list",
        description = "List all the org files defined in the org-mode configuration",
        annotations(title = "org-file-list tool")
    )]
    async fn tool_list_files(&self) -> Result<CallToolResult, McpError> {
        let org_mode = self.org_mode.lock().await;
        match org_mode.list_files() {
            Ok(files) => match Content::json(files) {
                Ok(serialized) => Ok(CallToolResult::success(vec![serialized])),
                Err(e) => Err(McpError {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: format!("Failed to serialize files: {e}").into(),
                    data: None,
                }),
            },
            Err(e) => {
                let error_code = match &e {
                    OrgModeError::InvalidDirectory(_) => ErrorCode::INVALID_PARAMS,
                    OrgModeError::WalkDirError(_) => ErrorCode::INTERNAL_ERROR,
                    _ => ErrorCode::INTERNAL_ERROR,
                };
                Err(McpError {
                    code: error_code,
                    message: format!("Failed to list files: {e}").into(),
                    data: None,
                })
            }
        }
    }
}

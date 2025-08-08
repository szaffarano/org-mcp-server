use rmcp::model::{ReadResourceResult, ResourceContents};
use rmcp::{ErrorData as McpError, model::ErrorCode};

use serde_json::json;

use crate::core::OrgModeRouter;

impl OrgModeRouter {
    pub(crate) async fn list_files(&self, uri: String) -> Result<ReadResourceResult, McpError> {
        let org_mode = self.org_mode.lock().await;
        match org_mode.list_files() {
            Ok(files) => Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(
                    serde_json::to_string(&files).unwrap_or_default(),
                    uri,
                )],
            }),
            Err(e) => Err(McpError {
                code: ErrorCode::INTERNAL_ERROR,
                message: format!("Failed to read org file: {}", e).into(),
                data: Some(json!({ "uri": uri})),
            }),
        }
    }
}

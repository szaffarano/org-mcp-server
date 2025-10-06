use rmcp::model::{ReadResourceResult, ResourceContents};
use rmcp::{ErrorData as McpError, model::ErrorCode};

use serde_json::json;

use crate::core::OrgModeRouter;

impl OrgModeRouter {
    pub(crate) async fn read_file(
        &self,
        uri: String,
        path: String,
    ) -> Result<ReadResourceResult, McpError> {
        let org_mode = self.org_mode.lock().await;
        match org_mode.read_file(&path) {
            Ok(content) => Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(content, uri)],
            }),
            Err(e) => Err(McpError {
                code: ErrorCode::INTERNAL_ERROR,
                message: format!("Failed to read org file '{}': {}", path, e).into(),
                data: Some(json!({"path": path, "uri": uri})),
            }),
        }
    }
}

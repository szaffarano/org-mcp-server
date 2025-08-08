use rmcp::model::{ReadResourceResult, ResourceContents};
use rmcp::{ErrorData as McpError, model::ErrorCode};

use serde_json::json;

use crate::core::OrgModeRouter;

impl OrgModeRouter {
    pub(crate) async fn outline(
        &self,
        uri: String,
        path: String,
    ) -> Result<ReadResourceResult, McpError> {
        let org_mode = self.org_mode.lock().await;
        match org_mode.get_outline(&path) {
            Ok(tree) => Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri,
                    mime_type: Some("json".into()),
                    text: serde_json::to_string(&tree).unwrap_or_default(),
                    meta: None,
                }],
            }),
            Err(e) => Err(McpError {
                code: ErrorCode::INTERNAL_ERROR,
                message: format!("Failed to get outline for '{}': {}", path, e).into(),
                data: Some(json!({"path": path, "uri": uri})),
            }),
        }
    }
}

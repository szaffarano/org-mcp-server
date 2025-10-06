use rmcp::ErrorData as McpError;
use rmcp::model::{ErrorCode, ReadResourceResult, ResourceContents};
use serde_json::json;

use crate::core::OrgModeRouter;

impl OrgModeRouter {
    #[allow(unused)]
    pub(crate) async fn id(&self, uri: String, id: String) -> Result<ReadResourceResult, McpError> {
        let org_mode = self.org_mode.lock().await;
        match org_mode.get_element_by_id(&id) {
            Ok(content) => Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(content, uri)],
            }),
            Err(e) => Err(McpError {
                code: ErrorCode::INTERNAL_ERROR,
                message: format!("Failed to get element by id '{}': {}", id, e).into(),
                data: Some(json!({"id": id, "uri": uri})),
            }),
        }
    }
}

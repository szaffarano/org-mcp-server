use org_core::org_mode::AgendaViewType;
use rmcp::model::{ReadResourceResult, ResourceContents};
use rmcp::{ErrorData as McpError, model::ErrorCode};

use serde_json::json;

use crate::core::OrgModeRouter;

impl OrgModeRouter {
    pub(crate) async fn read_agenda(
        &self,
        uri: String,
        agenda_vew_type: AgendaViewType,
    ) -> Result<ReadResourceResult, McpError> {
        let org_mode = self.org_mode.lock().await;

        let result = org_mode
            .get_agenda_view(agenda_vew_type, None, None)
            .map(|tasks| json!(tasks).to_string());

        match result {
            Ok(content) => Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(content, uri)],
            }),
            Err(e) => Err(McpError {
                code: ErrorCode::INTERNAL_ERROR,
                message: format!("Failed to read agenda: {e}").into(),
                data: Some(json!({ "uri": uri })),
            }),
        }
    }
}

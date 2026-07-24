use org_core::{ClearField, OrgModeError};
use rmcp::{
    ErrorData as McpError,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock, ErrorCode},
    schemars, tool, tool_router,
};

use crate::core::OrgModeRouter;

#[derive(Debug, schemars::JsonSchema, serde::Deserialize)]
pub struct UpdateTodoRequest {
    #[schemars(
        description = "Org ID property of the target heading. Wins when file/heading_path are also given."
    )]
    pub id: Option<String>,
    #[schemars(
        description = "Relative file path within org directory. Required together with heading_path."
    )]
    pub file: Option<String>,
    #[schemars(
        description = "Slash-separated heading path (e.g., 'Projects/Work'). Required together with file."
    )]
    pub heading_path: Option<String>,
    #[schemars(
        description = "New TODO state keyword. Must match a configured keyword in org_todo_keywords."
    )]
    pub todo_state: Option<String>,
    #[schemars(description = "New priority level: A, B, or C")]
    pub priority: Option<String>,
    #[schemars(
        description = "Replace the tag list wholesale. Each tag must match ^[A-Za-z0-9_@]+$."
    )]
    pub tags: Option<Vec<String>>,
    #[schemars(
        description = "New SCHEDULED active timestamp (ISO 'YYYY-MM-DD' or 'YYYY-MM-DD HH:MM', optional repeater +N|++N|.+N{h|d|w|m|y} and warning -N{h|d|w|m|y})."
    )]
    pub scheduled: Option<String>,
    #[schemars(description = "New DEADLINE active timestamp; same grammar as scheduled.")]
    pub deadline: Option<String>,
    #[schemars(
        description = "CLOSED inactive timestamp. Usually auto-managed: stamped on done transitions, removed on reactivation."
    )]
    pub closed: Option<String>,
    #[schemars(
        description = "Fields to remove: 'todo_state', 'priority', 'tags', 'scheduled', 'deadline', 'closed'."
    )]
    pub clear: Option<Vec<String>>,
}

#[tool_router(router = "tool_router_update_todo", vis = "pub(crate)")]
impl OrgModeRouter {
    #[tool(
        name = "org-update-todo",
        description = "Update the TODO state and planning metadata of an existing heading. Target by org ID property or by file + slash heading path. Set todo_state, priority, tags (replaced wholesale), SCHEDULED/DEADLINE/CLOSED timestamps, and/or remove fields via the clear list. CLOSED is auto-managed on done/active transitions unless org_auto_closed_timestamp is disabled.",
        annotations(title = "org-update-todo tool")
    )]
    async fn tool_update_todo(
        &self,
        Parameters(UpdateTodoRequest {
            id,
            file,
            heading_path,
            todo_state,
            priority,
            tags,
            scheduled,
            deadline,
            closed,
            clear,
        }): Parameters<UpdateTodoRequest>,
    ) -> Result<CallToolResult, McpError> {
        let clear: Vec<ClearField> = match clear {
            Some(values) => values
                .iter()
                .map(|v| v.parse::<ClearField>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| McpError {
                    code: ErrorCode::INVALID_PARAMS,
                    message: format!("Invalid clear value: {e}").into(),
                    data: None,
                })?,
            None => Vec::new(),
        };

        let entry = org_core::UpdateEntry {
            id,
            file,
            heading_path,
            todo_state,
            priority,
            tags,
            scheduled,
            deadline,
            closed,
            clear,
        };

        let org_mode = self.org_mode.lock().await;

        match org_mode.update_todo(entry) {
            Ok(result) => match ContentBlock::json(&result) {
                Ok(serialized) => Ok(CallToolResult::success(vec![serialized])),
                Err(e) => Err(McpError {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: format!("Failed to serialize update result: {e}").into(),
                    data: None,
                }),
            },
            Err(e) => {
                let error_code = match &e {
                    OrgModeError::InvalidTodoKeyword(_)
                    | OrgModeError::InvalidPriority(_)
                    | OrgModeError::InvalidHeadingPath(_)
                    | OrgModeError::InvalidTag(_)
                    | OrgModeError::InvalidDirectory(_)
                    | OrgModeError::InvalidTimestamp { .. }
                    | OrgModeError::HeadingNotFound(_)
                    | OrgModeError::AmbiguousTarget(_)
                    | OrgModeError::InvalidUpdate(_) => ErrorCode::INVALID_PARAMS,
                    _ => ErrorCode::INTERNAL_ERROR,
                };
                Err(McpError {
                    code: error_code,
                    message: format!("Failed to update todo: {e}").into(),
                    data: None,
                })
            }
        }
    }
}

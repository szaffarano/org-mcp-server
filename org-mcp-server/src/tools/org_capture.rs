use org_core::OrgModeError;
use rmcp::{
    ErrorData as McpError,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ErrorCode},
    schemars, tool, tool_router,
};

use crate::core::OrgModeRouter;

#[derive(Debug, schemars::JsonSchema, serde::Deserialize)]
pub struct PropertyPairRequest {
    pub key: String,
    pub value: String,
}

impl From<PropertyPairRequest> for org_core::PropertyPair {
    fn from(p: PropertyPairRequest) -> Self {
        org_core::PropertyPair {
            key: p.key,
            value: p.value,
        }
    }
}

#[derive(Debug, schemars::JsonSchema, serde::Deserialize)]
pub struct CaptureRequest {
    #[schemars(
        description = "Title for the new heading (required). Must be non-empty after trimming and contain no newline or carriage return characters."
    )]
    pub title: String,
    #[schemars(
        description = "Heading level (1..=19). If omitted, defaults to parent_level+1 when target_heading is set, else 1."
    )]
    pub level: Option<usize>,
    #[schemars(
        description = "TODO state keyword (e.g., 'TODO', 'DONE'). Must match a configured keyword in org_todo_keywords."
    )]
    pub todo_state: Option<String>,
    #[schemars(
        description = "Tags. Each must match the org-mode tag character set: ^[A-Za-z0-9_@]+$."
    )]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "Priority level: A, B, or C")]
    pub priority: Option<String>,
    #[schemars(description = "Body content placed beneath the heading.")]
    pub body: Option<String>,
    #[schemars(
        description = "Relative file path within org directory. Uses the configured default notes file if omitted."
    )]
    pub file: Option<String>,
    #[schemars(
        description = "Slash-separated heading path to insert under. Each segment must be a direct child of the previous match. Missing intermediate headings are created."
    )]
    pub target_heading: Option<String>,
    #[schemars(
        description = "SCHEDULED active timestamp (ISO 'YYYY-MM-DD' or 'YYYY-MM-DD HH:MM', optional repeater +N|++N|.+N{h|d|w|m|y} and warning -N{h|d|w|m|y})."
    )]
    pub scheduled: Option<String>,
    #[schemars(description = "DEADLINE active timestamp; same grammar as scheduled.")]
    pub deadline: Option<String>,
    #[schemars(description = "CLOSED inactive timestamp; same grammar as scheduled.")]
    pub closed: Option<String>,
    #[schemars(description = "Property drawer entries written in given order. Each {key, value}.")]
    pub properties: Option<Vec<PropertyPairRequest>>,
    #[schemars(
        description = "When true, expand target_heading with a Year/Month/Day datetree before resolution so the entry lands under the chosen day's leaf."
    )]
    #[serde(default)]
    pub datetree: bool,
    #[schemars(
        description = "Optional override for the datetree day (YYYY-MM-DD). Defaults to today when datetree=true."
    )]
    pub datetree_date: Option<String>,
}

#[tool_router(router = "tool_router_capture", vis = "pub(crate)")]
impl OrgModeRouter {
    #[tool(
        name = "org-capture",
        description = "Append a new heading to an org file. Supports TODO state, priority, tags, body, SCHEDULED/DEADLINE/CLOSED timestamps (with optional repeater/warning), property drawer entries, and Year/Month/Day datetree expansion. Can target a specific heading to insert under, or append to end of file.",
        annotations(title = "org-capture tool")
    )]
    async fn tool_capture(
        &self,
        Parameters(CaptureRequest {
            title,
            level,
            todo_state,
            tags,
            priority,
            body,
            file,
            target_heading,
            scheduled,
            deadline,
            closed,
            properties,
            datetree,
            datetree_date,
        }): Parameters<CaptureRequest>,
    ) -> Result<CallToolResult, McpError> {
        let org_mode = self.org_mode.lock().await;

        let entry = org_core::CaptureEntry {
            title,
            level,
            todo_state,
            tags,
            priority,
            body,
            file,
            target_heading,
            scheduled,
            deadline,
            closed,
            properties: properties.map(|v| v.into_iter().map(Into::into).collect()),
            datetree,
            datetree_date,
        };

        match org_mode.capture_append(entry) {
            Ok(result) => match Content::json(&result) {
                Ok(serialized) => Ok(CallToolResult::success(vec![serialized])),
                Err(e) => Err(McpError {
                    code: ErrorCode::INTERNAL_ERROR,
                    message: format!("Failed to serialize capture result: {e}").into(),
                    data: None,
                }),
            },
            Err(e) => {
                let error_code = match &e {
                    OrgModeError::InvalidTodoKeyword(_)
                    | OrgModeError::InvalidPriority(_)
                    | OrgModeError::InvalidHeadingPath(_)
                    | OrgModeError::InvalidTitle(_)
                    | OrgModeError::InvalidLevel(_)
                    | OrgModeError::InvalidTag(_)
                    | OrgModeError::InvalidDirectory(_)
                    | OrgModeError::InvalidTimestamp { .. }
                    | OrgModeError::InvalidPropertyKey(_)
                    | OrgModeError::InvalidPropertyValue { .. }
                    | OrgModeError::DuplicatePropertyKey(_)
                    | OrgModeError::InvalidDatetreeDate(_)
                    | OrgModeError::DatetreeDateWithoutFlag => ErrorCode::INVALID_PARAMS,
                    _ => ErrorCode::INTERNAL_ERROR,
                };
                Err(McpError {
                    code: error_code,
                    message: format!("Failed to capture: {e}").into(),
                    data: None,
                })
            }
        }
    }
}

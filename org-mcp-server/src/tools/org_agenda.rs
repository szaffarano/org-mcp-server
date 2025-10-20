use chrono::{Local, NaiveDate, TimeZone};
use org_core::{OrgModeError, Priority, org_mode::AgendaViewType};
use rmcp::{
    ErrorData as McpError,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ErrorCode},
    schemars, tool, tool_router,
};

use crate::core::OrgModeRouter;

#[derive(Debug, schemars::JsonSchema, serde::Deserialize)]
pub struct AgendaRequest {
    #[schemars(
        description = "Start date for agenda view in ISO 8601 format (YYYY-MM-DD, optional)"
    )]
    pub start_date: Option<String>,
    #[schemars(description = "End date for agenda view in ISO 8601 format (YYYY-MM-DD, optional)")]
    pub end_date: Option<String>,
    #[schemars(
        description = "Filter by TODO states (optional, e.g., ['TODO', 'DONE', 'IN_PROGRESS'])"
    )]
    pub todo_states: Option<Vec<String>>,
    #[schemars(
        description = "Filter results by tags (optional, matches any of the provided tags)"
    )]
    pub tags: Option<Vec<String>>,
    #[schemars(description = "Filter by priority level (optional: A, B, or C)")]
    pub priority: Option<String>,
    #[schemars(description = "Maximum number of items to return (optional)")]
    pub limit: Option<usize>,
    #[schemars(
        description = "View mode: 'list' for all tasks, 'view' for date-organized agenda (default: 'list')"
    )]
    pub mode: Option<String>,
}

#[tool_router(router = "tool_router_agenda", vis = "pub(crate)")]
impl OrgModeRouter {
    #[tool(
        name = "org-agenda",
        description = "Query agenda items (TODO/DONE tasks) with support for filtering by dates, states, tags, and priorities. Use 'list' mode to get all tasks, or 'view' mode for calendar-like agenda organized by scheduled/deadline dates.",
        annotations(title = "org-agenda tool")
    )]
    async fn tool_agenda(
        &self,
        Parameters(AgendaRequest {
            start_date,
            end_date,
            todo_states,
            tags,
            priority,
            limit,
            mode,
        }): Parameters<AgendaRequest>,
    ) -> Result<CallToolResult, McpError> {
        let org_mode = self.org_mode.lock().await;

        let mode_str = mode.as_deref().unwrap_or("list");

        let priority_filter = if let Some(ref p) = priority {
            match p.to_uppercase().as_str() {
                "A" => Some(Priority::A),
                "B" => Some(Priority::B),
                "C" => Some(Priority::C),
                _ => {
                    return Err(McpError {
                        code: ErrorCode::INVALID_PARAMS,
                        message: format!("Invalid priority '{p}'. Must be A, B, or C.").into(),
                        data: None,
                    });
                }
            }
        } else {
            None
        };

        match mode_str {
            "list" => {
                let tasks = org_mode.list_tasks(
                    todo_states.as_deref(),
                    tags.as_deref(),
                    priority_filter,
                    limit,
                );

                match tasks {
                    Ok(tasks) => match Content::json(tasks) {
                        Ok(serialized) => Ok(CallToolResult::success(vec![serialized])),
                        Err(e) => Err(McpError {
                            code: ErrorCode::INTERNAL_ERROR,
                            message: format!("Failed to serialize tasks: {e}").into(),
                            data: None,
                        }),
                    },
                    Err(e) => Err(Self::map_org_error(e)),
                }
            }
            "view" => {
                let agenda_view_type = match (start_date, end_date) {
                    (Some(start), Some(end)) => {
                        let from_result = NaiveDate::parse_from_str(&start, "%Y-%m-%d");
                        let to_result = NaiveDate::parse_from_str(&end, "%Y-%m-%d");

                        // TODO: improve error handling
                        match (from_result, to_result) {
                            (Ok(from_date), Ok(to_date)) => {
                                let from_datetime = Local
                                    .from_local_datetime(
                                        &from_date.and_hms_opt(0, 0, 0).unwrap_or_default(),
                                    )
                                    .single()
                                    .unwrap_or_else(Local::now);
                                let to_datetime = Local
                                    .from_local_datetime(
                                        &to_date.and_hms_opt(23, 59, 59).unwrap_or_default(),
                                    )
                                    .single()
                                    .unwrap_or_else(Local::now);

                                AgendaViewType::Custom {
                                    from: from_datetime,
                                    to: to_datetime,
                                }
                            }
                            _ => AgendaViewType::default(),
                        }
                    }
                    _ => AgendaViewType::default(),
                };

                let view = org_mode.get_agenda_view(
                    agenda_view_type,
                    todo_states.as_deref(),
                    tags.as_deref(),
                );

                match view {
                    Ok(view) => match Content::json(view) {
                        Ok(serialized) => Ok(CallToolResult::success(vec![serialized])),
                        Err(e) => Err(McpError {
                            code: ErrorCode::INTERNAL_ERROR,
                            message: format!("Failed to serialize agenda view: {e}").into(),
                            data: None,
                        }),
                    },
                    Err(e) => Err(Self::map_org_error(e)),
                }
            }
            _ => Err(McpError {
                code: ErrorCode::INVALID_PARAMS,
                message: format!("Invalid mode '{mode_str}'. Must be 'list' or 'view'.").into(),
                data: None,
            }),
        }
    }
}

impl OrgModeRouter {
    fn map_org_error(e: OrgModeError) -> McpError {
        let error_code = match &e {
            OrgModeError::InvalidDirectory(_) => ErrorCode::INVALID_PARAMS,
            OrgModeError::WalkError(_) => ErrorCode::INTERNAL_ERROR,
            OrgModeError::IoError(_) => ErrorCode::INTERNAL_ERROR,
            _ => ErrorCode::INTERNAL_ERROR,
        };
        McpError {
            code: error_code,
            message: format!("Agenda query failed: {e}").into(),
            data: None,
        }
    }
}

use std::{error, sync::Arc};
use tokio::sync::Mutex;

use org_core::OrgMode;
use rmcp::handler::server::tool::ToolRouter;

pub struct OrgModeRouter {
    pub(crate) org_mode: Arc<Mutex<OrgMode>>,
    pub(crate) tool_router: ToolRouter<Self>,
}

impl OrgModeRouter {
    pub fn new() -> Result<Self, Box<dyn error::Error>> {
        let org_mode = OrgMode::with_defaults()?;
        Ok(Self {
            org_mode: Arc::new(Mutex::new(org_mode)),
            tool_router: Self::tool_router(),
        })
    }

    pub fn with_directory(org_dir: &str) -> Result<Self, Box<dyn error::Error>> {
        let org_mode = OrgMode::new(org_dir)?;
        Ok(Self {
            org_mode: Arc::new(Mutex::new(org_mode)),
            tool_router: Self::tool_router(),
        })
    }

    fn tool_router() -> ToolRouter<Self> {
        Self::tool_router_list_files() + Self::tool_router_search()
    }
}

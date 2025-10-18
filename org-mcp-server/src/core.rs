use std::{error, sync::Arc};
use tokio::sync::Mutex;

use org_core::{OrgMode, config::OrgConfig};
use rmcp::handler::server::tool::ToolRouter;

pub struct OrgModeRouter {
    pub(crate) org_mode: Arc<Mutex<OrgMode>>,
    pub(crate) tool_router: ToolRouter<Self>,
}

impl OrgModeRouter {
    pub fn with_config(config: OrgConfig) -> Result<Self, Box<dyn error::Error>> {
        let org_mode = OrgMode::new(config)?;
        Ok(Self {
            org_mode: Arc::new(Mutex::new(org_mode)),
            tool_router: Self::tool_router(),
        })
    }

    pub fn with_directory(org_dir: &str) -> Result<Self, Box<dyn error::Error>> {
        let config = OrgConfig {
            org_directory: org_dir.to_string(),
            ..OrgConfig::default()
        };
        let config = config.validate()?;
        Self::with_config(config)
    }

    fn tool_router() -> ToolRouter<Self> {
        Self::tool_router_list_files() + Self::tool_router_search()
    }
}

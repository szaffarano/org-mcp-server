use mcp_server::core::OrgModeRouter;
use rmcp::{ServiceExt, transport::stdio};
use tracing::{error, info};

use std::error;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    // TODO parameterize log location and level
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .init();

    info!("Starting MCP server");

    let service = OrgModeRouter::new()?
        .serve(stdio())
        .await
        .inspect_err(|e| {
            error!("Error starting server: {e}");
        })?;
    service.waiting().await?;

    Ok(())
}

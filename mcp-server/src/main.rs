use clap::Parser;
use mcp_server::core::OrgModeRouter;
use rmcp::{ServiceExt, transport::stdio};
use tracing::{error, info};

use std::error;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "mcp-server")]
#[command(about = "MCP server for org-mode knowledge management")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Root directory containing org-mode files
    #[arg(short, long, default_value = "~/org/")]
    root: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let cli = Cli::parse();

    // TODO parameterize log location and level
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .init();

    info!("Starting MCP server with org directory: {}", cli.root);

    let service = OrgModeRouter::with_directory(&cli.root)?
        .serve(stdio())
        .await
        .inspect_err(|e| {
            error!("Error starting server: {e}");
        })?;
    service.waiting().await?;

    Ok(())
}

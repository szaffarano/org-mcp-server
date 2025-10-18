use clap::Parser;
use org_core::Config;
use org_mcp_server::core::OrgModeRouter;
use rmcp::{ServiceExt, transport::stdio};
use tracing::{error, info};

use std::error;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "org-mcp-server")]
#[command(about = "MCP server for org-mode knowledge management")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<String>,

    /// Root directory containing org-mode files
    #[arg(short, long)]
    root_directory: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long)]
    log_level: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let cli = Cli::parse();

    // Load configuration with CLI overrides
    let config =
        Config::load_with_overrides(cli.config, cli.root_directory, cli.log_level.clone())?;

    // Initialize logging with config
    let log_level = cli
        .log_level
        .as_deref()
        .unwrap_or(&config.logging.level)
        .parse::<tracing::Level>()
        .unwrap_or(tracing::Level::INFO);

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(log_level.into()))
        .with_writer(std::io::stderr)
        .init();

    info!(
        "Starting MCP server with org directory: {}",
        config.org.org_directory
    );

    let service = OrgModeRouter::with_config(config.org)?
        .serve(stdio())
        .await
        .inspect_err(|e| {
            error!("Error starting server: {e}");
        })?;
    service.waiting().await?;

    Ok(())
}

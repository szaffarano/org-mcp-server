use clap::Parser;
use org_mcp_server::{
    config::{ServerAppConfig, Transport},
    core::OrgModeRouter,
};
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

    /// Transport to serve MCP over (stdio, http)
    #[arg(short, long, value_enum)]
    transport: Option<Transport>,

    /// Address to bind when using the http transport (e.g. 127.0.0.1:8000)
    #[arg(short, long)]
    bind: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let cli = Cli::parse();

    // Load configuration with CLI overrides
    let mut config = ServerAppConfig::load(cli.config, cli.root_directory, cli.log_level.clone())?;

    if let Some(transport) = cli.transport {
        config.server.transport = transport;
    }
    if let Some(bind) = cli.bind {
        config.server.bind_address = bind;
    }

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

    let router = OrgModeRouter::with_config(config.org)?;

    match config.server.transport {
        Transport::Stdio => {
            let service = router.serve(stdio()).await.inspect_err(|e| {
                error!("Error starting server: {e}");
            })?;
            service.waiting().await?;
        }
        Transport::Http => serve_http(router, &config.server.bind_address).await?,
    }

    Ok(())
}

#[cfg(feature = "http")]
async fn serve_http(
    router: OrgModeRouter,
    bind_address: &str,
) -> Result<(), Box<dyn error::Error>> {
    let listener = tokio::net::TcpListener::bind(bind_address).await?;
    info!(
        "Serving MCP over HTTP on http://{}{}",
        listener.local_addr()?,
        org_mcp_server::http::MCP_PATH
    );

    org_mcp_server::http::serve(router, listener, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
}

#[cfg(not(feature = "http"))]
async fn serve_http(_: OrgModeRouter, _: &str) -> Result<(), Box<dyn error::Error>> {
    Err("the http transport requires building with `--features http`".into())
}

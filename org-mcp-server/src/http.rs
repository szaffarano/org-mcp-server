use std::{error, future::Future, sync::Arc};

use axum::Router;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

use crate::core::OrgModeRouter;

pub const MCP_PATH: &str = "/mcp";

pub fn router(handler: OrgModeRouter, cancel: CancellationToken) -> Router {
    let service = StreamableHttpService::new(
        move || Ok(handler.clone()),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_cancellation_token(cancel),
    );
    Router::new().nest_service(MCP_PATH, service)
}

pub async fn serve(
    handler: OrgModeRouter,
    listener: TcpListener,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), Box<dyn error::Error>> {
    let cancel = CancellationToken::new();
    let app = router(handler, cancel.child_token());

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown.await;
            cancel.cancel();
        })
        .await?;
    Ok(())
}

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

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
        sync::oneshot,
    };

    use super::*;

    async fn post_mcp(addr: std::net::SocketAddr, path: &str, body: &str) -> String {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n\
             Accept: application/json, text/event-stream\r\nConnection: close\r\n\
             Content-Length: {}\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        response
    }

    #[tokio::test]
    async fn test_serve_handles_initialize_and_shuts_down() {
        let org_dir = test_utils::fixtures::setup_test_org_files().unwrap();
        let handler = OrgModeRouter::with_directory(org_dir.path().to_str().unwrap()).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let server = serve(handler, listener, async move {
            let _ = shutdown_rx.await;
        });

        let initialize = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0.0.0"}}}"#;
        let client = async {
            let response = post_mcp(addr, MCP_PATH, initialize).await;
            assert!(response.starts_with("HTTP/1.1 200"), "{response}");
            assert!(response.contains("serverInfo"), "{response}");

            let response = post_mcp(addr, "/other", initialize).await;
            assert!(response.starts_with("HTTP/1.1 404"), "{response}");

            shutdown_tx.send(()).unwrap();
        };

        let (result, ()) = tokio::join!(server, client);
        result.unwrap();
    }
}

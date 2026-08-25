//! Stdio-only transport composition and lifecycle ownership.

use ores_mcp_server_core_libs::state_machine::LifecycleController;
use ores_mcp_server_core_libs::transport::{CancellationToken, run_stdio as run_bounded_stdio};

use crate::server::{ApostilleMeMCPServer, SERVER_NAME, SERVER_NAMESPACE};

/// Initialize telemetry and serve bounded MCP frames on stdio.
///
/// # Errors
///
/// Returns an error when lifecycle initialization or MCP transport
/// startup, service, or shutdown fails.
pub async fn run_stdio() -> anyhow::Result<()> {
    let _telemetry = ores_mcp_server_core_libs::observability::init(SERVER_NAME, SERVER_NAMESPACE);
    let lifecycle = LifecycleController::new(128)?;
    let cancellation = CancellationToken::new();
    let signal_token = cancellation.clone();
    tokio::spawn(async move {
        wait_for_shutdown_signal().await;
        signal_token.cancel();
    });
    let server_lifecycle = lifecycle.clone();
    run_bounded_stdio(
        move || Ok(ApostilleMeMCPServer::new(server_lifecycle.clone())),
        lifecycle,
        cancellation,
    )
    .await?;
    Ok(())
}

async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        if let Ok(mut terminate) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = terminate.recv() => {},
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

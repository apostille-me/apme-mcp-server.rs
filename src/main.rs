use apme_mcp_server::{ApmeMcp, SERVER_NAME, SERVER_NAMESPACE};
use ore_mcp_runtime::{AccessMode, ExactProtocol, RuntimeError, RuntimeSpec, run_stdio};
use rmcp::model::ProtocolVersion;

#[tokio::main]
async fn main() -> Result<(), RuntimeError> {
    let spec = RuntimeSpec::stdio(
        SERVER_NAME,
        SERVER_NAMESPACE,
        env!("CARGO_PKG_VERSION"),
        AccessMode::ReadOnly,
    )?;

    run_stdio(
        spec,
        || Ok::<_, RuntimeError>(()),
        |_config, _spec| Ok::<_, RuntimeError>(()),
        |_config, _spec| {
            Ok::<_, RuntimeError>(ExactProtocol::new(ApmeMcp, ProtocolVersion::V_2025_11_25))
        },
    )
    .await
}

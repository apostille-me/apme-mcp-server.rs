//! Typed, read-only MCP tool routing.

use std::{borrow::Cow, sync::Arc};

use ores_mcp_server_core_libs::observability::{ToolClass, ToolMetrics, ToolOutcome};
use ores_mcp_server_core_libs::state_machine::LifecycleController;
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use serde_json::Value;

use crate::{
    domain::{self, PlanInput},
    knowledge,
};

pub const SERVER_NAME: &str = "apme-mcp-server";
pub const SERVER_NAMESPACE: &str = "apostille-me";

#[derive(Clone)]
pub struct ApostilleMeMCPServer {
    tool_router: ToolRouter<Self>,
    metrics: ToolMetrics,
    lifecycle: Arc<LifecycleController>,
}

impl ApostilleMeMCPServer {
    #[must_use]
    pub fn new(lifecycle: LifecycleController) -> Self {
        Self {
            tool_router: Self::tool_router(),
            metrics: ToolMetrics::global(),
            lifecycle: Arc::new(lifecycle),
        }
    }
}

#[tool_router]
impl ApostilleMeMCPServer {
    #[tool(
        description = "Return the product-owned repository topology and component roles. Pure, local, and read-only."
    )]
    #[tracing::instrument(name = "mcp.tool", skip_all, fields(mcp.tool.name = "apme_fleet_map", mcp.tool.class = "inventory"))]
    fn apme_fleet_map(&self) -> String {
        let timer = self.metrics.start(ToolClass::Inventory);
        let output = render(&domain::fleet_map());
        timer.finish(ToolOutcome::Ok);
        output
    }

    #[tool(
        description = "Calculate a bounded, deterministic product plan from a closed workload enum and numeric units. Never executes or mutates anything."
    )]
    #[tracing::instrument(name = "mcp.tool", skip_all, fields(mcp.tool.name = "apme_plan", mcp.tool.class = "details"))]
    fn apme_plan(&self, Parameters(input): Parameters<PlanInput>) -> Result<String, String> {
        let timer = self.metrics.start(ToolClass::Details);
        let result = domain::plan(input).map(|value| render(&value));
        timer.finish(if result.is_ok() {
            ToolOutcome::Ok
        } else {
            ToolOutcome::Rejected
        });
        result
    }

    #[tool(
        description = "Report presence-only configuration readiness. Values are never read into output, logged, or authenticated; no network request is made."
    )]
    #[tracing::instrument(name = "mcp.tool", skip_all, fields(mcp.tool.name = "apme_runtime_readiness", mcp.tool.class = "health"))]
    fn apme_runtime_readiness(&self) -> String {
        let timer = self.metrics.start(ToolClass::Health);
        let output = render(&domain::runtime_readiness());
        timer.finish(ToolOutcome::Ok);
        output
    }

    #[tool(
        description = "Return bounded shared knowledge for ORE Kubernetes, shared definitions, dpm, Cloudflare/Squarespace, Supabase, and Fiducia. Descriptive only."
    )]
    #[tracing::instrument(name = "mcp.tool", skip_all, fields(mcp.tool.name = "apme_shared_platform", mcp.tool.class = "inventory"))]
    fn apme_shared_platform(&self) -> String {
        let timer = self.metrics.start(ToolClass::Inventory);
        let output = render(&knowledge::shared_platform());
        timer.finish(ToolOutcome::Ok);
        output
    }

    #[tool(
        description = "Return the formal runtime lifecycle state, monotonic revision, and bounded transition audit. Callers cannot trigger transitions."
    )]
    #[tracing::instrument(name = "mcp.tool", skip_all, fields(mcp.tool.name = "apme_lifecycle_state", mcp.tool.class = "health"))]
    fn apme_lifecycle_state(&self) -> Result<String, String> {
        let timer = self.metrics.start(ToolClass::Health);
        let result = self
            .lifecycle
            .snapshot_and_audit()
            .map(|(snapshot, audit)| {
                render(&serde_json::json!({
                    "state": snapshot.state(),
                    "revision": snapshot.revision(),
                    "transitions": audit,
                    "readOnly": true
                }))
            })
            .map_err(|error| error.to_string());
        timer.finish(if result.is_ok() {
            ToolOutcome::Ok
        } else {
            ToolOutcome::Error
        });
        result
    }

    #[tool(
        description = "Return the product-specific safety and privacy boundary. Pure, local, and read-only."
    )]
    #[tracing::instrument(name = "mcp.tool", skip_all, fields(mcp.tool.name = "apme_safety_boundary", mcp.tool.class = "inventory"))]
    fn apme_safety_boundary(&self) -> String {
        let timer = self.metrics.start(ToolClass::Inventory);
        let output = render(&domain::safety_boundary());
        timer.finish(ToolOutcome::Ok);
        output
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ApostilleMeMCPServer {
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(&[
            ProtocolVersion::V_2026_07_28,
            ProtocolVersion::V_2025_11_25,
            ProtocolVersion::V_2025_06_18,
        ])
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(SERVER_NAME, env!("CARGO_PKG_VERSION")).with_title("Apostille Me MCP Server"))
            .with_instructions("Read-only MCP diagnostics for apostille document, provider, status, and delivery contracts. The server is read-only and never logs MCP arguments or results.")
    }
}

fn render(value: &Value) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| r#"{"error":"serialization failed"}"#.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_prefixed_tool_catalog_is_exposed() {
        let server =
            ApostilleMeMCPServer::new(LifecycleController::new(8).expect("valid lifecycle"));
        let names = server
            .tool_router
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "apme_fleet_map",
                "apme_lifecycle_state",
                "apme_plan",
                "apme_runtime_readiness",
                "apme_safety_boundary",
                "apme_shared_platform",
            ]
        );
    }

    #[test]
    fn metadata_is_read_only_and_namespaced() {
        let server =
            ApostilleMeMCPServer::new(LifecycleController::new(8).expect("valid lifecycle"));
        let info = server.get_info();
        assert_eq!(info.server_info.name, SERVER_NAME);
        assert!(
            info.instructions
                .as_deref()
                .is_some_and(|value| value.contains("read-only"))
        );
    }
}

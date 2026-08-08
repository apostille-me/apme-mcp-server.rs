//! Apostille Me's read-only MCP server contract.
//!
//! Official `rmcp` owns protocol lifecycle and dispatch. Product-owned package
//! coordinates remain local, while `ore-mcp-zed-graph` owns the closed tool
//! descriptor and text-plus-structured result contract.

#![forbid(unsafe_code)]

use ore_mcp_zed_graph::{DependencyGraph, TOOL_NAME, tool_descriptor};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, Implementation, JsonObject, ListToolsResult,
        PaginatedRequestParams, ProtocolVersion, ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext,
};

/// Stable MCP server implementation name.
pub const SERVER_NAME: &str = "apme-mcp-server";
/// Stable human-readable MCP server title.
pub const SERVER_TITLE: &str = "Apostille Me MCP Server";
/// Stable product namespace used by the shared runtime.
pub const SERVER_NAMESPACE: &str = "apostille-me";
/// Product instructions preserved from the handwritten server.
pub const SERVER_INSTRUCTIONS: &str =
    "Read-only server that reports the canonical Apostille Me Zed dependency graph.";

const ORGANIZATION: &str = "apostille-me";
const REPOSITORY: &str = "apostille-me/apme-mcp-server.rs";
const PACKAGE: &str = "apme-mcp-server";
const DEPENDENCIES: [&str; 6] = [
    "apostille-me/apme-clients",
    "apostille-me/apme-interfaces",
    "apostille-me/apme-libs",
    "apostille-me/apme-cli",
    "apostille-me/apme-sync",
    "shared-auth/shared-auth-clients",
];

/// Stateless, read-only Apostille Me MCP handler.
#[derive(Clone, Copy, Debug, Default)]
pub struct ApmeMcp;

/// Returns the product-owned dependency graph.
#[must_use]
pub fn dependency_graph() -> DependencyGraph {
    DependencyGraph::new(ORGANIZATION, REPOSITORY, PACKAGE, DEPENDENCIES)
        .expect("static Apostille Me dependency graph must remain valid")
}

/// Returns the exact shared MCP tool descriptor as an official `rmcp` model.
#[must_use]
pub fn dependency_tool() -> Tool {
    serde_json::from_value(tool_descriptor())
        .expect("shared Zed dependency tool descriptor must deserialize as an rmcp Tool")
}

/// Returns the exact shared text-plus-structured tool result as an official
/// `rmcp` model.
#[must_use]
pub fn dependency_tool_result() -> CallToolResult {
    serde_json::from_value(dependency_graph().tool_result())
        .expect("shared Zed dependency result must deserialize as an rmcp CallToolResult")
}

/// Returns final MCP 2025-11-25 server metadata.
#[must_use]
pub fn server_info() -> ServerInfo {
    ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
        .with_protocol_version(ProtocolVersion::V_2025_11_25)
        .with_server_info(
            Implementation::new(SERVER_NAME, env!("CARGO_PKG_VERSION")).with_title(SERVER_TITLE),
        )
        .with_instructions(SERVER_INSTRUCTIONS)
}

fn require_empty_arguments(arguments: Option<&JsonObject>) -> Result<(), McpError> {
    if arguments.is_some_and(|arguments| !arguments.is_empty()) {
        return Err(McpError::invalid_params(
            "zed_dependency_graph accepts only an empty argument object",
            None,
        ));
    }
    Ok(())
}

impl ServerHandler for ApmeMcp {
    fn get_info(&self) -> ServerInfo {
        server_info()
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(vec![dependency_tool()]))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        if request.name.as_ref() != TOOL_NAME {
            return Err(McpError::invalid_params(
                format!("unknown tool: {}", request.name),
                None,
            ));
        }
        require_empty_arguments(request.arguments.as_ref())?;
        Ok(dependency_tool_result())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value, json};

    use super::*;

    #[test]
    fn server_metadata_is_stable_and_final() {
        let info = server_info();
        assert_eq!(info.protocol_version, ProtocolVersion::V_2025_11_25);
        assert_eq!(info.server_info.name, SERVER_NAME);
        assert_eq!(info.server_info.title.as_deref(), Some(SERVER_TITLE));
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.instructions.as_deref(), Some(SERVER_INSTRUCTIONS));

        let serialized = serde_json::to_value(info).expect("serialize server info");
        assert!(serialized["capabilities"]["tools"].is_object());
        assert_eq!(serialized["protocolVersion"], "2025-11-25");
    }

    #[test]
    fn tool_descriptor_round_trips_without_contract_drift() {
        let serialized = serde_json::to_value(dependency_tool()).expect("serialize tool");
        assert_eq!(serialized, tool_descriptor());
        assert_eq!(serialized["name"], TOOL_NAME);
        assert_eq!(serialized["inputSchema"]["additionalProperties"], false);
    }

    #[test]
    fn tool_result_round_trips_without_contract_drift() {
        let serialized =
            serde_json::to_value(dependency_tool_result()).expect("serialize tool result");
        assert_eq!(serialized, dependency_graph().tool_result());
        assert_eq!(serialized["isError"], false);
        assert_eq!(serialized["structuredContent"]["dependencies"], json!(DEPENDENCIES));
    }

    #[test]
    fn graph_retains_exact_product_coordinates() {
        let graph = dependency_graph();
        assert_eq!(graph.organization(), ORGANIZATION);
        assert_eq!(graph.repository(), REPOSITORY);
        assert_eq!(graph.package(), PACKAGE);
        assert_eq!(graph.dependencies(), DEPENDENCIES);
    }

    #[test]
    fn only_empty_or_absent_arguments_are_accepted() {
        assert!(require_empty_arguments(None).is_ok());
        assert!(require_empty_arguments(Some(&Map::<String, Value>::new())).is_ok());

        let mut unexpected = Map::new();
        unexpected.insert("unexpected".to_string(), Value::Bool(true));
        assert!(require_empty_arguments(Some(&unexpected)).is_err());
    }
}

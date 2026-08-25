use std::time::Duration;

use serde_json::Value;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn real_process_negotiates_and_lists_namespaced_tools_without_stdout_noise() {
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_apme-mcp-server"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn MCP server");

    let mut stdin = child.stdin.take().expect("piped stdin");
    stdin.write_all(concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2026-07-28","capabilities":{},"clientInfo":{"name":"fleet-smoke","version":"1"}}}"#,
            "
",
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            "
",
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
            "
"
        ).as_bytes()).await.expect("write MCP frames");
    drop(stdin);

    let output = tokio::time::timeout(Duration::from_secs(15), child.wait_with_output())
        .await
        .expect("server exits after stdin EOF")
        .expect("collect output");
    assert!(
        output.status.success(),
        "server failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout UTF-8");
    let frames = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str::<Value>(line).expect("stdout contains JSON-RPC only"))
        .collect::<Vec<_>>();
    assert!(frames.iter().any(|frame| frame["id"] == 1));
    let tools = frames
        .iter()
        .find(|frame| frame["id"] == 2)
        .expect("tools/list response");
    let serialized = serde_json::to_string(tools).expect("serialize tools response");
    assert!(serialized.contains("apme_fleet_map"));
    assert!(serialized.contains("apme_plan"));
}

#[test]
fn architecture_reserves_stdout_and_uses_explicit_low_cardinality_telemetry() {
    let main = include_str!("../src/main.rs");
    let runtime = include_str!("../src/runtime.rs");
    let server = include_str!("../src/server.rs");
    let manifest = include_str!("../Cargo.toml");
    assert!(!main.contains("println!"));
    assert!(!runtime.contains("println!"));
    assert!(!server.contains("println!"));
    assert!(server.contains("ToolMetrics"));
    assert!(server.contains("tracing::instrument"));
    assert!(server.contains("skip_all"));
    assert!(!server.contains("mcp.tool.arguments"));
    assert!(!server.contains("mcp.tool.result"));
    assert!(manifest.contains("c6101656c8227251d1dbd61df54f03a186b42ade"));
}

#[test]
fn shared_knowledge_names_every_required_boundary() {
    let source = include_str!("../src/knowledge.rs");
    for required in [
        "oreKubernetes",
        "sharedDefinitions",
        "dpm",
        "cloudflareSquarespace",
        "supabase",
        "fiducia",
    ] {
        assert!(
            source.contains(required),
            "missing shared boundary {required}"
        );
    }
}

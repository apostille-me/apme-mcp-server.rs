use std::{
    collections::BTreeMap,
    io::{Read, Write},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use apme_mcp_server::{SERVER_NAME, dependency_graph};
use ore_mcp_testkit::{
    audit_closed_world_tool_catalog_response, audit_initialize_response, audit_stdio_stdout,
    audit_text_tool_result_response,
};
use ore_mcp_zed_graph::{TOOL_NAME, tool_descriptor};
use serde_json::{Value, json};

const PROCESS_TIMEOUT: Duration = Duration::from_secs(10);

fn line(value: Value) -> String {
    serde_json::to_string(&value).expect("serialize test request")
}

fn run_session(requests: &[Value]) -> (Vec<u8>, Vec<u8>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_apme-mcp-server"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn Apostille Me MCP server");

    let mut stdin = child.stdin.take().expect("child stdin");
    for request in requests {
        stdin
            .write_all(line(request.clone()).as_bytes())
            .expect("write MCP request");
        stdin.write_all(b"\n").expect("write request delimiter");
    }
    stdin.flush().expect("flush MCP requests");
    drop(stdin);

    let deadline = Instant::now() + PROCESS_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll MCP process") {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill timed-out MCP process");
            let _ = child.wait();
            panic!("MCP process did not stop after stdin closed");
        }
        thread::sleep(Duration::from_millis(20));
    };

    let mut stdout = Vec::new();
    child
        .stdout
        .take()
        .expect("child stdout")
        .read_to_end(&mut stdout)
        .expect("read MCP stdout");
    let mut stderr = Vec::new();
    child
        .stderr
        .take()
        .expect("child stderr")
        .read_to_end(&mut stderr)
        .expect("read MCP stderr");

    assert!(
        status.success(),
        "MCP process failed: {}",
        String::from_utf8_lossy(&stderr)
    );
    (stdout, stderr)
}

fn responses_by_id(stdout: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let text = std::str::from_utf8(stdout).expect("stdout must be UTF-8");
    let mut responses = BTreeMap::new();
    for raw_line in text.lines().filter(|line| !line.trim().is_empty()) {
        let value: Value = serde_json::from_str(raw_line).expect("stdout frame must be JSON");
        if let Some(id) = value.get("id") {
            responses.insert(id.to_string(), raw_line.as_bytes().to_vec());
        }
    }
    responses
}

#[test]
fn official_rmcp_process_preserves_protocol_and_tool_contract() {
    let requests = [
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "den-2972-test", "version": "1.0.0"}
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {"name": TOOL_NAME, "arguments": {}}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {"name": TOOL_NAME, "arguments": {"unexpected": true}}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {"name": TOOL_NAME, "arguments": []}
        }),
        json!({"jsonrpc": "2.0", "id": 6, "method": "ping", "params": {}}),
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "apostille/unknown",
            "params": {}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {"name": TOOL_NAME}
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {"name": TOOL_NAME, "arguments": null}
        }),
    ];

    let (stdout, _stderr) = run_session(&requests);
    let frame_audit = audit_stdio_stdout(&stdout).expect("stdout must contain only MCP frames");
    assert_eq!(frame_audit.response_count, 9);
    assert_eq!(frame_audit.notification_count, 0);

    let responses = responses_by_id(&stdout);
    let initialize = responses.get("1").expect("initialize response");
    let initialize_audit = audit_initialize_response(initialize, &json!(1), &["2025-11-25"])
        .expect("final protocol initialize response");
    assert_eq!(initialize_audit.protocol_version, "2025-11-25");
    assert_eq!(initialize_audit.server_name, SERVER_NAME);
    assert_eq!(initialize_audit.server_version, env!("CARGO_PKG_VERSION"));

    let list = responses.get("2").expect("tools/list response");
    let catalog = audit_closed_world_tool_catalog_response(list, &json!(2), 1)
        .expect("one closed-world tool");
    assert_eq!(catalog.tool_count, 1);
    assert_eq!(catalog.tool_names, vec![TOOL_NAME.to_string()]);
    let list_json: Value = serde_json::from_slice(list).expect("parse tools/list response");
    assert_eq!(list_json["result"]["tools"], json!([tool_descriptor()]));

    for id in ["3", "8", "9"] {
        let call = responses.get(id).expect("tools/call response");
        let result = audit_text_tool_result_response(call, &json!(id.parse::<u64>().unwrap()), 64 * 1024)
            .expect("bounded text tool result");
        assert_eq!(result.content_items, 1);
        assert!(!result.is_error);
        let call_json: Value = serde_json::from_slice(call).expect("parse tools/call response");
        assert_eq!(call_json["result"], dependency_graph().tool_result());
    }

    for id in ["4", "5"] {
        let response: Value =
            serde_json::from_slice(responses.get(id).expect("invalid-argument response"))
                .expect("parse invalid-argument response");
        assert_eq!(response["error"]["code"], -32602);
    }

    let ping: Value = serde_json::from_slice(responses.get("6").expect("ping response"))
        .expect("parse ping response");
    assert_eq!(ping["result"], json!({}));

    let unknown: Value = serde_json::from_slice(responses.get("7").expect("unknown response"))
        .expect("parse unknown response");
    assert_eq!(unknown["error"]["code"], -32601);
}

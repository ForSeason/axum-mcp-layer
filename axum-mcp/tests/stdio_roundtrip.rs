#![cfg(feature = "stdio")]
use mcp_protocol_sdk::protocol::types::{JSONRPC_VERSION, JsonRpcRequest};
use mcp_protocol_sdk::transport::stdio::StdioClientTransport;
use mcp_protocol_sdk::transport::traits::Transport;
use serde_json::json;
use std::path::PathBuf;
use tokio::process::Command;

#[tokio::test]
async fn tools_list_and_call() {
    let status = Command::new("cargo")
        .args(["build", "-p", "axum-mcp-demo-stdio"])
        .status()
        .await
        .expect("build failed");
    assert!(status.success());

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push("target");
    path.push("debug");
    let exe = if cfg!(windows) {
        "axum-mcp-demo-stdio.exe"
    } else {
        "axum-mcp-demo-stdio"
    };
    path.push(exe);

    let mut transport = StdioClientTransport::new(path.to_str().unwrap(), vec![])
        .await
        .expect("spawn stdio server");

    let list_req = JsonRpcRequest {
        jsonrpc: JSONRPC_VERSION.to_string(),
        id: json!(1),
        method: "tools/list".into(),
        params: None,
    };
    let list_resp = transport.send_request(list_req).await.unwrap();
    let tools = list_resp.result.unwrap()["tools"]
        .as_array()
        .unwrap()
        .clone();
    assert!(tools.iter().any(|t| t["name"] == "sum"));

    let call_req = JsonRpcRequest {
        jsonrpc: JSONRPC_VERSION.to_string(),
        id: json!(2),
        method: "tools/call".into(),
        params: Some(json!({"name":"sum","arguments":{"a":1,"b":2}})),
    };
    let call_resp = transport.send_request(call_req).await.unwrap();
    assert_eq!(call_resp.result.unwrap()["result"]["sum"], 3);

    transport.close().await.unwrap();
}

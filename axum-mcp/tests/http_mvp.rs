#![cfg(feature = "http")]
use std::sync::Arc;

use axum::http::{Request, HeaderValue};
use axum::body::Body;
use axum_mcp::{security::AllowedOrigins, ToolRegistry};
use axum_mcp::tool::{ToolHandler, ToolCtx};
use async_trait::async_trait;

struct Echo;
#[async_trait]
impl ToolHandler for Echo {
    async fn call(&self, _ctx: &ToolCtx, args: serde_json::Value) -> Result<serde_json::Value, axum_mcp::tool::ToolError> {
        Ok(args)
    }
}

#[tokio::test]
async fn tools_list_and_call() {
    let reg = ToolRegistry::empty_with_state(Arc::new(()));
    reg.insert(axum_mcp::tool::ToolDescriptor {
        name: "echo",
        description: Some("echo"),
        input_schema: schemars::schema_for!(serde_json::Value),
        output_schema: schemars::schema_for!(serde_json::Value),
        handler: Arc::new(Echo),
        structured: true,
    }).await;

    // list
    let req = Request::post("/mcp")
        .header("MCP-Protocol-Version", axum_mcp::security::REQUIRED_PROTOCOL_VERSION)
        .header("Origin", HeaderValue::from_static("http://127.0.0.1:3000"))
        .body(Body::from("{\"op\":\"tools/list\"}"))
        .unwrap();
    let resp = axum_mcp::http::handle_post(
        req,
        &reg,
        AllowedOrigins::LocalhostOnly,
        axum_mcp::security::Auth::None,
        axum_mcp::security::VersionPolicy::AllowFallback { required: axum_mcp::security::REQUIRED_PROTOCOL_VERSION, fallback: axum_mcp::security::FALLBACK_PROTOCOL_VERSION }
    ).await;
    assert_eq!(resp.status(), 200);

    // call
    let req = Request::post("/mcp")
        .header("MCP-Protocol-Version", axum_mcp::security::REQUIRED_PROTOCOL_VERSION)
        .header("Origin", HeaderValue::from_static("http://127.0.0.1:3000"))
        .body(Body::from("{\"op\":\"tools/call\",\"name\":\"echo\",\"args\":{\"x\":1}}"))
        .unwrap();
    let resp = axum_mcp::http::handle_post(
        req,
        &reg,
        AllowedOrigins::LocalhostOnly,
        axum_mcp::security::Auth::None,
        axum_mcp::security::VersionPolicy::AllowFallback { required: axum_mcp::security::REQUIRED_PROTOCOL_VERSION, fallback: axum_mcp::security::FALLBACK_PROTOCOL_VERSION }
    ).await;
    assert_eq!(resp.status(), 200);
}

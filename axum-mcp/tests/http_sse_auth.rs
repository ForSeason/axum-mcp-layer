#![cfg(feature = "http")]
#![allow(unused_imports, dead_code)]
use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{HeaderValue, Method, Request};
use axum_mcp::tool::{ToolCtx, ToolHandler};
use axum_mcp::{
    ToolRegistry,
    security::{AllowedOrigins, Auth},
};

struct Echo;
#[async_trait]
impl ToolHandler for Echo {
    async fn call(
        &self,
        _ctx: &ToolCtx,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, axum_mcp::tool::ToolError> {
        Ok(args)
    }
}

#[tokio::test]
async fn auth_rejects_without_bearer() {
    let reg = ToolRegistry::empty_with_state(Arc::new(()));
    let req = Request::post("/mcp")
        .header(
            "MCP-Protocol-Version",
            axum_mcp::security::REQUIRED_PROTOCOL_VERSION,
        )
        .header("Origin", HeaderValue::from_static("http://127.0.0.1:3000"))
        .body(Body::from("{\"op\":\"tools/list\"}"))
        .unwrap();
    let resp = axum_mcp::http::handle_post(
        req,
        &reg,
        AllowedOrigins::LocalhostOnly,
        Auth::Bearer {
            token: "secret".into(),
        },
        axum_mcp::security::VersionPolicy::AllowFallback {
            required: axum_mcp::security::REQUIRED_PROTOCOL_VERSION,
            fallback: axum_mcp::security::FALLBACK_PROTOCOL_VERSION,
        },
    )
    .await;
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn sse_get_returns_event_stream() {
    let reg = ToolRegistry::empty_with_state(Arc::new(()));
    let resp =
        axum_mcp::http::handle_sse_get(&reg, AllowedOrigins::LocalhostOnly, Auth::None).await;
    assert_eq!(resp.status(), 200);
    let ct = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.starts_with("text/event-stream"));
}

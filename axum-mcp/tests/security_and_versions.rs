#![cfg(feature = "http")]
use std::sync::Arc;
use axum::http::{Request, HeaderValue, StatusCode};
use axum::body::Body;
use axum_mcp::{security::{AllowedOrigins, REQUIRED_PROTOCOL_VERSION}, ToolRegistry};

#[tokio::test]
async fn missing_version_header_fallback_allowed() {
    let reg = ToolRegistry::empty_with_state(Arc::new(()));
    let req = Request::post("/mcp")
        .header("Origin", HeaderValue::from_static("http://127.0.0.1:3000"))
        .body(Body::from("{\"op\":\"tools/list\"}"))
        .unwrap();
    let resp = axum_mcp::http::handle_post(
        req,
        &reg,
        AllowedOrigins::LocalhostAll,
        axum_mcp::security::Auth::None,
        axum_mcp::security::VersionPolicy::AllowFallback { required: axum_mcp::security::REQUIRED_PROTOCOL_VERSION, fallback: axum_mcp::security::FALLBACK_PROTOCOL_VERSION }
    ).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn wrong_origin_rejected() {
    let reg = ToolRegistry::empty_with_state(Arc::new(()));
    let req = Request::post("/mcp")
        .header("MCP-Protocol-Version", REQUIRED_PROTOCOL_VERSION)
        .header("Origin", HeaderValue::from_static("http://evil.example.com"))
        .body(Body::from("{\"op\":\"tools/list\"}"))
        .unwrap();
    let resp = axum_mcp::http::handle_post(
        req,
        &reg,
        AllowedOrigins::LocalhostOnly,
        axum_mcp::security::Auth::None,
        axum_mcp::security::VersionPolicy::AllowFallback { required: axum_mcp::security::REQUIRED_PROTOCOL_VERSION, fallback: axum_mcp::security::FALLBACK_PROTOCOL_VERSION }
    ).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn unknown_tool_and_invalid_args() {
    let reg = ToolRegistry::empty_with_state(Arc::new(()));
    // unknown tool
    let req = Request::post("/mcp")
        .header("MCP-Protocol-Version", REQUIRED_PROTOCOL_VERSION)
        .header("Origin", HeaderValue::from_static("http://127.0.0.1:3000"))
        .body(Body::from("{\"op\":\"tools/call\",\"name\":\"not_exist\",\"args\":{}}"))
        .unwrap();
    let resp = axum_mcp::http::handle_post(
        req,
        &reg,
        AllowedOrigins::LocalhostAll,
        axum_mcp::security::Auth::None,
        axum_mcp::security::VersionPolicy::AllowFallback { required: axum_mcp::security::REQUIRED_PROTOCOL_VERSION, fallback: axum_mcp::security::FALLBACK_PROTOCOL_VERSION }
    ).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

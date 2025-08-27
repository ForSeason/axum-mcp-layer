use std::sync::Arc;

use axum::{Router, routing::post};
use axum_mcp::{McpLayer, McpLayerConfig, ToolRegistry};
use axum_mcp_macros::mcp_tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct AppState;

#[derive(Deserialize, JsonSchema)]
struct SumIn { a: i64, b: i64 }

#[derive(Serialize, JsonSchema)]
struct SumOut { sum: i64 }

#[mcp_tool(name="sum", desc="Add two integers", state = "AppState")]
async fn sum(axum::extract::State(state): axum::extract::State<AppState>, axum::Json(inp): axum::Json<SumIn>) -> axum::Json<SumOut> {
    let _ = state; // unused for demo
    axum::Json(SumOut { sum: inp.a + inp.b })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = Arc::new(AppState);
    let registry = ToolRegistry::gather_with_state(state.clone());

    let app = Router::new()
        .route("/sum", post(sum))
        .layer(McpLayer::new(registry, McpLayerConfig {
            path: "/mcp",
            require_version: true,
            allowed_origins: axum_mcp::security::AllowedOrigins::LocalhostAll,
            enable_sse: true,
            auth: axum_mcp::security::Auth::None,
            version_policy: axum_mcp::security::VersionPolicy::AllowFallback { required: axum_mcp::security::REQUIRED_PROTOCOL_VERSION, fallback: axum_mcp::security::FALLBACK_PROTOCOL_VERSION },
        }))
        .with_state((*state).clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:37650").await?;
    println!("HTTP listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

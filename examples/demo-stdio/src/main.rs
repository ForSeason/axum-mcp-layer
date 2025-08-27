use std::sync::Arc;

use axum_mcp::{ToolRegistry};
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
    let _ = state;
    axum::Json(SumOut { sum: inp.a + inp.b })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = Arc::new(AppState);
    let registry = ToolRegistry::gather_with_state(state.clone());
    eprintln!(
        "{}",
        r#"STDIO demo ready.
Type JSON lines like:
{"op":"tools/list"}
{"op":"tools/call","name":"sum","args":{"a":1,"b":2}}"#
    );
    axum_mcp::stdio::run_stdio(registry, state).await
}

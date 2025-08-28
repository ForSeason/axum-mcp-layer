# axum-mcp-layer

Minimal Model Context Protocol (MCP) tooling for [Axum](https://github.com/tokio-rs/axum).

## Features

- `McpLayer` for serving MCP over HTTP.
- `run_stdio` helper for MCP over STDIO, backed by `mcp-protocol-sdk`.
- `#[mcp_tool]` macro to expose Axum handlers as MCP tools.

## Usage

```rust
use axum::{routing::post, Router};
use axum_mcp::{McpLayer, McpLayerConfig, ToolRegistry};
use axum_mcp_macros::mcp_tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, JsonSchema)]
struct SumIn { a: i64, b: i64 }

#[derive(Serialize, JsonSchema)]
struct SumOut { sum: i64 }

#[mcp_tool(name="sum", desc="Add two numbers", state = "()")]
async fn sum(_: axum::extract::State<()>, axum::Json(inp): axum::Json<SumIn>) -> axum::Json<SumOut> {
    axum::Json(SumOut { sum: inp.a + inp.b })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = std::sync::Arc::new(());
    let registry = ToolRegistry::gather_with_state(state.clone());

    // HTTP endpoint
    let app = Router::new()
        .route("/sum", post(sum))
        .layer(McpLayer::new(registry.clone(), McpLayerConfig::default()));
    tokio::spawn(async move {
        axum::serve(tokio::net::TcpListener::bind("127.0.0.1:37650").await?, app).await
    });

    // STDIO endpoint
    axum_mcp::stdio::run_stdio(registry, state).await?
}
```

Run the STDIO example:

```bash
cargo run -p axum-mcp-demo-stdio --features axum-mcp/stdio
```

## Development

- `cargo build --workspace`
- `cargo test -p axum-mcp --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo fmt --all`

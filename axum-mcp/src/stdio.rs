use std::any::Any;
use std::sync::Arc;

use mcp_protocol_sdk::core::error::McpError;
use mcp_protocol_sdk::protocol::types::{self, JsonRpcRequest, JsonRpcResponse};
use mcp_protocol_sdk::transport::stdio::StdioServerTransport;
use mcp_protocol_sdk::transport::traits::ServerTransport;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::registry::ToolRegistry;
use crate::tool::ToolError;

#[derive(serde::Serialize)]
struct ToolMeta {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    desc: Option<&'static str>,
    input_schema: Value,
    output_schema: Value,
    #[serde(rename = "structuredContent")]
    structured: bool,
}

#[derive(Deserialize)]
struct CallParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

pub async fn run_stdio(
    registry: Arc<ToolRegistry>,
    _state: Arc<dyn Any + Send + Sync>,
) -> anyhow::Result<()> {
    let handler = {
        let registry = registry.clone();
        std::sync::Arc::new(move |req: JsonRpcRequest| {
            let registry = registry.clone();
            Box::pin(async move {
                match req.method.as_str() {
                    "tools/list" => {
                        let list = registry.list().await;
                        let tools: Vec<_> = list
                            .into_iter()
                            .map(|(name, desc, i, o)| ToolMeta {
                                name,
                                desc,
                                input_schema: serde_json::to_value(i).unwrap(),
                                output_schema: serde_json::to_value(o).unwrap(),
                                structured: true,
                            })
                            .collect();
                        Ok(JsonRpcResponse {
                            jsonrpc: types::JSONRPC_VERSION.to_string(),
                            id: req.id,
                            result: Some(json!({"tools": tools})),
                        })
                    }
                    "tools/call" => {
                        let params_val = req
                            .params
                            .clone()
                            .ok_or_else(|| McpError::protocol("missing params"))?;
                        let CallParams { name, arguments } = serde_json::from_value(params_val)
                            .map_err(|e| McpError::protocol(format!("invalid params: {e}")))?;
                        match registry.call(&name, arguments).await {
                            Ok(v) => Ok(JsonRpcResponse {
                                jsonrpc: types::JSONRPC_VERSION.to_string(),
                                id: req.id,
                                result: Some(json!({"result": v})),
                            }),
                            Err(e) => Err(match e {
                                ToolError::NotFound(n) => McpError::ToolNotFound(n),
                                ToolError::InvalidArgs(msg) => McpError::Validation(msg),
                                ToolError::Internal(msg) => McpError::Internal(msg),
                            }),
                        }
                    }
                    _ => Err(McpError::protocol(format!(
                        "Method '{}' not found",
                        req.method
                    ))),
                }
            })
                as std::pin::Pin<
                    Box<dyn std::future::Future<Output = Result<JsonRpcResponse, McpError>> + Send>,
                >
        })
    };

    let mut transport = StdioServerTransport::new();
    transport.set_request_handler(handler);
    transport.start().await?;
    Ok(())
}

use std::any::Any;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::registry::ToolRegistry;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Deserialize)]
struct RawOp {
    op: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    args: Value,
}

#[derive(Serialize)]
struct ToolMeta {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")] desc: Option<&'static str>,
    input_schema: Value,
    output_schema: Value,
    #[serde(rename = "structuredContent")]
    structured: bool,
}

pub async fn run_stdio(registry: Arc<ToolRegistry>, _state: Arc<dyn Any + Send + Sync>) -> anyhow::Result<()> {
    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    let mut line = String::new();
    loop {
        line.clear();
        let n = stdin.read_line(&mut line).await?;
        if n == 0 { break; }
        let parsed: Result<RawOp, _> = serde_json::from_str(line.trim());
        let resp = match parsed {
            Ok(raw) => match raw.op.as_str() {
                "tools/list" => {
                    let list = registry.list().await;
                    let tools: Vec<_> = list.into_iter().map(|(name, desc, i, o)| ToolMeta { name, desc, input_schema: serde_json::to_value(i).unwrap(), output_schema: serde_json::to_value(o).unwrap(), structured: true }).collect();
                    json!({"tools": tools})
                }
                "tools/call" => {
                    match raw.name {
                        Some(n) => match registry.call(&n, raw.args).await {
                            Ok(v) => json!({"ok": true, "result": v}),
                            Err(e) => {
                                use crate::tool::ToolError::*;
                                let code = match e { NotFound(_) => "tool_not_found", InvalidArgs(_) => "invalid_args", Internal(_) => "internal" };
                                json!({"ok": false, "code": code, "message": e.to_string()})
                            }
                        },
                        None => json!({"ok": false, "code": "invalid_request", "message": "missing name"})
                    }
                }
                _ => json!({"ok": false, "code": "unknown_op", "message": raw.op }),
            },
            Err(e) => json!({"ok": false, "code": "invalid_json", "message": e.to_string()}),
        };
        let ser = serde_json::to_string(&resp)?;
        stdout.write_all(ser.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }
    Ok(())
}

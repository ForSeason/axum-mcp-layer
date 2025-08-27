use std::sync::Arc;

fn main() {
    let reg = axum_mcp::registry::ToolRegistry::gather_with_state(Arc::new(()));
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tools = rt.block_on(async move { reg.list().await });
    let json_tools: Vec<serde_json::Value> = tools.into_iter().map(|(name, desc, i, o)| {
        serde_json::json!({"name": name, "description": desc, "input_schema": i, "output_schema": o, "structuredContent": true})
    }).collect();
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({"tools": json_tools})).unwrap());
}

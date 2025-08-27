pub mod tool;
pub mod registry;
#[cfg(feature = "http")] pub mod layer;
#[cfg(feature = "http")] pub mod http;
#[cfg(feature = "stdio")] pub mod stdio;
pub mod security;
pub mod prelude;
pub mod schema;

/// Helper trait for converting common HTTP return types into JSON values
/// usable by MCP Tool handlers.
pub trait IntoJsonValue {
    fn into_json_value(self) -> serde_json::Value;
}

impl<T> IntoJsonValue for axum::Json<T>
where
    T: serde::Serialize,
{
    fn into_json_value(self) -> serde_json::Value {
        serde_json::to_value(self.0).unwrap_or(serde_json::Value::Null)
    }
}

#[cfg(feature = "http")] pub use layer::{McpLayer, McpLayerConfig};
pub use registry::{ToolRegistry};

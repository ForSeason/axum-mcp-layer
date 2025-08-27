use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use crate::schema::RootSchema;

#[derive(Clone)]
pub struct ToolDescriptor {
    pub name: &'static str,
    pub description: Option<&'static str>,
    pub input_schema: RootSchema,
    pub output_schema: RootSchema,
    pub handler: Arc<dyn ToolHandler + Send + Sync>,
    pub structured: bool,
}

#[derive(Clone, Default)]
pub struct ReqMeta {
    pub origin: Option<String>,
    pub headers: Vec<(String, String)>,
}

pub struct ToolCtx {
    pub app_state: Arc<dyn Any + Send + Sync>,
    pub req_meta: ReqMeta,
}

#[derive(thiserror::Error, Debug)]
pub enum ToolError {
    #[error("tool_not_found: {0}")]
    NotFound(String),
    #[error("invalid_args: {0}")]
    InvalidArgs(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn call(&self, ctx: &ToolCtx, args: Value) -> Result<Value, ToolError>;
}

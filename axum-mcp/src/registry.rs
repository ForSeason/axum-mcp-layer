use std::{any::Any, collections::HashMap, sync::Arc};

use linkme::distributed_slice;
use serde_json::Value;
use crate::schema::RootSchema;
use tokio::sync::RwLock;

use crate::tool::{ToolCtx, ToolDescriptor, ToolError, ToolHandler};

pub struct ToolRegistration {
    pub name: &'static str,
    pub description: Option<&'static str>,
    pub input_schema: fn() -> RootSchema,
    pub output_schema: fn() -> RootSchema,
    pub build_handler: fn() -> Arc<dyn ToolHandler + Send + Sync>,
    pub defined_at_file: &'static str,
    pub defined_at_line: u32,
    pub structured: bool,
}

#[distributed_slice]
pub static TOOLS: [ToolRegistration] = [..];

pub struct ToolRegistry {
    inner: RwLock<HashMap<String, ToolDescriptor>>,
    app_state: Arc<dyn Any + Send + Sync>,
}

impl ToolRegistry {
    pub fn empty_with_state(app_state: Arc<dyn Any + Send + Sync>) -> Arc<Self> {
        Arc::new(Self { inner: Default::default(), app_state })
    }

    pub fn gather_with_state(app_state: Arc<dyn Any + Send + Sync>) -> Arc<Self> {
        let reg = Self::empty_with_state(app_state);
        // Collect from distributed slice
        for item in TOOLS {
            let desc = ToolDescriptor {
                name: item.name,
                description: item.description,
                input_schema: (item.input_schema)(),
                output_schema: (item.output_schema)(),
                handler: (item.build_handler)(),
                structured: item.structured,
            };
            // Duplicate detection with helpful message
            let existed = futures::executor::block_on(reg.insert(desc));
            if let Some(prev) = existed {
                panic!(
                    "duplicate MCP tool name '{}'\nfirst defined previously, now again at {}:{}",
                    prev.name, item.defined_at_file, item.defined_at_line
                );
            }
        }
        reg
    }

    pub async fn insert(&self, desc: ToolDescriptor) -> Option<ToolDescriptor> {
        self.inner.write().await.insert(desc.name.to_string(), desc)
    }

    pub async fn get(&self, name: &str) -> Option<ToolDescriptor> {
        self.inner.read().await.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<(String, Option<&'static str>, RootSchema, RootSchema)> {
        self.inner
            .read()
            .await
            .values()
            .map(|d| (d.name.to_string(), d.description, d.input_schema.clone(), d.output_schema.clone()))
            .collect()
    }

    pub async fn call(&self, name: &str, args: Value) -> Result<Value, ToolError> {
        let d = self
            .inner
            .read()
            .await
            .get(name)
            .cloned()
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;
        let ctx = ToolCtx { app_state: self.app_state.clone(), req_meta: Default::default() };
        // Optional runtime schema validation (feature = jsonschema)
        #[cfg(feature = "jsonschema")]
        if let Err(e) = crate::schema::validate_json(&args, &d.input_schema) {
            return Err(ToolError::InvalidArgs(e));
        }
        let out = d.handler.call(&ctx, args).await?;
        #[cfg(feature = "jsonschema")]
        if let Err(e) = crate::schema::validate_json(&out, &d.output_schema) {
            return Err(ToolError::Internal(format!("output schema validation failed: {}", e)));
        }
        Ok(out)
    }

    pub fn app_state(&self) -> Arc<dyn Any + Send + Sync> { self.app_state.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct Echo;
    #[async_trait]
    impl ToolHandler for Echo {
        async fn call(&self, _ctx: &crate::tool::ToolCtx, args: Value) -> Result<Value, ToolError> {
            Ok(args)
        }
    }

    #[tokio::test]
    async fn insert_and_call() {
        let r = ToolRegistry::empty_with_state(Arc::new(()));
        r.insert(ToolDescriptor {
            name: "echo",
            description: Some("echo"),
            input_schema: schemars::schema_for!(serde_json::Value),
            output_schema: schemars::schema_for!(serde_json::Value),
            handler: Arc::new(Echo),
            structured: true,
        }).await;

        let out = r.call("echo", serde_json::json!({"a":1})).await.unwrap();
        assert_eq!(out, serde_json::json!({"a":1}));
    }
}

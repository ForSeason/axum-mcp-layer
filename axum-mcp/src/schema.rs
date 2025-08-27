#[cfg(feature = "jsonschema")]
use jsonschema::{Draft, JSONSchema};
use schemars::Schema;
use serde_json::Value;

// Align with docs: expose a RootSchema alias.
pub type RootSchema = Schema;

pub fn schema_for<T: schemars::JsonSchema>() -> RootSchema {
    schemars::schema_for!(T)
}

#[cfg(feature = "jsonschema")]
pub fn validate_json(value: &Value, schema: &RootSchema) -> Result<(), String> {
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&serde_json::to_value(schema).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let result = compiled.validate(value);
    if let Err(errors) = result {
        let msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
        Err(msgs.join("; "))
    } else {
        Ok(())
    }
}

#[cfg(not(feature = "jsonschema"))]
pub fn validate_json(_value: &Value, _schema: &RootSchema) -> Result<(), String> { Ok(()) }

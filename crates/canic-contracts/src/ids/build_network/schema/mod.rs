//! Verified native metadata for renamed unit enums; Serde and storage stay unchanged.
use super::*;
use ic_wire::typescript::{GenerationError, SchemaContext, Wire};
use serde_json::{Value, json};

impl Wire for BuildNetwork {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"enum", "variants":[
        {"name":"ic","id":0,"value":{"k":"unit"}},
        {"name":"local","id":1,"value":{"k":"unit"}}
        ]}))
    }
}

#[cfg(test)]
mod tests;

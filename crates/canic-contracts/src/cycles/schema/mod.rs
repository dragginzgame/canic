//! Metadata for Cycles' existing binary newtype serialization; human-readable storage stays text.
use super::Cycles;
use ic_wire::typescript::{GenerationError, SchemaContext, Wire};
impl Wire for Cycles {
    fn native_schema(context: &mut SchemaContext) -> Result<serde_json::Value, GenerationError> {
        <u128 as Wire>::native_schema(context)
    }
}

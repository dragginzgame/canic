use canic_contracts::ids::ReleaseBuildId;
use serde::{Deserialize, Serialize};

///
/// EndpointReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct EndpointReport {
    pub(super) source: String,
    pub(super) source_kind: EndpointSourceKind,
    pub(super) release_build_id: Option<ReleaseBuildId>,
    pub(super) endpoints: Vec<EndpointEntry>,
    pub(super) types: serde_json::Value,
}

/// Origin of the declaration being inspected, independent of its display path.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum EndpointSourceKind {
    Built,
    Installed,
}

/// Display projection of a release-admitted Wire method; payload semantics stay with Wire.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct EndpointEntry {
    pub name: String,
    pub id: String,
    pub kind: String,
    pub mode: String,
    pub request: serde_json::Value,
    pub response: serde_json::Value,
    pub request_bytes: u64,
    pub request_allocation: u64,
    pub request_nodes: u64,
    pub request_depth: u64,
    pub response_bytes: u64,
    pub response_allocation: u64,
    pub response_nodes: u64,
    pub response_depth: u64,
}

#[derive(Deserialize)]
pub(super) struct EndpointSchema {
    schema_version: u32,
    pub methods: Vec<EndpointEntry>,
    pub types: serde_json::Value,
}

pub(super) fn read_schema(bytes: &[u8]) -> Result<EndpointSchema, super::EndpointsCommandError> {
    let mut schema: EndpointSchema = serde_json::from_slice(bytes)?;
    if schema.schema_version != 1 || schema.methods.is_empty() || !schema.types.is_object() {
        return Err(super::EndpointsCommandError::Schema);
    }
    let mut names = std::collections::BTreeSet::new();
    let mut ids = std::collections::BTreeSet::new();
    for method in &schema.methods {
        let id = method
            .id
            .parse::<u64>()
            .map_err(|_| super::EndpointsCommandError::Schema)?;
        if id.to_string() != method.id || !names.insert(&method.name) || !ids.insert(id) {
            return Err(super::EndpointsCommandError::Schema);
        }
    }
    schema
        .methods
        .sort_by(|left, right| left.name.cmp(&right.name));
    Ok(schema)
}

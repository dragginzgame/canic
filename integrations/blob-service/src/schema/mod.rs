//! Native projection of mounted Blob codecs into the shared Canic registry.

use canic::endpoint::schema::{CompiledMethod, Method, MethodKind, MethodLimits};
use ic_blob_storage_contracts::{
    endpoints::{MethodMode, SERVICE_METHODS},
    wire::schema,
};
use ic_wire::{Limits, typescript::GenerationError};

/// Preserve the supplier's actual projections, including its tenant-specific layouts.
pub fn methods() -> Result<Vec<Method>, GenerationError> {
    let limits = |value: Limits| MethodLimits {
        bytes: value.bytes as u64,
        allocation: value.allocation as u64,
        nodes: value.nodes as u64,
        depth: value.depth as u64,
    };
    schema::methods()?
        .into_iter()
        .map(|mut projection| {
            let spec = SERVICE_METHODS
                .iter()
                .find(|spec| Some(spec.name) == projection["name"].as_str())
                .ok_or(GenerationError::Schema)?;
            let contract = spec.wire().map_err(|_| GenerationError::Schema)?;
            Ok(Method::project(
                CompiledMethod {
                    name: spec.name,
                    kind: match spec.mode {
                        MethodMode::Query => MethodKind::Query,
                        MethodMode::Update => MethodKind::Update,
                    },
                    id: contract.id,
                    request: limits(contract.request),
                    response: limits(contract.response),
                },
                projection["request"].take(),
                projection["response"].take(),
            ))
        })
        .collect()
}

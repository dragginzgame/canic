//! Module: fleet_ensure::ops::reinstall::source::view
//!
//! Responsibility: hash completed source bootstrap evidence without constructing an executable action.
//! Does not own: current protocol decoding, fixture defaults, replay or reset admission.
//! Boundary: the source inspector may use this projection only for an already Applied prefix row.

use crate::fleet_ensure::{json, ops::EnsureStateError};
use candid::Principal;
use canic_core::{
    cdk::utils::hash::sha256_hex,
    dto::root_store::{RootStoreBootstrapRequest, RootStoreCatalogEntry},
    ids::FleetSubnetRootReleaseSet,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Verify only the completed receipt shape that has no fixture declaration.
pub(super) fn completed_bootstrap_hash(action: &Value) -> Result<Option<String>, EnsureStateError> {
    if action.get("kind").and_then(Value::as_str) != Some("fleet_protocol")
        || action.pointer("/action/kind").and_then(Value::as_str) != Some("bootstrap_store")
        || action.pointer("/action/expected/fixtures").is_some()
    {
        return Ok(None);
    }
    let view: CompletedBootstrapView = serde_json::from_value(action.clone())
        .map_err(|_| EnsureStateError::InvalidActivationSource)?;
    let burn = view
        .maximum_execution_burn_cycles
        .parse::<u128>()
        .map_err(|_| EnsureStateError::InvalidActivationSource)?;
    if burn.to_string() != view.maximum_execution_burn_cycles {
        return Err(EnsureStateError::InvalidActivationSource);
    }
    let bytes = json::to_vec(&view).map_err(|_| EnsureStateError::InvalidActivationSource)?;
    Ok(Some(sha256_hex(&bytes)))
}

// Field order retains the source action digest contract. These private projections
// have no conversion into EnsureAction or the current bootstrap response.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CompletedBootstrapView {
    kind: String,
    action: CompletedBootstrapProtocolView,
    candid: String,
    candid_sha256: String,
    maximum_execution_burn_cycles: String,
    name: String,
    principal: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CompletedBootstrapProtocolView {
    kind: String,
    expected: CompletedBootstrapReceiptView,
    request: RootStoreBootstrapRequest,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CompletedBootstrapReceiptView {
    fleet_subnet_root: Principal,
    wasm_store: Principal,
    release_set: FleetSubnetRootReleaseSet,
    catalog: Vec<RootStoreCatalogEntry>,
}

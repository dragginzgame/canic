//! Passive ordinary inventory transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::{
    component_registry::{ComponentRegistryPartitionRequest, ComponentRegistryPartitionResponse},
    role::OperationStatusRequest,
    root::RootOperationStatusResponse,
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded Request wire projection for ordinary inventory.

#[derive(CandidType)]
pub enum Request {
    ComponentRegistryPartition(ComponentRegistryPartitionRequest),
    Operation(OperationStatusRequest),
}

/// Bounded Response wire projection for ordinary inventory.
#[derive(CandidType, Deserialize)]
pub enum Response {
    ComponentRegistryPartition(Box<ComponentRegistryPartitionResponse>),
    Operation(Box<RootOperationStatusResponse>),
}

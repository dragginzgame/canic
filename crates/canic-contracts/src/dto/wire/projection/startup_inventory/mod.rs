//! Passive startup inventory transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::component_registry::{
    ComponentDirectoryHead, ComponentDirectoryHeadRequest, ComponentRegistryPartitionRequest,
    ComponentRegistryPartitionResponse,
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded Request wire projection for startup inventory.

#[derive(CandidType)]
pub enum Request {
    ComponentDirectoryHead(ComponentDirectoryHeadRequest),
    ComponentRegistryPartition(ComponentRegistryPartitionRequest),
}

/// Bounded Response wire projection for startup inventory.
#[derive(CandidType, Deserialize)]
pub enum Response {
    ComponentDirectoryHead(Box<ComponentDirectoryHead>),
    ComponentRegistryPartition(Box<ComponentRegistryPartitionResponse>),
}

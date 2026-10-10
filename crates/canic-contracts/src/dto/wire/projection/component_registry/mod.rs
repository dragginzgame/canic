//! Passive component registry transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::{
    dto::{
        component_registry::ComponentRuntimeDirectoryPreparationRequest,
        role::{ComponentRuntimeOperationStatus, OperationStatusRequest},
        runtime::CanicReadinessStatus,
    },
    ids::ManagedCanisterBinding,
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded CanisterCommandFragment wire projection for component registry.

#[derive(CandidType)]
pub enum CanisterCommandFragment {
    ConfigureRuntime(ComponentRuntimeDirectoryPreparationRequest),
}

/// Bounded CanisterStatusRequestFragment wire projection for component registry.
#[derive(CandidType)]
pub enum CanisterStatusRequestFragment {
    Binding,
    Operation(OperationStatusRequest),
    Readiness,
}

/// Bounded CanisterStatusResponseFragment wire projection for component registry.
#[derive(CandidType, Deserialize)]
pub enum CanisterStatusResponseFragment {
    Binding(Box<ManagedCanisterBinding>),
    Operation(Box<CanisterOperationStatusFragment>),
    Readiness(CanicReadinessStatus),
}

/// Bounded CanisterOperationStatusFragment wire projection for component registry.
#[derive(CandidType, Deserialize)]
pub enum CanisterOperationStatusFragment {
    ConfigureRuntime(ComponentRuntimeOperationStatus),
}

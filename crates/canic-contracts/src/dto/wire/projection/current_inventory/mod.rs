//! Passive current inventory transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::{
    canister::CanisterInfo,
    component_provisioning::RootComponentProvisioningStatusResponse,
    component_registry::{
        ComponentRegistryActivePartitionRequest, ComponentRegistryActivePartitionResponse,
        RootComponentChildAllocationResponse,
    },
    page::{Page, PageRequest},
    pool::{CanisterPoolResponse, CanisterPoolStatusRequest},
    role::OperationStatusRequest,
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded ChildrenStatusRequest wire projection for current inventory.

#[derive(CandidType)]
pub enum ChildrenStatusRequest {
    Children(PageRequest),
}

/// Bounded ChildrenStatusResponse wire projection for current inventory.
#[derive(CandidType, Deserialize)]
pub enum ChildrenStatusResponse {
    Children(Page<CanisterInfo>),
}

/// Bounded RootInventoryStatusRequest wire projection for current inventory.
#[derive(CandidType)]
pub enum RootInventoryStatusRequest {
    ComponentChildProvisioning(OperationStatusRequest),
    ComponentRegistryActivePartition(ComponentRegistryActivePartitionRequest),
    ComponentProvisioning(OperationStatusRequest),
    Pool(CanisterPoolStatusRequest),
}

/// Bounded RootInventoryStatusResponse wire projection for current inventory.
#[derive(CandidType, Deserialize)]
pub enum RootInventoryStatusResponse {
    ComponentChildProvisioning(Box<RootComponentChildAllocationResponse>),
    ComponentRegistryActivePartition(Box<ComponentRegistryActivePartitionResponse>),
    ComponentProvisioning(Box<RootComponentProvisioningStatusResponse>),
    Pool(Box<CanisterPoolResponse>),
}

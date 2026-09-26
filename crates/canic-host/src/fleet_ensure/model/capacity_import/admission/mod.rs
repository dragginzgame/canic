//! Retained infrastructure provenance and explicit operator disposition evidence.

use candid::Principal;
use canic_core::ids::SubnetId;
use serde::{Deserialize, Serialize};

/// Exact installed infrastructure custody verified before a capacity operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportInfrastructureRecord {
    pub kind: CapacityImportInfrastructureKind,
    pub principal: Principal,
    pub subnet: SubnetId,
    pub controllers: Vec<Principal>,
    pub module_sha256: [u8; 32],
}

/// Current installed infrastructure role; Store custody names its exact Root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum CapacityImportInfrastructureKind {
    Coordinator,
    Root,
    Store { root: Principal },
}

/// Host admission evidence bound by the same immutable plan as every destructive effect.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportAdmissionRecord {
    pub infrastructure: Vec<CapacityImportInfrastructureRecord>,
    pub registry_candid_hex: String,
    pub declarations_toml: String,
    pub declarations_sha256: [u8; 32],
}

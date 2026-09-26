//! Read-only admission evidence supplied by the capacity-import observation owner.

use crate::fleet_ensure::model::capacity_import::{
    CapacityImportAuthority, CapacityImportSourceBinding,
};
use candid::Principal;
use std::collections::BTreeSet;

/// Complete destination inventory and current admission fences.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityImportDestinationView {
    pub authority: CapacityImportAuthority,
    pub ready: bool,
    pub draining: bool,
    pub competing_operation: bool,
    /// Includes every non-Store registered asset and any reserved creation slot.
    pub occupied_capacity: u32,
    pub maximum_capacity: u32,
    pub controlled_cycles: u128,
    pub reserved_cycles: u128,
    pub minimum_retained_cycles: u128,
    /// Complete known infrastructure and workload membership across the observed Fleet.
    pub assigned_canisters: BTreeSet<Principal>,
}

/// Whether the observation owner established eligibility for destructive enrollment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapacityImportOwnershipView {
    Unassigned,
    Assigned,
    Unknown,
}

/// Exact live candidate observation; unknown ownership must remain blocked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityImportSourceView {
    pub binding: CapacityImportSourceBinding,
    pub cycles: u128,
    pub reserved_cycles: u128,
    pub ownership: CapacityImportOwnershipView,
    /// Retained and verified absence/retirement evidence for this exact candidate.
    pub disposition_evidence_sha256: Option<[u8; 32]>,
}

/// Exact replacement bytes for one reviewed generator input.
/// Original bytes remain bound even when TOML formatting or comments change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityImportDocumentView {
    pub before_sha256: [u8; 32],
    pub after_sha256: [u8; 32],
    pub replacement: Vec<u8>,
}

/// A paired estate/policy projection; neither document may be published independently.
/// This carries no approval, filesystem intent or proof of Root completion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityImportInventoryView {
    pub plan_sha256: [u8; 32],
    pub policy: CapacityImportDocumentView,
    pub seed: CapacityImportDocumentView,
}

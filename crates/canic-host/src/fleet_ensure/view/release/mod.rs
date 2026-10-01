//! Fresh release-admission evidence supplied by the authenticated observation owner.
//!
//! These views are neither serialized declarations nor substitutes for IC observations.

use crate::fleet_ensure::{
    model::{
        capacity_import::CapacityImportSourceBinding,
        capacity_import::survey::CapacityImportSampleRecord,
        release::{FleetReleaseAccountRecord, FleetReleaseAuthority},
    },
    view::capacity_import::CapacityImportDestinationView,
};
use candid::Principal;

/// Authenticated physical sample after independent custody, without price or role assertions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetReleasePhysicalSourceView {
    pub sample: CapacityImportSampleRecord,
    pub snapshots: Vec<Vec<u8>>,
}

/// Physical and funding observations for one source, including its current call quote.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetReleaseSourceView {
    pub binding: CapacityImportSourceBinding,
    pub snapshots: Vec<Vec<u8>>,
    pub cycles: u128,
    pub reserved_cycles: u128,
    pub maximum_call_debit_cycles: u128,
    /// Complete operation-specific remaining path, including reconciliation allowances.
    pub required_paid_calls: u32,
}

/// One fenced owner's complete direct inventory and unresolved paid operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetReleaseOwnerView {
    pub owner: Principal,
    pub children: Vec<Principal>,
    pub producers_quiescent: bool,
    pub unresolved_operations: Vec<[u8; 32]>,
}

/// Exact current authority and externally retained inventory used for review admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetReleaseObservation {
    pub authority: FleetReleaseAuthority,
    pub sources: Vec<FleetReleaseSourceView>,
    /// Coordinator lists Roots; each Root lists its Store and all controlled children.
    pub owners: Vec<FleetReleaseOwnerView>,
    /// Complete observed account inventory, including zero-balance accounts.
    pub accounts: Vec<FleetReleaseAccountRecord>,
    pub destinations: Vec<CapacityImportDestinationView>,
}

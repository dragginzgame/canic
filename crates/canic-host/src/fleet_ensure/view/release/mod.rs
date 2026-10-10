//! Fresh release-admission evidence supplied by the authenticated observation owner.
//!
//! These views are neither serialized declarations nor substitutes for IC observations.

pub mod funding;
pub mod pool;
pub mod provisioning;
pub mod receipts;

use crate::fleet_ensure::{
    model::{
        capacity_import::{CapacityImportSourceBinding, survey::CapacityImportSampleRecord},
        release::{FleetReleaseAccountRecord, FleetReleaseAuthority},
    },
    view::capacity_import::CapacityImportDestinationView,
};
use candid::Principal;
use canic_contracts::dto::{
    fleet_coordinator::CoordinatorFundingStatusResponse,
    root::{RootFundingReleaseResponse, RootPoolReleaseResponse, RootProvisioningReleaseResponse},
};
use std::collections::{BTreeMap, BTreeSet};

/// Original shared replay pages from every selected Root and Coordinator; not settlement proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetReleaseReceiptsView {
    pub owners:
        BTreeMap<Principal, Vec<canic_contracts::dto::release_receipts::ReplayReleaseResponse>>,
}

/// Complete bounded provisioning discovery for the selected Roots, without settlement authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetReleaseProvisioningView {
    pub roots: Vec<FleetReleaseRootProvisioningView>,
}

/// Original journal pages from one Root; producers may still be active.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetReleaseRootProvisioningView {
    pub root: Principal,
    pub pages: Vec<RootProvisioningReleaseResponse>,
}

/// Exact pool obligations for each selected Root, without settlement or custody disposition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetReleasePoolView {
    pub roots: Vec<RootPoolReleaseResponse>,
}

/// Time-local Coordinator treasury and complete Root funding evidence, without settlement authority.
#[derive(Clone, Debug)]
pub struct FleetReleaseFundingView {
    pub coordinator: CoordinatorFundingStatusResponse,
    pub roots: Vec<FleetReleaseRootFundingView>,
}

/// Complete bounded refill history for one Root; producers may still be active.
#[derive(Clone, Debug)]
pub struct FleetReleaseRootFundingView {
    pub root: Principal,
    pub pages: Vec<RootFundingReleaseResponse>,
}

/// Complete queried ownership closure, with no assertion that producers are fenced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetReleaseInventoryView {
    pub children: BTreeMap<Principal, BTreeSet<Principal>>,
}

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

/// Complete original canonical accounting pages for each selected Root and Coordinator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetReleaseIntentsView {
    pub owners:
        BTreeMap<Principal, Vec<canic_contracts::dto::release_intents::IntentReleaseResponse>>,
}

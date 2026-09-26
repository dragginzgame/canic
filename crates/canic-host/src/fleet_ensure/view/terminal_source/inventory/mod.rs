//! Read-only source membership claims for completed-estate inspection.
//!
//! These facts seed live observation; they never establish live custody or reset authority.

pub(in crate::fleet_ensure) mod coordinator;
pub(in crate::fleet_ensure) mod evidence;
pub(in crate::fleet_ensure) mod ledger;
pub(in crate::fleet_ensure) mod membership;

use crate::{fleet_ensure::model::DesiredCanisterKind, protocol_binding::RegistryProtocolBinding};
use candid::Principal;
use canic_core::ids::{FleetBinding, ReleaseBuildId, SubnetId};
use std::collections::BTreeMap;

/// Local source evidence with every installed Candid bound to its finalized source release.
/// Transport must still verify the actual network, live module and controller authority.
#[derive(Debug)]
pub struct CompletedSourceInspectionView {
    pub inventory: CompletedEstateInventoryView,
    pub source_protocols: BTreeMap<String, crate::protocol_binding::ResolvedProtocolBinding>,
}

/// Locally cross-checked physical membership, bound to the audited source documents.
#[derive(Debug)]
pub struct CompletedEstateInventoryView {
    pub receipts: super::CompletedReceiptAuditView,
    pub fleet: FleetBinding,
    pub release_build_id: ReleaseBuildId,
    pub coordinator: Principal,
    pub coordinator_registry: coordinator::CompletedCoordinatorMembershipView,
    pub canisters: BTreeMap<String, CompletedCanisterInventoryView>,
    /// Historical terminal observations, never a replacement conservation baseline.
    pub recorded_controlled_canister_cycles: u128,
}

/// A source canister's recorded identity and parentage, still requiring live verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletedCanisterInventoryView {
    pub principal: Principal,
    pub subnet: SubnetId,
    pub kind: DesiredCanisterKind,
    pub parent: Option<String>,
    /// Physical pool ownership remains with this Root even when application parentage changes.
    pub root: Option<String>,
    pub module_sha256: Option<String>,
    pub protocol_binding: Option<RegistryProtocolBinding>,
    /// Original configured controllers; allocation can change these. They are not live authority.
    pub originally_declared_controllers: Vec<Principal>,
    pub recorded_cycles: u128,
}

///
/// CompletedCanisterCustodyView
///
/// One verified certificate's controller, code and subnet observations. This is
/// a time-local custody sample; it contains no cycle balance or wipe permission.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletedCanisterCustodyView {
    pub(in crate::fleet_ensure) principal: Principal,
    pub(in crate::fleet_ensure) subnet: SubnetId,
    pub(in crate::fleet_ensure) controllers: Vec<Principal>,
    pub(in crate::fleet_ensure) module_sha256: Option<String>,
    pub(in crate::fleet_ensure) certificate_tree_sha256: [u8; 32],
}

impl CompletedCanisterCustodyView {
    /// Physical identity authenticated by the certificate delegation.
    #[must_use]
    pub const fn principal(&self) -> Principal {
        self.principal
    }
    /// Observed subnet, independently checked against source placement.
    #[must_use]
    pub const fn subnet(&self) -> SubnetId {
        self.subnet
    }
    /// Exact observed controllers, including every additional controller.
    #[must_use]
    pub fn controllers(&self) -> &[Principal] {
        &self.controllers
    }
    /// Installed module, or certified absence for an empty canister.
    #[must_use]
    pub fn module_sha256(&self) -> Option<&str> {
        self.module_sha256.as_deref()
    }
    /// Certified tree root binds the observed fields and IC certificate time.
    #[must_use]
    pub const fn certificate_tree_sha256(&self) -> [u8; 32] {
        self.certificate_tree_sha256
    }
}

///
/// CompletedEstateCustodyView
///
/// All locally recorded physical IDs passed fresh certified custody checks under
/// the source signer/network. This does not prove there are no unrecorded assets.
///
#[derive(Debug)]
pub struct CompletedEstateCustodyView {
    pub(in crate::fleet_ensure) observed_at: std::time::Instant,
    pub(in crate::fleet_ensure) documents: crate::fleet_ensure::model::FleetTerminalSourceRecord,
    pub(in crate::fleet_ensure) network: canic_core::ids::CanonicalNetworkId,
    pub(in crate::fleet_ensure) operator: Principal,
    pub(in crate::fleet_ensure) canisters: BTreeMap<String, CompletedCanisterCustodyView>,
}

impl CompletedEstateCustodyView {
    /// Exact original byte identities rechecked after the last certified read.
    #[must_use]
    pub const fn documents(&self) -> &crate::fleet_ensure::model::FleetTerminalSourceRecord {
        &self.documents
    }
    /// IC trust-anchor identity matched before issuing any read.
    #[must_use]
    pub const fn network(&self) -> canic_core::ids::CanonicalNetworkId {
        self.network
    }
    /// Authenticated signer that directly controls the recorded Coordinator and Roots.
    #[must_use]
    pub const fn operator(&self) -> Principal {
        self.operator
    }
    /// Separate certificate sample for each recorded physical canister.
    #[must_use]
    pub const fn canisters(&self) -> &BTreeMap<String, CompletedCanisterCustodyView> {
        &self.canisters
    }
}

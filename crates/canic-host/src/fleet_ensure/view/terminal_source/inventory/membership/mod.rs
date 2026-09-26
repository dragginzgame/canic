//! Read-only Root membership samples, separate from fresh balance and reset authority.

use crate::fleet_ensure::{
    CompletedCoordinatorMembershipView, CompletedEstateCustodyView, CompletedLedgerBalancesView,
    model::DesiredCanisterKind,
};
use candid::Principal;
use canic_core::ids::{ComponentInstanceId, FleetSubnetCanisterPoolConfig};
use std::collections::BTreeMap;

/// One Root's complete sampled membership matched to the completed source IDs.
/// Cached pool balances are deliberately excluded: they are not fresh management observations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletedRootMembershipView {
    pub(in crate::fleet_ensure) root: Principal,
    pub(in crate::fleet_ensure) config: FleetSubnetCanisterPoolConfig,
    pub(in crate::fleet_ensure) completed_handoffs: u64,
    pub(in crate::fleet_ensure) assets: BTreeMap<Principal, CompletedPoolAssetView>,
}

impl CompletedRootMembershipView {
    /// Exact Root whose protected query returned these assets.
    #[must_use]
    pub const fn root(&self) -> Principal {
        self.root
    }

    /// Pool policy returned consistently on every page.
    #[must_use]
    pub const fn config(&self) -> &FleetSubnetCanisterPoolConfig {
        &self.config
    }

    /// Durable completed handoff counter returned consistently on every page.
    #[must_use]
    pub const fn completed_handoffs(&self) -> u64 {
        self.completed_handoffs
    }

    /// Sampled pool assets, including Store and application descendants.
    #[must_use]
    pub const fn assets(&self) -> &BTreeMap<Principal, CompletedPoolAssetView> {
        &self.assets
    }
}

/// A sampled terminal pool role and its optional durable Workload allocation identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletedPoolAssetView {
    pub(in crate::fleet_ensure) kind: DesiredCanisterKind,
    pub(in crate::fleet_ensure) allocation: Option<CompletedWorkloadAllocationView>,
}

/// Exact Component allocation reported by Root for an application canister.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletedWorkloadAllocationView {
    pub component: ComponentInstanceId,
    pub operation_id: [u8; 32],
}

impl CompletedPoolAssetView {
    /// Store, idle Pool or application Component role, matched to local source evidence.
    #[must_use]
    pub const fn kind(&self) -> DesiredCanisterKind {
        self.kind
    }

    /// Root's sampled allocation identity; application parentage is verified separately.
    #[must_use]
    pub const fn allocation(&self) -> Option<&CompletedWorkloadAllocationView> {
        self.allocation.as_ref()
    }
}

/// Bounded read-only enumeration, bracketed by matching certified physical custody passes.
/// This sample is not an atomic snapshot, a mutation fence or permission to wipe state.
#[derive(Debug)]
pub struct CompletedEstateMembershipView {
    pub(in crate::fleet_ensure) custody: CompletedEstateCustodyView,
    pub(in crate::fleet_ensure) coordinator: CompletedCoordinatorMembershipView,
    pub(in crate::fleet_ensure) roots: BTreeMap<String, CompletedRootMembershipView>,
    pub(in crate::fleet_ensure) ledger: CompletedLedgerBalancesView,
}

impl CompletedEstateMembershipView {
    /// Coordinator Root rows matched before and after the Root pool survey.
    #[must_use]
    pub const fn coordinator(&self) -> &CompletedCoordinatorMembershipView {
        &self.coordinator
    }

    /// Final certified sample; its bindings match those observed before enumeration.
    #[must_use]
    pub const fn custody(&self) -> &CompletedEstateCustodyView {
        &self.custody
    }

    /// Every recorded Root's complete pool membership at the time of its queries.
    #[must_use]
    pub const fn roots(&self) -> &BTreeMap<String, CompletedRootMembershipView> {
        &self.roots
    }

    /// Fresh default Ledger account samples reconciled against original receipts.
    #[must_use]
    pub const fn ledger(&self) -> &CompletedLedgerBalancesView {
        &self.ledger
    }
}

//! Coordinator identity and Root membership observations, without executable Registry authority.

use candid::Principal;
use canic_core::{
    dto::fleet_registry::FleetSubnetRootEntry,
    ids::{FleetBinding, SubnetId},
};

/// Exact source identity, revision and Root rows. Recovery controller declarations
/// are not projected or defaulted; physical custody is observed independently.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletedCoordinatorMembershipView {
    pub(in crate::fleet_ensure) fleet: FleetBinding,
    pub(in crate::fleet_ensure) coordinator: Principal,
    pub(in crate::fleet_ensure) coordinator_subnet: SubnetId,
    pub(in crate::fleet_ensure) epoch: u64,
    pub(in crate::fleet_ensure) revision: u64,
    pub(in crate::fleet_ensure) roots: Vec<FleetSubnetRootEntry>,
}

impl CompletedCoordinatorMembershipView {
    /// Registry epoch and revision matched to exact completed source evidence.
    #[must_use]
    pub const fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Registry revision matched to exact completed source evidence.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// All source Root rows, including placement, release, policy and lifecycle status.
    #[must_use]
    pub fn roots(&self) -> &[FleetSubnetRootEntry] {
        &self.roots
    }
}

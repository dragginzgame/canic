//! Module: view::root_funding
//!
//! Responsibility: carry validated Root-funding authority and acceptance dispositions.
//! Does not own: durable records, policy derivation, cycle acceptance, or endpoint auth.
//! Boundary: Root workflow derives authority; Root ops return one fresh-or-replay decision.

use candid::Principal;
use canic_core::{
    dto::{
        fleet_funding::FleetRootFundingAcceptanceReceipt,
        fleet_registry::{FleetRegistryVersion, FleetSubnetRootStatus},
    },
    ids::FleetSubnetRootFundingAuthority,
};

/// Minimal protected and Registry-derived authority for one Root funding operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootFundingAuthorityView {
    pub registry: FleetRegistryVersion,
    pub fleet_subnet_root: Principal,
    pub status: FleetSubnetRootStatus,
    pub funding_eligible: bool,
    pub funding: FleetSubnetRootFundingAuthority,
}

/// Protected inputs consumed by the sole Root cycle-top-up scheduler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootFundingScheduleView {
    pub request_threshold: u128,
    pub cooldown_secs: u64,
}

/// Whether an exact Coordinator acceptance call may accept once or must replay at zero.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RootFundingAcceptanceDisposition {
    Fresh,
    Replay(Box<FleetRootFundingAcceptanceReceipt>),
}

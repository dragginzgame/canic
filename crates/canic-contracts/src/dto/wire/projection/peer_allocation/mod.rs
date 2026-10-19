//! Bounded peer-allocation transport for framework validation canisters.
//!
//! Keep only the allocation selector and acceptance receipt so these probes do
//! not link unrelated Root commands or recursive operation status payloads.

use crate::dto::component_registry::RootPeerComponentAllocationRequest;
use ic_platform::candid::contract::CandidType;

/// Request a peer from the owning Root.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootCommand {
    ProvisionPeer(RootPeerComponentAllocationRequest),
}

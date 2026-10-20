//! Module: ids::application_checkpoint
//!
//! Responsibility: identify one application participant in a Root-issued checkpoint.
//! Boundary: identity data supplies no freeze, snapshot or spending authority.

use crate::ids::CallerReceiverAuthority;
use ic_platform::candid::contract::CandidType;
use serde::{Deserialize, Serialize};

/// Exact installed audience and monotonic Root-issued identity of one checkpoint.
///
/// Root owns sequence issuance outside application snapshots. The unit digest
/// commits complete semantic membership and the selected application contract.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
#[serde(deny_unknown_fields)]
pub struct ApplicationCheckpointBinding {
    pub authority: CallerReceiverAuthority,
    pub operation_id: [u8; 32],
    pub sequence: u64,
    pub unit_digest: [u8; 32],
}

/// Canonical participant phase shared by durable state and wire acknowledgements.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum ApplicationCheckpointPhase {
    Preparing,
    Prepared,
    Releasing,
    Released,
}

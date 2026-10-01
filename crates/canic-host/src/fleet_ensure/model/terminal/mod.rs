//! Durable final accounting for exact completed Ensure replay, including clean reinstall.

use crate::fleet_ensure::model::ActualCycleConservation;
use serde::{Deserialize, Serialize};

/// Exact local completion evidence, published before the final journal transition.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetEnsureTerminalRecord {
    pub schema_version: u16,
    pub plan_sha256: String,
    pub journal_sha256: [u8; 32],
    pub state_sha256: [u8; 32],
    pub clean_reinstall_selection_sha256: Option<[u8; 32]>,
    pub actual: ActualCycleConservation,
}

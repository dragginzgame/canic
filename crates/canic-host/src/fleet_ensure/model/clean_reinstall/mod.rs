//! Current replacement selection, independent of completed release schemas.

use crate::fleet_ensure::model::ReviewedDesiredFleetRecord;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Frozen current build and explicit operator inventory used by every reset phase.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CleanReinstallRecord {
    pub schema_version: u16,
    pub desired: ReviewedDesiredFleetRecord,
    pub policy: PathBuf,
    pub seed: PathBuf,
}

/// Exact locally replayable completion, published before the final journal transition.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CleanReinstallTerminalRecord {
    pub schema_version: u16,
    pub plan_sha256: String,
    pub journal_sha256: [u8; 32],
    pub state_sha256: [u8; 32],
    pub selection_sha256: [u8; 32],
    pub actual: crate::fleet_ensure::model::ActualCycleConservation,
}

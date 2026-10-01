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

/// Exact cancellation intent and receipt for an unapproved local reset review.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CleanReinstallCancellationRecord {
    pub schema_version: u16,
    pub environment: String,
    pub fleet: String,
    pub operation_id: String,
    pub plan_sha256: String,
    pub archive_sha256: String,
}

//! Supplementary Host attempt authority; original operation and spending records remain immutable.

use serde::{Deserialize, Serialize};

/// Digest-approved finite continuation for one exact retained counter.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptRecoveryGrantRecord {
    pub binding_sha256: [u8; 32],
    pub resource: String,
    pub spent_attempts: u32,
    pub previous_maximum: u32,
    pub additional_attempts: u32,
    pub grant_sha256: [u8; 32],
}

/// Exact owner bytes reviewed before an effect-free local allowance extension.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptRecoveryOwnerRecord {
    pub relative_path: String,
    pub before_sha256: [u8; 32],
    pub grants: Vec<AttemptRecoveryGrantRecord>,
}

/// One review binds the Fleet and every exhausted resource; approval submits no IC call.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptRecoveryReviewRecord {
    pub schema_version: u16,
    pub environment: String,
    pub fleet: String,
    pub owners: Vec<AttemptRecoveryOwnerRecord>,
    pub review_sha256: [u8; 32],
}

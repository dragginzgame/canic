//! Immutable evidence that an exact management ingress can no longer execute.

use crate::fleet_ensure::model::capacity_import::CapacityImportHandoffRequestRecord;
use serde::{Deserialize, Serialize};

/// Authenticated terminal status retained before inspecting custody or renewing ingress.
/// Digests identify observed certificates; they do not authenticate local bytes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportHandoffRetirementRecord {
    pub request: CapacityImportHandoffRequestRecord,
    pub reason: CapacityImportHandoffRetirementReason,
    pub certificate_sha256: [u8; 32],
}

/// Certified rejection, pruned completion, or absence after the request's expiry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum CapacityImportHandoffRetirementReason {
    Rejected {
        reject_code: u8,
        reject_message_sha256: [u8; 32],
    },
    Done,
    Absent {
        certified_at_ns: u64,
    },
}

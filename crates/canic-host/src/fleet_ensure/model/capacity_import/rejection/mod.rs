//! Immutable local receipts for certified rejected management ingress.

use crate::fleet_ensure::model::capacity_import::CapacityImportHandoffRequestRecord;
use serde::{Deserialize, Serialize};

/// Authenticated rejection retained before any new ingress may be prepared.
/// Digests identify the observed certificate and message; they do not authenticate local bytes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportHandoffRejectionRecord {
    pub request: CapacityImportHandoffRequestRecord,
    pub reject_code: u8,
    pub reject_message_sha256: [u8; 32],
    pub certificate_sha256: [u8; 32],
}

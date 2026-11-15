//! Bounded metadata enumeration under one closed admission attempt. No payload integrity claim.
use crate::{Custody, MaintenanceAttempt, Quota, Release};
use serde::{Deserialize, Serialize};

/// Caller must own the exact currently closed attempt on every page.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Request {
    /// Exact closure, never inferred from current state.
    pub attempt: MaintenanceAttempt,
    /// Previous returned release; None starts enumeration.
    pub after: Option<Release>,
}
/// Stable metadata counters, deliberately excluding executable identity and cache state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Catalog {
    /// Configured limits, including incomplete releases.
    pub quota: Quota,
    /// Reserved payload capacity, not physical backing usage.
    pub reserved_bytes: u64,
    /// Retained release count, used to detect incomplete enumeration.
    pub releases: u64,
    /// Distinct artifacts shared by those releases.
    pub artifacts: u64,
    /// Distinct retained payload chunks; this read does not hash their bytes.
    pub chunks: u64,
}
/// One complete release record and its shared pins; None establishes end of enumeration.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Page {
    /// Must remain identical for every page under the selected closure.
    pub catalog: Catalog,
    /// At most one release, including its incomplete-upload progress and former-owner pins.
    pub entry: Option<Custody>,
}

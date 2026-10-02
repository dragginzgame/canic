//! Immutable historical operation evidence, independent of its retired execution contract.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Byte identities for retired local authority; never executable replacement authority.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationArchiveRecord {
    pub schema_version: u16,
    pub environment: String,
    pub fleet: String,
    /// Present only when an admitted historical envelope established these identities.
    pub operation_id: Option<String>,
    pub plan_sha256: Option<String>,
    pub files: BTreeMap<String, String>,
    /// Unavailable historical content is recorded, never reconstructed or executed.
    pub unavailable_objects: BTreeSet<String>,
}

/// Durable local retirement intent, bound to already qualified replacement authority.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedOperationRetirementRecord {
    pub schema_version: u16,
    pub environment: String,
    pub fleet: String,
    pub archive_sha256: String,
    pub replacement_sha256: String,
}

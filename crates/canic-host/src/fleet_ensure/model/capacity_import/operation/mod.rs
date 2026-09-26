//! Durable operator review, finite call budgets and paired inventory publication.
//!
//! Root owns canister reset. These records bind local inputs and retain host progress.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Exact original and replacement bytes for one workspace-owned generator input.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportDocumentRecord {
    pub relative_path: String,
    pub original: String,
    pub replacement: String,
    pub before_sha256: [u8; 32],
    pub after_sha256: [u8; 32],
}

/// Complete operator approval, including all local writes and maximum host submissions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportOperationReviewRecord {
    pub schema_version: u16,
    pub publication_kind: CapacityImportPublicationKind,
    pub plan_sha256: [u8; 32],
    pub environment: String,
    pub fleet: String,
    pub policy: CapacityImportDocumentRecord,
    pub seed: CapacityImportDocumentRecord,
    pub maximum_submissions_per_step: u32,
    pub maximum_management_observations_per_canister: u32,
    pub review_sha256: [u8; 32],
}

/// Whether reviewed inventory adds capacity or publishes an explicitly held initial source set.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityImportPublicationKind {
    ExtendEstate,
    InitializeEstate,
}

/// Monotonic progress survives process loss and preserves terminal replay after allocation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportOperationRecord {
    pub review: CapacityImportOperationReviewRecord,
    pub submissions: BTreeMap<String, u32>,
    pub inspections: BTreeMap<String, u32>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub settled_status_candid_hex: Option<String>,
    pub publication_started: bool,
    pub publication_complete: bool,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub released_status_candid_hex: Option<String>,
}

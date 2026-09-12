//! Module: storage::stable::fleet_activation::fixture
//!
//! Responsibility: persist the immutable source assignment inside the activation owner.
//! Does not own: application rows, import progress, receipts or Store grant mutation.
//! Boundary: ops converts and validates the complete assignment before persistence.

use crate::ids::{ManagedCanisterBinding, ReleaseBuildId};
use candid::Principal;
use serde::{Deserialize, Serialize};

/// Protected source authority for one exact managed installation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureAssignmentRecord {
    pub store: Principal,
    pub target: ManagedCanisterBinding,
    pub installation: [u8; 32],
    pub release_build_id: ReleaseBuildId,
    pub content_id: [u8; 32],
    pub grant_revision: u64,
    pub grant_enabled: bool,
    pub schema_version: u16,
    pub format_hash: [u8; 32],
    pub encoded_length: u64,
    pub chunks: Vec<FixtureChunkRecord>,
    pub completion_summary: [u8; 32],
}

/// Ordered digest and length of one independently decodable source chunk.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureChunkRecord {
    pub digest: [u8; 32],
    pub length: u32,
}

//! Passive fixture content, Store publication and exact target grant contracts.

use crate::ids::{ManagedCanisterBinding, ReleaseBuildId};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

/// One independently decodable opaque chunk selected by an application descriptor.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureChunkDescriptor {
    pub digest: [u8; 32],
    pub length: u32,
}

/// Release-independent fixture content; release authority binds its resulting digest.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureDescriptor {
    pub schema_version: u16,
    pub format_hash: [u8; 32],
    pub encoded_length: u64,
    pub chunks: Vec<FixtureChunkDescriptor>,
    pub completion_summary: [u8; 32],
}

/// One bounded, sequential Store upload; exact earlier chunks may be replayed.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureChunkUpload {
    pub content_id: [u8; 32],
    pub index: u32,
    pub bytes: Vec<u8>,
}

/// Durable source-byte progress; completion is not an application data receipt.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureSourceStatus {
    pub content_id: [u8; 32],
    pub next_chunk: u32,
    pub chunk_count: u32,
    pub received_bytes: u64,
    pub complete: bool,
}

/// Root-derived installation authority independent of reusable content identity.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureTargetBinding {
    pub target: ManagedCanisterBinding,
    pub installation: [u8; 32],
    pub release_build_id: ReleaseBuildId,
    pub content_id: [u8; 32],
}

/// Compare-and-set read-grant intent; stale grant or revoke messages cannot replace it.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureGrantRequest {
    pub expected_revision: u64,
    pub binding: FixtureTargetBinding,
    pub enabled: bool,
}

/// Current target authority, including revoked revisions retained against stale replay.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureGrant {
    pub revision: u64,
    pub binding: FixtureTargetBinding,
    pub enabled: bool,
}

/// Authenticated target pull, pinned to the exact granted revision and content.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureChunkRead {
    pub grant: FixtureGrant,
    pub index: u32,
}

/// Closed Store outcomes for deterministic caller retry and conflict handling.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FixtureStoreError {
    Authority,
    Bounds,
    Capacity,
    Conflict,
    Content,
    NotFound,
    NotReady,
    Sequence,
}

/// Immutable source selection installed by Root before the target can import data.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureAssignment {
    pub store: Principal,
    pub grant: FixtureGrant,
    pub descriptor: FixtureDescriptor,
}

/// Application-owned durable evidence of validated data for one exact installation.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureImportReceipt {
    pub binding: FixtureTargetBinding,
    pub completion_summary: [u8; 32],
}

/// Read-only projection of the application's sole durable import checkpoint.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureImportProgress {
    pub binding: FixtureTargetBinding,
    pub next_chunk: u32,
    pub receipt: Option<Box<FixtureImportReceipt>>,
}

/// Data prerequisite observed from protected selection and application-owned evidence.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FixtureProvisioningStatus {
    NotRequired,
    AwaitingImporter,
    Pending(Option<Box<FixtureImportProgress>>),
    Complete(Box<FixtureImportReceipt>),
    Failed(FixtureImportFailure),
}

/// Closed consumer failures preserve source, transport and application diagnostic owners.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FixtureImportError {
    Application { code: u32 },
    Authority,
    Busy,
    Codec(crate::dto::error::Error),
    ImporterMissing,
    NotReady,
    Progress,
    Receipt,
    Registration,
    Runtime(crate::dto::error::Error),
    Source(FixtureStoreError),
    SourceRejected(crate::dto::error::Error),
    Transport(crate::dto::error::Error),
}

/// Durable failure diagnostics retain the originating application or runtime owner.
pub use crate::domain::fixture_import::FixtureImportFailure;

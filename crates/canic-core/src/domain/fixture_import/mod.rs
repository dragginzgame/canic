//! Module: domain::fixture_import
//!
//! Responsibility: identify permanent import failures without transport or storage dependencies.
//! Does not own: callbacks, retries, records or application progress.
//! Boundary: concrete diagnostic codes retain the originating owner.

use candid::CandidType;
use serde::{Deserialize, Serialize};

/// Durable reason why automatic import requires review instead of another attempt.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FixtureImportFailure {
    Application { code: u32 },
    Authority,
    Codec { code: u16 },
    Progress,
    Receipt,
    Registration,
    Runtime { code: u16 },
    SourceAuthority,
    SourceBounds,
    SourceCapacity,
    SourceConflict,
    SourceContent,
    SourceNotFound,
    SourceSequence,
    SourceRejected { code: u16 },
}

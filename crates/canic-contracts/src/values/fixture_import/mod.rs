//! Module: domain::fixture_import
//!
//! Responsibility: identify permanent import failures without transport or storage dependencies.
//! Does not own: callbacks, retries, records or application progress.
//! Boundary: concrete diagnostic codes retain the originating owner.

use serde::{Deserialize, Serialize};

/// Durable reason why automatic import requires review instead of another attempt.

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
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

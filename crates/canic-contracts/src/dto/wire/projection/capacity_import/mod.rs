//! Passive capacity import transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::pool_import::{
    PoolImportCommand, PoolImportContext, PoolImportIdentity, PoolImportStatus,
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded Command wire projection for capacity import.

#[derive(CandidType)]
pub enum Command {
    ImportPoolCapacity(PoolImportCommand),
}

/// Bounded Response wire projection for capacity import.
#[derive(CandidType, Deserialize)]
pub enum Response {
    ImportPoolCapacity(PoolImportStatus),
}

/// Bounded StatusRequest wire projection for capacity import.
#[derive(CandidType, Deserialize)]
pub enum StatusRequest {
    PoolImport(PoolImportIdentity),
    PoolImportContext,
}

/// Bounded StatusResponse wire projection for capacity import.
#[derive(CandidType, Deserialize)]
pub enum StatusResponse {
    PoolImport(Box<PoolImportStatus>),
    PoolImportContext(Box<PoolImportContext>),
}

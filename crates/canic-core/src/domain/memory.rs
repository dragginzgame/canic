//! Module: domain::memory
//!
//! Responsibility: define pure memory diagnostic value enums shared by memory
//! ops and memory DTOs.
//! Does not own: memory response DTO structs, stable memory records, or memory
//! runtime mutation.
//! Boundary: DTOs re-export these values to preserve the public API path while
//! internal code imports them from the domain owner.

use candid::CandidType;
use serde::Deserialize;

/// Source of a measured ID's binding; unknown does not mean unused.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum MemoryAllocationBinding {
    Current { stable_key: String, owner: String },
    Ledger { stable_key: String, owner: String },
    Unknown,
}

///
/// MemoryCommitRecoveryErrorResponse
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum MemoryCommitRecoveryErrorResponse {
    NoValidGeneration,
    InvalidCommitSlots,
    AmbiguousGeneration,
    GenerationOverflow,
    UnexpectedGeneration,
    Unknown,
}

///
/// MemoryRangeAuthorityMode
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum MemoryRangeAuthorityMode {
    Reserved,
    Allowed,
}

///
/// MemoryAllocationState
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum MemoryAllocationState {
    Reserved,
    Active,
    Retired,
}

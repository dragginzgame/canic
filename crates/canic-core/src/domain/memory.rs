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

///
/// MemoryCommitRecoveryErrorResponse
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum MemoryCommitRecoveryErrorResponse {
    NoValidGeneration,
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

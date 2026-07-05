//! Module: domain::canister
//!
//! Responsibility: define pure canister status/settings value enums shared by
//! management ops and endpoint DTOs.
//! Does not own: IC management-call infra payloads, canister records, endpoint
//! DTO structs, or lifecycle orchestration.
//! Boundary: ops and DTO modules re-export these values to preserve public API
//! paths while conversion from foreign infra payloads remains explicit.

use crate::cdk::{candid::CandidType, types::Principal};
use serde::Deserialize;

///
/// CanisterStatusType
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum CanisterStatusType {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "stopped")]
    Stopped,
}

///
/// LogVisibility
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum LogVisibility {
    #[serde(rename = "controllers")]
    Controllers,
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "allowed_viewers")]
    AllowedViewers(Vec<Principal>),
}

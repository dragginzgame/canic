//! Module: domain::http
//!
//! Responsibility: define pure HTTP value enums shared by HTTP ops, runtime
//! metrics, and boundary DTOs.
//! Does not own: raw IC management HTTP payloads, HTTP request/response DTO
//! structs, metrics storage, or workflow retry policy.
//! Boundary: ops, metrics, and DTO modules re-export these values to preserve
//! public API paths while conversion from foreign infra payloads remains
//! explicit.

use candid::CandidType;
use serde::Deserialize;

///
/// HttpMethod
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
pub enum HttpMethod {
    #[serde(rename = "get")]
    Get,
    #[serde(rename = "head")]
    Head,
    #[serde(rename = "post")]
    Post,
}

impl HttpMethod {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Post => "POST",
        }
    }
}

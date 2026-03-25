use crate::dto::prelude::*;

///
/// Page
/// Generic pagination envelope
///

#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct Page<T> {
    pub entries: Vec<T>,
    pub total: u64,
}

///
/// PageRequest
/// Pagination envelope to avoid passing raw integers around
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct PageRequest {
    pub limit: u64,
    pub offset: u64,
}

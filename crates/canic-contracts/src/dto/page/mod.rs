//
// Page
//
// Pagination envelope.
//

use crate::dto::prelude::*;

#[derive(CandidType, Clone, Debug, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct Page<T> {
    pub entries: Vec<T>,
    pub total: u64,
}

//
// PageRequest
//
// Pagination request.
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[candid_path("ic_platform::candid::contract")]
pub struct PageRequest {
    pub limit: u64,
    pub offset: u64,
}

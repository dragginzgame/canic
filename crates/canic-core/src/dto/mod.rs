use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// Page
/// Generic pagination envelope
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct Page<T> {
    pub entries: Vec<T>,
    pub total: u64,
}

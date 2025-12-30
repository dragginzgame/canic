use crate::dto::prelude::*;

///
/// Page
/// Generic pagination envelope
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct Page<T> {
    pub entries: Vec<T>,
    pub total: u64,
}

///
/// PageRequest
/// Pagination envelope to avoid passing raw integers around
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PageRequest {
    pub limit: u64,
    pub offset: u64,
}

impl PageRequest {
    pub const MAX_LIMIT: u64 = 1_000;

    #[must_use]
    pub const fn new(limit: u64, offset: u64) -> Self {
        Self { limit, offset }
    }

    #[must_use]
    pub fn bounded(limit: u64, offset: u64) -> Self {
        let limit = limit.min(Self::MAX_LIMIT);

        Self { limit, offset }
    }

    #[must_use]
    pub fn clamped(self) -> Self {
        Self::bounded(self.limit, self.offset)
    }
}

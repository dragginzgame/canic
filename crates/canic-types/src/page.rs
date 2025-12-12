use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// PageRequest
/// Common pagination envelope to avoid passing raw integers around.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PageRequest {
    pub limit: u64,
    pub offset: u64,
}

impl PageRequest {
    pub const MAX_LIMIT: u64 = 1_000;
    pub const DEFAULT: Self = Self {
        limit: 50,
        offset: 0,
    };

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

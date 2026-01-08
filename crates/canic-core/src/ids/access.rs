use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// AccessMetricKind
/// Enumerates the access-control stage that rejected the call.
///

#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[remain::sorted]
pub enum AccessMetricKind {
    Auth,
    Guard,
    Rule,
}

//! Shared log severity value; logging and rendering remain runtime-owned.

use ic_platform::candid::contract::CandidType;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, CandidType, Deserialize, Serialize,
)]
#[candid_path("ic_platform::candid::contract")]
pub enum Level {
    Debug,
    Info,
    Ok,
    Warn,
    Error,
}

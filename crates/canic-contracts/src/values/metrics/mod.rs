//! Passive protocol classifications; runtime decisions remain with Core.

use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

#[derive(CandidType, Clone, Copy, Debug, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
#[remain::sorted]
pub enum MetricsKind {
    Core,
    Placement,
    Platform,
    Runtime,
    Security,
    Storage,
}

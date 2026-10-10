//! Passive protocol classifications; runtime decisions remain with Core.

use candid::CandidType;
use serde::Deserialize;

#[derive(CandidType, Clone, Copy, Debug, Deserialize)]
#[remain::sorted]
pub enum MetricsKind {
    Core,
    Placement,
    Platform,
    Runtime,
    Security,
    Storage,
}

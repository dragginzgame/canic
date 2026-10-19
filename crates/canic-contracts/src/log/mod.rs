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
impl Level {
    #[must_use]
    pub const fn ansi_label(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "\x1b[34mINFO \x1b[0m",
            Self::Ok => "\x1b[32m OK  \x1b[0m",
            Self::Warn => "\x1b[33mWARN \x1b[0m",
            Self::Error => "\x1b[31mERROR\x1b[0m",
        }
    }
}

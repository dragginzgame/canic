//! Shared log severity value; logging and rendering remain runtime-owned.

use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, CandidType, Deserialize, Serialize,
)]
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

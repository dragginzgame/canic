pub mod auth;
pub mod guard;
pub mod helper;
pub mod ic;
pub mod interface;
pub mod macros;
pub mod memory;
pub mod serialize;
pub mod state;

pub mod export {
    pub use defer;
}

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

///
/// Prelude
///

pub mod prelude {
    pub use crate::{
        Error as IcuError, Log, auth_require_all, auth_require_any,
        guard::{guard_query, guard_update},
        ic::{
            api::msg_caller, candid::CandidType, export_candid,
            icrc_ledger_types::icrc1::account::Account, init, ledger_types::Subaccount,
            principal::Principal, query, update,
        },
        icu_register_memory, icu_start, icu_start_root, log, perf, perf_start,
    };
}

///
/// Error
///

#[derive(CandidType, Debug, Deserialize, Serialize, ThisError)]
pub enum Error {
    #[error(transparent)]
    AuthError(#[from] auth::AuthError),

    #[error(transparent)]
    InterfaceError(#[from] interface::InterfaceError),

    #[error(transparent)]
    MemoryError(#[from] memory::MemoryError),

    #[error(transparent)]
    StateError(#[from] state::StateError),
}

///
/// MemoryIds
///

pub const MEMORY_REGISTRY_ID: u8 = 0;

///
/// Log
///

pub enum Log {
    Ok,
    Perf,
    Info,
    Warn,
    Error,
}

///
/// CYCLES
///

pub const KC: u128 = 1_000;
pub const MC: u128 = 1_000_000;
pub const BC: u128 = 1_000_000_000;
pub const TC: u128 = 1_000_000_000_000;
pub const QC: u128 = 1_000_000_000_000_000;

///
/// Cycles
///

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Cycles(pub u128);

impl From<u128> for Cycles {
    fn from(n: u128) -> Self {
        Self(n)
    }
}

// parse_cycles
// helper function to parse string with multiplier suffix
pub fn parse_cycles(value: &str) -> Result<u128, String> {
    let mut num_str = String::new();
    let mut suffix_str = String::new();
    let mut seen_dot = false;

    for c in value.chars() {
        if c.is_ascii_digit() || (c == '.' && !seen_dot) {
            if c == '.' {
                seen_dot = true;
            }
            num_str.push(c);
        } else {
            suffix_str.push(c);
        }
    }

    let number: f64 = num_str.parse::<f64>().map_err(|e| e.to_string())?;

    let multiplier = match suffix_str.as_str() {
        "K" => 1_000_f64,
        "M" => 1_000_000_f64,
        "B" => 1_000_000_000_f64,
        "T" => 1_000_000_000_000_f64,
        "Q" => 1_000_000_000_000_000_f64,
        _ => 1_f64,
    };

    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    Ok((number * multiplier) as u128)
}

///
/// Instructions
///

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn format_instructions(n: u64) -> String {
    if n >= 1_000_000_000_000 {
        format!("{:.2}T", n as f64 / 1_000_000_000_000.0)
    } else if n >= 1_000_000_000 {
        format!("{:.2}B", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.2}K", n as f64 / 1_000.0)
    } else {
        format!("{n}")
    }
}

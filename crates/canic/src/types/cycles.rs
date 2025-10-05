//!
//! Cycle-denominated helpers covering human-friendly parsing, config shorthands,
//! and arithmetic wrappers used by ops modules.
//! The constants offer readable units (K/M/B/T/Q) while `Cycles` wraps `Nat`
//! with derived math traits and serde helpers.
//!

use candid::{CandidType, Nat};
use derive_more::{Add, AddAssign, Sub, SubAssign};
use num_traits::cast::ToPrimitive;
use serde::{Deserialize, Serialize, de::Deserializer};
use std::{
    fmt::{self, Display},
    str::FromStr,
};

///
/// Constants
/// Cycle unit shorthands for configs and logs
///

pub const KC: u128 = 1_000;
pub const MC: u128 = 1_000_000;
pub const BC: u128 = 1_000_000_000;
pub const TC: u128 = 1_000_000_000_000;
pub const QC: u128 = 1_000_000_000_000_000;

///
/// Cycles
/// Thin wrapper around `Nat` that carries helper traits and serializers for
/// arithmetic on cycle balances.
///

#[derive(
    Add,
    AddAssign,
    CandidType,
    Clone,
    Default,
    Debug,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    SubAssign,
    Sub,
)]
pub struct Cycles(Nat);

impl Cycles {
    #[must_use]
    pub fn new(amount: u128) -> Self {
        Self(amount.into())
    }

    #[must_use]
    pub fn to_u128(&self) -> u128 {
        self.0.0.to_u128().unwrap_or(u128::MAX)
    }

    // from_config
    // accepts the short hand 10T format or a number
    pub fn from_config<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Str(String),
            Num(u128),
        }

        match Helper::deserialize(deserializer)? {
            Helper::Str(s) => s.parse::<Self>().map_err(serde::de::Error::custom),
            Helper::Num(n) => Ok(Self::new(n)),
        }
    }
}

#[allow(clippy::cast_precision_loss)]
impl Display for Cycles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // default format in TeraCycles
        write!(f, "{:.3} TC", self.to_u128() as f64 / 1_000_000_000_000f64)
    }
}

impl From<u128> for Cycles {
    fn from(v: u128) -> Self {
        Self(Nat::from(v))
    }
}

impl From<Nat> for Cycles {
    fn from(n: Nat) -> Self {
        Self(n)
    }
}

impl From<Cycles> for Nat {
    fn from(c: Cycles) -> Self {
        c.0
    }
}

// Human-input parser: "10K", "1.5T", etc.
impl FromStr for Cycles {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut num = String::new();
        let mut suf = String::new();
        let mut seen_dot = false;

        for ch in s.chars() {
            if ch.is_ascii_digit() || (ch == '.' && !seen_dot) {
                if ch == '.' {
                    seen_dot = true;
                }
                num.push(ch);
            } else {
                suf.push(ch);
            }
        }

        let n: f64 = num
            .parse::<f64>()
            .map_err(|e| format!("Invalid number '{num}': {e}"))?;

        let mul = match suf.as_str() {
            "K" => 1_000_f64,
            "M" => 1_000_000_f64,
            "B" => 1_000_000_000_f64,
            "T" => 1_000_000_000_000_f64,
            "Q" => 1_000_000_000_000_000_f64,
            "" => 1.0,
            _ => return Err(format!("Unknown suffix '{suf}'")),
        };

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(Self::new((n * mul) as u128))
    }
}

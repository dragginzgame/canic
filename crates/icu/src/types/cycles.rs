use candid::{CandidType, Nat};
use derive_more::{Add, AddAssign, Sub, SubAssign};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    str::FromStr,
};

///
/// Constants
///

pub const KC: u128 = 1_000;
pub const MC: u128 = 1_000_000;
pub const BC: u128 = 1_000_000_000;
pub const TC: u128 = 1_000_000_000_000;
pub const QC: u128 = 1_000_000_000_000_000;

///
/// Cycles
///

#[derive(
    Add,
    AddAssign,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Deserialize,
    CandidType,
    Serialize,
    SubAssign,
    Sub,
    Hash,
)]
pub struct Cycles(u128);

impl Cycles {
    #[must_use]
    pub const fn new(amount: u128) -> Self {
        Self(amount)
    }

    #[must_use]
    pub const fn as_u128(&self) -> u128 {
        self.0
    }

    // from_config
    // accepts the short hand 10T format or a number
    pub fn from_config<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
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

impl From<u128> for Cycles {
    fn from(u: u128) -> Self {
        Self(u)
    }
}

impl TryFrom<Nat> for Cycles {
    type Error = String;

    fn try_from(value: Nat) -> Result<Self, Self::Error> {
        value
            .0
            .try_into()
            .map(Self::new)
            .map_err(|_| "BigUint too large for u128".to_string())
    }
}

// FromStr so you can do "10T".parse::<Cycles>()
impl FromStr for Cycles {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut num_str = String::new();
        let mut suffix_str = String::new();
        let mut seen_dot = false;

        for c in s.chars() {
            if c.is_ascii_digit() || (c == '.' && !seen_dot) {
                if c == '.' {
                    seen_dot = true;
                }
                num_str.push(c);
            } else {
                suffix_str.push(c);
            }
        }

        let number: f64 = num_str
            .parse::<f64>()
            .map_err(|e| format!("Invalid number '{num_str}': {e}"))?;

        let multiplier = match suffix_str.as_str() {
            "K" => 1_000_f64,
            "M" => 1_000_000_f64,
            "B" => 1_000_000_000_f64,
            "T" => 1_000_000_000_000_f64,
            "Q" => 1_000_000_000_000_000_f64,
            "" => 1.0, // no suffix = raw cycles
            _ => return Err(format!("Unknown suffix '{suffix_str}'")),
        };

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(Self((number * multiplier) as u128))
    }
}

// Implement Display so we can format nicely
impl Display for Cycles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // default format in TeraCycles
        write!(f, "{:.3} TC", self.0 as f64 / 1_000_000_000_000f64)
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn parse_plain_number() {
        let c = Cycles::from_str("12345").unwrap();
        assert_eq!(c.as_u128(), 12345);
    }

    #[test]
    fn parse_with_k_suffix() {
        let c = Cycles::from_str("10K").unwrap();
        assert_eq!(c.as_u128(), 10 * KC);
    }

    #[test]
    fn parse_with_m_suffix() {
        let c = Cycles::from_str("2M").unwrap();
        assert_eq!(c.as_u128(), 2 * MC);
    }

    #[test]
    fn parse_with_b_suffix() {
        let c = Cycles::from_str("3B").unwrap();
        assert_eq!(c.as_u128(), 3 * BC);
    }

    #[test]
    fn parse_with_t_suffix() {
        let c = Cycles::from_str("4T").unwrap();
        assert_eq!(c.as_u128(), 4 * TC);
    }

    #[test]
    fn parse_with_q_suffix() {
        let c = Cycles::from_str("5Q").unwrap();
        assert_eq!(c.as_u128(), 5 * QC);
    }

    #[test]
    fn parse_decimal_number() {
        let c = Cycles::from_str("1.5K").unwrap();
        assert_eq!(c.as_u128(), 1500);
    }

    #[test]
    fn parse_invalid_number() {
        let err = Cycles::from_str("notanumber").unwrap_err();
        assert!(err.contains("Invalid number"));
    }

    #[test]
    fn parse_invalid_suffix() {
        let err = Cycles::from_str("10X").unwrap_err();
        assert!(err.contains("Unknown suffix"));
    }
}

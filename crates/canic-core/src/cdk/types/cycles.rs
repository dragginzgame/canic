//! Module: cdk::types::cycles
//!
//! Responsibility: cycle amount wrapper and parsing helpers.
//! Does not own: billing policy, cycle transfer workflows, or ledger calls.
//! Boundary: provides stable serialization and display behavior for cycle values.

use crate::cdk::{
    candid::{CandidType, Nat},
    structures::{Storable, storable::Bound},
};
use serde::{Deserialize, Serialize, de::Deserializer};
use std::{
    borrow::Cow,
    fmt::{self, Display},
    str::FromStr,
};
use thiserror::Error as ThisError;

///
/// Constants
///
/// Cycle unit shorthands for configs and logs
///

pub const KC: u128 = 1_000;
pub const MC: u128 = 1_000_000;
pub const BC: u128 = 1_000_000_000;
pub const TC: u128 = 1_000_000_000_000;
pub const QC: u128 = 1_000_000_000_000_000;

///
/// Cycles
///
/// Thin wrapper around `u128` that carries exact parsing and serialization
/// helpers for cycle balances.
///

#[derive(
    CandidType, Clone, Default, Debug, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize,
)]
pub struct Cycles(u128);

impl Cycles {
    #[must_use]
    pub const fn new(n: u128) -> Self {
        Self(n)
    }

    #[must_use]
    pub const fn to_u128(&self) -> u128 {
        self.0
    }

    /// Deserialize cycle config from either shorthand text such as `10T` or a number.
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

#[expect(clippy::cast_precision_loss)]
impl Display for Cycles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Render balances in teracycles for compact operator output.
        write!(f, "{:.3} TC", self.to_u128() as f64 / 1_000_000_000_000f64)
    }
}

///
/// CyclesConversionError
///
/// Typed failure while converting an unbounded Candid cycle amount.
/// Owned by the cycle value boundary and preserved by callers that narrow `Nat`.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum CyclesConversionError {
    #[error("cycle amount does not fit in u128: {value}")]
    NatOverflow { value: Nat },
}

impl TryFrom<Nat> for Cycles {
    type Error = CyclesConversionError;

    fn try_from(value: Nat) -> Result<Self, Self::Error> {
        u128::try_from(value.0.clone())
            .map(Self::new)
            .map_err(|_| CyclesConversionError::NatOverflow { value })
    }
}

impl From<u128> for Cycles {
    fn from(v: u128) -> Self {
        Self(v)
    }
}

impl From<Cycles> for u128 {
    fn from(c: Cycles) -> Self {
        c.0
    }
}

///
/// CyclesParseError
///
/// Typed failure while parsing an exact human-readable cycle amount.
/// Owned by the cycle value boundary and returned by `Cycles::from_str`.
///

#[derive(Clone, Debug, Eq, PartialEq, ThisError)]
pub enum CyclesParseError {
    #[error("cycle amount is empty")]
    Empty,

    #[error("cycle amount number is invalid: {value}")]
    InvalidNumber { value: String },

    #[error("cycle amount suffix is invalid: {suffix}")]
    InvalidSuffix { suffix: String },

    #[error("cycle amount exceeds u128: {value}")]
    Overflow { value: String },

    #[error("cycle amount has precision below one cycle: {value}")]
    SubcyclePrecision { value: String },
}

// Accept exact human-input cycle shorthand such as "10K" and "1.5T".
impl FromStr for Cycles {
    type Err = CyclesParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(CyclesParseError::Empty);
        }

        let suffix_start = s
            .find(|character: char| !(character.is_ascii_digit() || character == '.'))
            .unwrap_or(s.len());
        let (number, suffix) = s.split_at(suffix_start);
        let (multiplier, decimal_places) = match suffix {
            "" => (1, 0),
            "K" => (KC, 3),
            "M" => (MC, 6),
            "B" => (BC, 9),
            "T" => (TC, 12),
            "Q" => (QC, 15),
            _ => {
                return Err(CyclesParseError::InvalidSuffix {
                    suffix: suffix.to_string(),
                });
            }
        };
        let mut parts = number.split('.');
        let whole = parts.next().unwrap_or_default();
        let fraction = parts.next();
        if parts.next().is_some()
            || (whole.is_empty() && fraction.is_none_or(str::is_empty))
            || !whole.chars().all(|character| character.is_ascii_digit())
            || fraction
                .is_some_and(|digits| !digits.chars().all(|character| character.is_ascii_digit()))
        {
            return Err(CyclesParseError::InvalidNumber {
                value: number.to_string(),
            });
        }

        let whole = if whole.is_empty() {
            0
        } else {
            whole
                .parse::<u128>()
                .map_err(|_| CyclesParseError::Overflow {
                    value: s.to_string(),
                })?
        };
        let whole_cycles =
            whole
                .checked_mul(multiplier)
                .ok_or_else(|| CyclesParseError::Overflow {
                    value: s.to_string(),
                })?;
        let Some(fraction) = fraction else {
            return Ok(Self::new(whole_cycles));
        };
        let fraction = fraction.trim_end_matches('0');
        if fraction.is_empty() {
            return Ok(Self::new(whole_cycles));
        }
        let fraction_places =
            u32::try_from(fraction.len()).map_err(|_| CyclesParseError::SubcyclePrecision {
                value: s.to_string(),
            })?;
        if fraction_places > decimal_places {
            return Err(CyclesParseError::SubcyclePrecision {
                value: s.to_string(),
            });
        }
        let fraction_value = fraction
            .parse::<u128>()
            .map_err(|_| CyclesParseError::Overflow {
                value: s.to_string(),
            })?;
        let fraction_scale = 10_u128.pow(fraction_places);
        let fraction_cycles = fraction_value
            .checked_mul(multiplier / fraction_scale)
            .ok_or_else(|| CyclesParseError::Overflow {
                value: s.to_string(),
            })?;
        whole_cycles
            .checked_add(fraction_cycles)
            .map(Self::new)
            .ok_or_else(|| CyclesParseError::Overflow {
                value: s.to_string(),
            })
    }
}

impl Storable for Cycles {
    // u128 is exactly 16 bytes, fixed-size.
    const BOUND: Bound = Bound::Bounded {
        max_size: 16,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.0.to_be_bytes().to_vec())
    }

    fn into_bytes(self) -> Vec<u8> {
        self.0.to_be_bytes().to_vec()
    }

    /// Decode the exact fixed-width stable representation.
    ///
    /// # Panics
    ///
    /// Panics when stable memory contains a cycle value that is not exactly 16 bytes.
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let b = bytes.as_ref();
        let arr = <[u8; 16]>::try_from(b)
            .unwrap_or_else(|_| panic!("invalid stable Cycles length {}; expected 16", b.len()));

        Self(u128::from_be_bytes(arr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_exact_cycle_shorthand_without_floating_point() {
        assert_eq!("10K".parse::<Cycles>(), Ok(Cycles::new(10_000)));
        assert_eq!("1.5T".parse::<Cycles>(), Ok(Cycles::new(1_500_000_000_000)));
        assert_eq!(".25M".parse::<Cycles>(), Ok(Cycles::new(250_000)));
        assert_eq!("1.0000T".parse::<Cycles>(), Ok(Cycles::new(TC)));
        assert_eq!(
            u128::MAX.to_string().parse::<Cycles>(),
            Ok(Cycles::new(u128::MAX))
        );
    }

    #[test]
    fn rejects_cycle_overflow_and_subcycle_precision() {
        let overflow = format!("{}K", u128::MAX);
        assert!(matches!(
            overflow.parse::<Cycles>(),
            Err(CyclesParseError::Overflow { .. })
        ));
        assert!(matches!(
            "0.1".parse::<Cycles>(),
            Err(CyclesParseError::SubcyclePrecision { .. })
        ));
        assert!(matches!(
            "0.0001K".parse::<Cycles>(),
            Err(CyclesParseError::SubcyclePrecision { .. })
        ));
    }

    #[test]
    fn converts_candid_nat_only_when_it_fits() {
        assert_eq!(
            Cycles::try_from(Nat::from(u128::MAX)),
            Ok(Cycles::new(u128::MAX))
        );
        let too_large = Nat::parse(b"340282366920938463463374607431768211456")
            .expect("u128 max plus one is valid Nat");
        assert!(matches!(
            Cycles::try_from(too_large),
            Err(CyclesConversionError::NatOverflow { .. })
        ));
    }

    #[test]
    fn rejects_invalid_cycle_number_and_suffix() {
        assert!(matches!("".parse::<Cycles>(), Err(CyclesParseError::Empty)));
        assert!(matches!(
            ".".parse::<Cycles>(),
            Err(CyclesParseError::InvalidNumber { .. })
        ));
        assert!(matches!(
            "1TT".parse::<Cycles>(),
            Err(CyclesParseError::InvalidSuffix { .. })
        ));
    }

    #[test]
    #[should_panic(expected = "invalid stable Cycles length 15; expected 16")]
    fn malformed_stable_cycle_bytes_fail_closed() {
        let _ = Cycles::from_bytes(Cow::Borrowed(&[0; 15]));
    }
}

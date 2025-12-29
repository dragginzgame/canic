use crate::{
    candid::{CandidType, Nat},
    structures::{Storable, storable::Bound},
};
use derive_more::{Add, AddAssign, Sub, SubAssign};
use num_traits::cast::ToPrimitive;
use serde::{Deserialize, Serialize, de::Deserializer};
use std::{
    borrow::Cow,
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
pub struct Cycles(u128);

impl Cycles {
    #[must_use]
    pub const fn new(n: u128) -> Self {
        Self(n)
    }

    #[must_use]
    pub fn to_u64(&self) -> u64 {
        self.0.to_u64().unwrap_or(u64::MAX)
    }

    #[must_use]
    pub const fn to_u128(&self) -> u128 {
        self.0
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

impl From<Nat> for Cycles {
    fn from(n: Nat) -> Self {
        Self(n.0.to_u128().unwrap_or(0))
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

// Human-input parser: "10K", "1.5T", etc.
#[allow(clippy::cast_precision_loss)]
#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_possible_truncation)]
impl FromStr for Cycles {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut num = String::new();
        let mut suf = String::new();
        let mut suf_count = 0;
        for c in s.chars() {
            if c.is_ascii_digit() || c == '.' {
                if suf_count > 0 {
                    return Err("invalid suffix".to_string());
                }
                num.push(c);
            } else if suf_count >= 2 {
                return Err("invalid suffix".to_string());
            } else {
                suf.push(c);
                suf_count += 1;
            }
        }

        let mut n = num.parse::<f64>().map_err(|e| e.to_string())?;
        match suf.as_str() {
            "" => {}
            "K" => n *= KC as f64,
            "M" => n *= MC as f64,
            "B" => n *= BC as f64,
            "T" => n *= TC as f64,
            "Q" => n *= QC as f64,
            _ => return Err("invalid suffix".to_string()),
        }

        Ok(Self::new(n as u128))
    }
}

impl Storable for Cycles {
    // u128 is exactly 16 bytes, fixed-size
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

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let b = bytes.as_ref();

        // Defensive decode: never panic on corrupted data
        if b.len() != 16 {
            return Self::default();
        }

        let mut arr = [0u8; 16];
        arr.copy_from_slice(b);

        Self(u128::from_be_bytes(arr))
    }
}

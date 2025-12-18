use candid::{
    CandidType,
    types::{Serializer, Type, TypeInner},
};
use rust_decimal::Decimal as WrappedDecimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

///
/// Decimal
/// Candid-friendly decimal wrapper backed by `rust_decimal::Decimal`.
///
/// Candid encodes this as `text` using the canonical `to_string()` representation.
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Decimal(pub WrappedDecimal);

impl Decimal {
    #[must_use]
    pub const fn inner(self) -> WrappedDecimal {
        self.0
    }
}

impl CandidType for Decimal {
    fn _ty() -> Type {
        TypeInner::Text.into()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: Serializer,
    {
        self.0.to_string().idl_serialize(serializer)
    }
}

impl Serialize for Decimal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let d = WrappedDecimal::from_str(&s).map_err(serde::de::Error::custom)?;
        Ok(Self(d))
    }
}

impl From<WrappedDecimal> for Decimal {
    fn from(value: WrappedDecimal) -> Self {
        Self(value)
    }
}

impl From<u64> for Decimal {
    fn from(value: u64) -> Self {
        Self(WrappedDecimal::from(value))
    }
}

impl core::fmt::Display for Decimal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Decimal {
    type Err = rust_decimal::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(WrappedDecimal::from_str(s)?))
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_roundtrips_via_string() {
        let d: Decimal = "12.345".parse().unwrap();
        assert_eq!(d.to_string(), "12.345");
    }
}

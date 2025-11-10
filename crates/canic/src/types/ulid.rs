use candid::CandidType;
use derive_more::{Deref, DerefMut, Display, FromStr};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ulid::Ulid as WrappedUlid;

/// A Candid-safe wrapper around `ulid::Ulid`.
#[derive(
    Clone, Copy, Debug, Deref, DerefMut, Display, Eq, FromStr, Hash, Ord, PartialEq, PartialOrd,
)]
#[repr(transparent)]
pub struct Ulid(WrappedUlid);

impl Ulid {
    pub fn from_string(s: &str) -> Result<Self, ulid::DecodeError> {
        Ok(Self(WrappedUlid::from_string(s)?))
    }

    #[must_use]
    pub fn from_bytes(seed: u32) -> Self {
        let mut bytes = [0u8; 16];
        bytes[..4].copy_from_slice(&seed.to_be_bytes());

        Self(WrappedUlid::from_bytes(bytes))
    }
}

impl CandidType for Ulid {
    fn _ty() -> candid::types::Type {
        candid::types::TypeInner::Text.into()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        serializer.serialize_text(&self.0.to_string())
    }
}

impl From<WrappedUlid> for Ulid {
    fn from(u: WrappedUlid) -> Self {
        Self(u)
    }
}

impl From<Ulid> for WrappedUlid {
    fn from(u: Ulid) -> Self {
        u.0
    }
}

// The ulid crate's serde impls are gated behind its `serde` feature.
// With default-features disabled (to avoid pulling in `rand`), we implement
// Serialize/Deserialize here explicitly.
impl Serialize for Ulid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut buffer = [0; ::ulid::ULID_LEN];
        let text = self.array_to_str(&mut buffer);
        text.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Ulid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let deserialized_str = String::deserialize(deserializer)?;
        match WrappedUlid::from_string(&deserialized_str) {
            Ok(u) => Ok(Self(u)),
            Err(_) => Err(serde::de::Error::custom("invalid ulid string")),
        }
    }
}

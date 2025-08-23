use crate::impl_storable_bounded;
use candid::CandidType;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, str::FromStr};

///
/// CanisterType
///

#[derive(
    CandidType, Clone, Debug, Eq, Ord, Display, PartialOrd, Deserialize, Serialize, PartialEq, Hash,
)]
#[serde(transparent)]
pub struct CanisterType(pub Cow<'static, str>);

impl CanisterType {
    pub const ROOT: Self = Self(Cow::Borrowed("root"));

    #[must_use]
    pub const fn new(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }

    #[must_use]
    pub const fn owned(s: String) -> Self {
        Self(Cow::Owned(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for CanisterType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::owned(s.to_string()))
    }
}

impl_storable_bounded!(CanisterType, 48, false);

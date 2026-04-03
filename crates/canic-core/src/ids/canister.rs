use crate::{cdk::candid::CandidType, memory::impl_storable_bounded};
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, borrow::Cow, fmt, str::FromStr};

const ROOT_ROLE: &str = "root";
const WASM_STORE_ROLE: &str = "wasm_store";

///
/// CanisterRole
///

#[derive(
    CandidType, Clone, Debug, Eq, Ord, PartialOrd, Deserialize, Serialize, PartialEq, Hash,
)]
#[serde(transparent)]
pub struct CanisterRole(pub Cow<'static, str>);

impl CanisterRole {
    pub const ROOT: Self = Self(Cow::Borrowed(ROOT_ROLE));
    pub const WASM_STORE: Self = Self(Cow::Borrowed(WASM_STORE_ROLE));

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

    #[must_use]
    pub fn is_root(&self) -> bool {
        self.0.as_ref() == ROOT_ROLE
    }

    #[must_use]
    pub fn is_wasm_store(&self) -> bool {
        self.0.as_ref() == WASM_STORE_ROLE
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.0.into_owned()
    }
}

impl FromStr for CanisterRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::owned(s.to_string()))
    }
}

impl From<&'static str> for CanisterRole {
    fn from(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }
}

impl From<&String> for CanisterRole {
    fn from(s: &String) -> Self {
        Self(Cow::Owned(s.clone()))
    }
}

impl From<String> for CanisterRole {
    fn from(s: String) -> Self {
        Self(Cow::Owned(s))
    }
}

impl From<CanisterRole> for String {
    fn from(role: CanisterRole) -> Self {
        role.into_string()
    }
}

impl AsRef<str> for CanisterRole {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for CanisterRole {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for CanisterRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl_storable_bounded!(CanisterRole, 64, false);

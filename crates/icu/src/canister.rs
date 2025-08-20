use crate::impl_storable_bounded;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fmt::{self, Display},
    str::FromStr,
};

///
/// CanisterType
///

const ROOT_TYPE: &str = "root";

#[derive(
    CandidType, Deserialize, Clone, Debug, Eq, Ord, PartialOrd, PartialEq, Serialize, Hash,
)]
pub enum CanisterType {
    Root,
    Custom(Cow<'static, str>),
}

impl CanisterType {
    // runtime helper: can branch on string
    pub fn new<S: Into<Cow<'static, str>>>(s: S) -> Self {
        let s = s.into();
        match s.as_ref() {
            "root" => Self::Root,
            _ => Self::Custom(s),
        }
    }

    // const helper: only constructs Custom directly
    #[must_use]
    pub const fn custom(s: &'static str) -> Self {
        Self::Custom(Cow::Borrowed(s))
    }
}

impl Display for CanisterType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Root => ROOT_TYPE,
            Self::Custom(s) => s,
        })
    }
}

impl FromStr for CanisterType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.to_string()))
    }
}

impl_storable_bounded!(CanisterType, 64, false);

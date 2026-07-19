//! Module: ids::build_network
//!
//! Responsibility: build-network identifiers.
//! Does not own: ICP environment resolution or deployment selection.
//! Boundary: exposes the network class baked into a canister artifact.

use crate::cdk::candid::CandidType;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

///
/// BuildNetwork
///
/// Identifies the network class the canister was built for.
/// Owned by ids and consumed by build-network config and access checks.
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BuildNetwork {
    #[serde(rename = "ic")]
    Ic,
    #[serde(rename = "local")]
    Local,
}

impl BuildNetwork {
    /// Parse the canonical build-network label.
    #[must_use]
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "ic" => Some(Self::Ic),
            "local" => Some(Self::Local),
            _ => None,
        }
    }

    /// Return the stable build-network label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ic => "ic",
            Self::Local => "local",
        }
    }
}

impl Display for BuildNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

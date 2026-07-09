//! Module: ids::network
//!
//! Responsibility: build network identifiers.
//! Does not own: environment detection or deployment selection.
//! Boundary: exposes the network label a canister believes it runs under.

use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

///
/// BuildNetwork
///
/// Identifies the environment the canister believes it runs in.
/// Owned by ids and consumed by build-network config and access checks.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BuildNetwork {
    Ic,
    Local,
}

impl BuildNetwork {
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

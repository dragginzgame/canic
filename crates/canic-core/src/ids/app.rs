//! Module: ids::app
//!
//! Responsibility: identify one checked-in App definition.
//! Does not own: live Fleet identity, deployment labels, or filesystem paths.
//! Boundary: configuration validation admits the wrapped source name before use.

use serde::{Deserialize, Serialize};
use std::fmt;

///
/// AppId
///
/// Immutable source identity declared by `[app].name`.
///

#[derive(Clone, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct AppId(String);

impl AppId {
    #[must_use]
    pub const fn owned(value: String) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<str> for AppId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for AppId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<&str> for AppId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for AppId {
    fn from(value: String) -> Self {
        Self::owned(value)
    }
}

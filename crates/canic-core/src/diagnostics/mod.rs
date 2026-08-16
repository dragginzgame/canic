//! Module: canic_core::diagnostics
//!
//! Responsibility: own compact diagnostic identities available to runtime code.
//! Does not own: host rendering, allocation history, producer coverage, or public error mapping.
//! Boundary: raw identities preserve decoded values; registered identities alone authorize producers.

pub mod codes;

use std::fmt::{self, Debug, Display};

///
/// DiagnosticCode
///
/// Lossless raw diagnostic identity decoded from a boundary or retained for
/// observation. Possessing this value does not grant producer authority.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct DiagnosticCode(u16);

impl DiagnosticCode {
    /// Preserve any decoded raw identity, including unknown future values.
    #[must_use]
    pub const fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    /// Return the lossless numeric identity.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

impl Debug for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E{}", self.0)
    }
}

///
/// RegisteredDiagnosticCode
///
/// Centrally allocated producer identity. Only the canonical declaration tree
/// can construct this type from a number.
///

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct RegisteredDiagnosticCode(u16);

impl RegisteredDiagnosticCode {
    /// Return this registered producer identity as its lossless raw value.
    #[must_use]
    pub const fn raw_code(self) -> DiagnosticCode {
        DiagnosticCode(self.0)
    }
}

impl Debug for RegisteredDiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for RegisteredDiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.raw_code(), f)
    }
}

pub(in crate::diagnostics) const fn registered(raw: u16) -> RegisteredDiagnosticCode {
    assert!(raw != 0, "diagnostic code zero is not allocatable");
    RegisteredDiagnosticCode(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_and_registered_formatting_is_compact_and_numeric() {
        let raw = DiagnosticCode::from_raw(65_000);
        let registered = codes::ACCESS_UNAVAILABLE;

        assert_eq!(raw.raw(), 65_000);
        assert_eq!(raw.to_string(), "E65000");
        assert_eq!(format!("{raw:?}"), "E65000");
        assert_eq!(registered.raw_code().raw(), 1);
        assert_eq!(registered.to_string(), "E1");
        assert_eq!(format!("{registered:?}"), "E1");
    }
}

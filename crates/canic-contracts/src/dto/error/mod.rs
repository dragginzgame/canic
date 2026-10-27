use crate::{
    diagnostics::{DiagnosticCode, RegisteredDiagnosticCode},
    dto::prelude::*,
};
use std::{fmt, fmt::Display};

///
/// Error
///
/// Public API error payload. Only registered runtime reasons may originate a
/// value; Wire/Serde decoding may still preserve any raw `u16`.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct Error {
    code: u16,
}

impl Error {
    /// Originate a public error from a registered reason.
    #[must_use]
    pub const fn from_registered(code: RegisteredDiagnosticCode) -> Self {
        Self {
            code: code.raw_code().raw(),
        }
    }

    /// Observe the lossless diagnostic identity.
    #[must_use]
    pub const fn code(&self) -> DiagnosticCode {
        DiagnosticCode::from_raw(self.code)
    }

    /// Observe the raw Wire `nat16` value.
    #[must_use]
    pub const fn raw_code(&self) -> u16 {
        self.code
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.code(), f)
    }
}

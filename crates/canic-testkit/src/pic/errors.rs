use candid::Principal;

use super::{PicSerialGuardError, startup::PicStartError};

///
/// PicInstallError
///

#[derive(Debug, Eq, PartialEq)]
pub struct PicInstallError {
    canister_id: Principal,
    message: String,
}

///
/// StandaloneCanisterFixtureError
///

#[derive(Debug)]
pub enum StandaloneCanisterFixtureError {
    SerialGuard(PicSerialGuardError),
    Start(PicStartError),
    Install(PicInstallError),
}

impl PicInstallError {
    /// Capture one install failure for a specific canister id.
    #[must_use]
    pub const fn new(canister_id: Principal, message: String) -> Self {
        Self {
            canister_id,
            message,
        }
    }

    /// Read the canister id that failed to install.
    #[must_use]
    pub const fn canister_id(&self) -> Principal {
        self.canister_id
    }

    /// Read the captured panic message from the install attempt.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for PicInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "failed to install canister {}: {}",
            self.canister_id, self.message
        )
    }
}

impl std::error::Error for PicInstallError {}

impl std::fmt::Display for StandaloneCanisterFixtureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SerialGuard(err) => write!(f, "{err}"),
            Self::Start(err) => write!(f, "{err}"),
            Self::Install(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for StandaloneCanisterFixtureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::SerialGuard(err) => Some(err),
            Self::Start(err) => Some(err),
            Self::Install(err) => Some(err),
        }
    }
}

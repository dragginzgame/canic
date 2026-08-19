//! Bounded PocketIC server startup for repository-owned test journeys.

use ic_testkit::pic::{
    PocketIc, PocketIcBuilder, PocketIcBuilderExt, PocketIcStartupConfig, PocketIcStartupError,
};
use std::{error::Error as StdError, fmt, time::Duration};

const POCKET_IC_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const POCKET_IC_SERVER_URL_ENV: &str = "CANIC_POCKET_IC_SERVER_URL";

/// Structured failure while resolving or starting the pinned PocketIC server.
#[derive(Debug)]
pub enum PocketIcHarnessStartupError {
    /// The governed test entry point did not provide its owned server URL.
    MissingServerUrl,
    /// The testkit rejected, timed out or observed failure from bounded startup.
    Startup(PocketIcStartupError),
}

/// Start one explicitly configured PocketIC instance within the harness deadline.
pub fn try_start_pocket_ic(
    builder: PocketIcBuilder,
) -> Result<PocketIc, PocketIcHarnessStartupError> {
    let server_url = std::env::var(POCKET_IC_SERVER_URL_ENV)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or(PocketIcHarnessStartupError::MissingServerUrl)?;
    builder
        .try_build(PocketIcStartupConfig::connect(
            server_url,
            POCKET_IC_STARTUP_TIMEOUT,
        ))
        .map_err(PocketIcHarnessStartupError::Startup)
}

/// Start one PocketIC instance or stop the current test with structured diagnostics.
///
/// # Panics
///
/// Panics when the governed server URL is missing or bounded startup fails.
#[must_use]
pub fn start_pocket_ic(builder: PocketIcBuilder) -> PocketIc {
    try_start_pocket_ic(builder).unwrap_or_else(|error| panic!("start PocketIC: {error}"))
}

impl fmt::Display for PocketIcHarnessStartupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingServerUrl => write!(
                formatter,
                "{POCKET_IC_SERVER_URL_ENV} must name the governed PocketIC server; use the workspace test runner"
            ),
            Self::Startup(error) => error.fmt(formatter),
        }
    }
}

impl StdError for PocketIcHarnessStartupError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::MissingServerUrl => None,
            Self::Startup(error) => Some(error),
        }
    }
}

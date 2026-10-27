//! Bounded PocketIC server startup for repository-owned test journeys.

use std::time::Duration;
use {
    ic_platform::pocket_ic::{PocketIc, PocketIcBuilder},
    ic_testkit::pic::{PocketIcStartupConfig, PocketIcStartupError},
};

const POCKET_IC_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const POCKET_IC_SERVER_URL_ENV: &str = "IC_TESTKIT_POCKET_IC_URL";

/// Start one explicitly configured PocketIC instance within the harness deadline.
///
/// # Panics
/// Panics if Testkit connect mode unexpectedly acquires server ownership.
pub fn try_start_pocket_ic(builder: PocketIcBuilder) -> Result<PocketIc, PocketIcStartupError> {
    // An absent/non-Unicode URL is rejected by shared URL admission. Keep
    // explicit connect mode so an executable selection cannot spawn a server.
    let server_url = std::env::var(POCKET_IC_SERVER_URL_ENV).unwrap_or_default();
    PocketIcStartupConfig::Connect {
        server_url,
        timeout: POCKET_IC_STARTUP_TIMEOUT,
    }
    .build_instance(builder)
    .map(|owned| {
        // Connect borrows the caller-owned server; no server custody is transferred.
        let (instance, server) = owned.into_parts();
        assert!(server.is_none(), "Connect must not acquire server custody");
        instance
    })
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

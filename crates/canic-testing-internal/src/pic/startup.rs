//! Bounded PocketIC server startup for repository-owned test journeys.

use ic_testkit::pic::{
    PocketIc, PocketIcBuilder, PocketIcBuilderExt, PocketIcStartupConfig, PocketIcStartupError,
};
use std::time::Duration;

const POCKET_IC_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const POCKET_IC_SERVER_URL_ENV: &str = "IC_TESTKIT_POCKET_IC_URL";

/// Start one explicitly configured PocketIC instance within the harness deadline.
pub fn try_start_pocket_ic(builder: PocketIcBuilder) -> Result<PocketIc, PocketIcStartupError> {
    // An absent/non-Unicode URL is rejected by shared URL admission. Keep
    // explicit connect mode so an executable selection cannot spawn a server.
    let server_url = std::env::var(POCKET_IC_SERVER_URL_ENV).unwrap_or_default();
    builder.try_build(PocketIcStartupConfig::connect(
        server_url,
        POCKET_IC_STARTUP_TIMEOUT,
    ))
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

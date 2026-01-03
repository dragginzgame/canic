//! Non-root bootstrap phase.
//!
//! This module defines the asynchronous bootstrap phase for non-root canisters.
//! It runs *after* synchronous runtime initialization has completed successfully.
//!
//! Purpose:
//! - Provide a dedicated lifecycle phase for non-root canisters to perform
//!   local, asynchronous setup work.
//! - Preserve symmetry with the root lifecycle without duplicating
//!   cross-canister orchestration logic.
//!
//! Non-goals:
//! - This module does **not** create topology.
//! - This module does **not** self-register canisters.
//! - This module does **not** coordinate with other canisters.
//! - All cross-canister lifecycle orchestration is owned by the root canister.
//!
//! Current behavior:
//! - This phase is intentionally minimal and log-only.
//! - All required invariants are established earlier during
//!   `workflow::runtime` initialization.
//!
//! Architectural note:
//! This module exists to make the lifecycle boundary explicit and stable.
//! It provides a well-defined extension point should non-root canisters
//! later require asynchronous local bootstrap behavior.

use crate::{Error, log, log::Topic};

///
/// Bootstrap workflow for non-root canisters during init.
///
/// This function is scheduled asynchronously after the IC `init` hook
/// returns and after runtime initialization has completed.
///
/// At this point:
/// - Environment identity is fully initialized.
/// - Stable memory invariants hold.
/// - The canister is safe to run asynchronous tasks.
///
/// Failures in this phase are non-fatal:
/// - Errors are propagated to the lifecycle adapter.
/// - The adapter logs failures but does not abort canister initialization.
///
/// `_args` are opaque, user-provided bootstrap arguments forwarded from init.
/// They are currently unused.
///
/// This function is safe to retry and safe to run multiple times.
///
#[allow(clippy::unused_async)]
pub async fn bootstrap_init_nonroot_canister(_args: Option<Vec<u8>>) -> Result<(), Error> {
    log!(Topic::Init, Info, "bootstrap (nonroot): init start");
    log!(Topic::Init, Info, "bootstrap (nonroot): init complete");
    Ok(())
}

///
/// Bootstrap workflow for non-root canisters after upgrade.
///
/// This function runs asynchronously after a successful upgrade,
/// once runtime initialization has re-established all invariants.
///
/// This phase:
/// - Must be idempotent.
/// - Must tolerate repeated execution.
/// - Must not assume ordering relative to other canisters.
///
/// Errors are logged but do not abort the canister.
///
/// Current behavior is intentionally minimal.
///
#[allow(clippy::unused_async)]
pub async fn bootstrap_post_upgrade_nonroot_canister() -> Result<(), Error> {
    log!(Topic::Init, Info, "bootstrap (nonroot): post-upgrade start");
    log!(
        Topic::Init,
        Info,
        "bootstrap (nonroot): post-upgrade complete"
    );
    Ok(())
}

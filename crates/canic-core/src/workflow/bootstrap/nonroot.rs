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
//! - Bootstrap configured placement workers.
//! - Check delegated signer key material when configured.
//! - Mark the canister ready only after bootstrap work succeeds.
//!
//! Architectural note:
//! This module exists to make the lifecycle boundary explicit and stable.
//! It provides a well-defined extension point should non-root canisters
//! later require asynchronous local bootstrap behavior.

use crate::{InternalError, ops::runtime::ready::ReadyOps, workflow::prelude::*};

use crate::workflow::placement::scaling::ScalingWorkflow;
#[cfg(feature = "sharding")]
use crate::workflow::placement::sharding::ShardingWorkflow;
use crate::workflow::runtime::auth::RuntimeAuthWorkflow;

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
pub async fn bootstrap_init_nonroot_canister(_args: Option<Vec<u8>>) -> Result<(), InternalError> {
    log!(Topic::Init, Info, "bootstrap (nonroot): init start");

    #[cfg(feature = "sharding")]
    ShardingWorkflow::bootstrap_configured_initial_shards().await?;

    ScalingWorkflow::bootstrap_configured_initial_workers().await?;

    RuntimeAuthWorkflow::check_signer_key_material().await?;

    log!(Topic::Init, Info, "bootstrap (nonroot): init complete");
    ReadyOps::mark_ready();

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
/// Errors are propagated to the lifecycle adapter, matching init bootstrap
/// semantics.
///
pub async fn bootstrap_post_upgrade_nonroot_canister() -> Result<(), InternalError> {
    log!(Topic::Init, Info, "bootstrap (nonroot): post-upgrade start");
    RuntimeAuthWorkflow::check_signer_key_material().await?;
    log!(
        Topic::Init,
        Info,
        "bootstrap (nonroot): post-upgrade complete"
    );
    ReadyOps::mark_ready();

    Ok(())
}

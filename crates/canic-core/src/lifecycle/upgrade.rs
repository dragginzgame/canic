//! IC post-upgrade lifecycle adapters.
//!
//! This module contains **synchronous glue code** that adapts the IC
//! `post_upgrade` hook into async bootstrap workflows.
//!
//! Responsibilities:
//! - Restore minimal environment state required by workflows
//! - Perform no async work directly
//! - Delegate immediately to workflow bootstrap
//!
//! This module must NOT:
//! - Perform sequencing or orchestration
//! - Encode policy decisions
//! - Call ops beyond minimal environment restoration

use crate::{
    ids::CanisterRole,
    ops::{env::EnvOps, ic::timer::TimerOps},
    workflow,
};
use core::time::Duration;

/// Post-upgrade entrypoint for non-root canisters.
///
/// Environment state is expected to be persisted across upgrade;
/// only role context needs to be restored before delegating.
pub fn nonroot_post_upgrade(role: CanisterRole) {
    // Restore role context (env data already persisted)
    EnvOps::restore_role(role);

    // Delegate to async bootstrap workflow
    TimerOps::set(Duration::ZERO, "canic:bootstrap:nonroot_upgrade", async {
        workflow::bootstrap::nonroot_post_upgrade().await;
    });
}

/// Post-upgrade entrypoint for the root canister.
///
/// Root identity and subnet context are restored from stable state.
pub fn root_post_upgrade() {
    // Restore root environment context
    EnvOps::restore_root();

    // Delegate to async bootstrap workflow
    TimerOps::set(Duration::ZERO, "canic:bootstrap:root_upgrade", async {
        workflow::bootstrap::root_post_upgrade().await;
    });
}

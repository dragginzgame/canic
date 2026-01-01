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
    cdk::api::trap,
    ids::CanisterRole,
    ops::runtime::{env::EnvOps, timer::TimerOps},
    workflow,
};
use core::time::Duration;

/// Post-upgrade entrypoint for the root canister.
///
/// Root identity and subnet context are restored from stable state.
pub fn post_upgrade_root_canister() {
    // Restore root environment context
    if let Err(err) = EnvOps::restore_root() {
        let msg = format!("env restore failed (root upgrade): {err}");
        trap(&msg);
    }
    workflow::runtime::post_upgrade_root_canister();

    // Delegate to async bootstrap workflow
    TimerOps::set(
        Duration::ZERO,
        "canic:bootstrap:post_upgrade_root_canister",
        async {
            workflow::bootstrap::root::bootstrap_post_upgrade_root_canister().await;
        },
    );
}

/// Post-upgrade entrypoint for non-root canisters.
///
/// Environment state is expected to be persisted across upgrade;
/// only role context needs to be restored before delegating.
pub fn post_upgrade_nonroot_canister(role: CanisterRole) {
    // Restore role context (env data already persisted)
    if let Err(err) = EnvOps::restore_role(role.clone()) {
        let msg = format!("env restore failed (nonroot upgrade): {err}");
        trap(&msg);
    }
    workflow::runtime::post_upgrade_nonroot_canister(role);

    // Delegate to async bootstrap workflow
    TimerOps::set(
        Duration::ZERO,
        "canic:bootstrap:post_upgrade_nonroot_canister",
        async {
            workflow::bootstrap::nonroot::bootstrap_post_upgrade_nonroot_canister().await;
        },
    );
}

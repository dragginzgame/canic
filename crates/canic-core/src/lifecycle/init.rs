//! IC lifecycle adapters.
//!
//! This module adapts the IC’s synchronous lifecycle hooks (`init`,
//! `post_upgrade`, etc.) into the system’s two-phase initialization model:
//!
//! 1. **Synchronous runtime seeding**
//!    Minimal, non-async work that must execute inside the IC hook.
//!
//! 2. **Asynchronous bootstrap**
//!    Full initialization workflows scheduled via the timer immediately
//!    after the hook returns.
//!
//! This module exists to isolate **IC execution constraints** (synchronous
//! hooks, no `await`, strict time limits) from application orchestration.
//!
//! **DO NOT MERGE INTO WORKFLOW.**
//!
//! `lifecycle` is responsible only for *when* and *how* workflows are
//! permitted to start under IC rules. All orchestration, sequencing,
//! policy, and domain logic must remain in `workflow`.

use crate::{
    bootstrap,
    dto::{abi::v1::CanisterInitPayload, subnet::SubnetIdentity},
    ids::CanisterRole,
    lifecycle::{LifecyclePhase, lifecycle_trap},
    log,
    log::Topic,
    workflow::{self, runtime::timer::TimerWorkflow},
};
use std::time::Duration;

pub fn init_root_canister(identity: SubnetIdentity, config_str: &str, config_path: &str) {
    if let Err(err) = bootstrap::init_config(config_str) {
        lifecycle_trap(
            LifecyclePhase::Init,
            format!("config init failed (CANIC_CONFIG_PATH={config_path}): {err}"),
        );
    }

    // Perform minimal synchronous runtime seeding during IC init.
    if let Err(err) = workflow::runtime::init_root_canister(identity) {
        lifecycle_trap(LifecyclePhase::Init, err);
    }

    // Schedule async bootstrap immediately after init returns.
    TimerWorkflow::set(
        Duration::ZERO,
        "canic:bootstrap:init_root_canister",
        async {
            workflow::bootstrap::root::bootstrap_init_root_canister().await;
        },
    );
}

pub fn init_nonroot_canister(
    role: CanisterRole,
    payload: CanisterInitPayload,
    args: Option<Vec<u8>>,
    config_str: &str,
    config_path: &str,
) {
    if let Err(err) = bootstrap::init_config(config_str) {
        lifecycle_trap(
            LifecyclePhase::Init,
            format!("config init failed (CANIC_CONFIG_PATH={config_path}): {err}"),
        );
    }

    // Perform minimal synchronous runtime seeding during IC init.
    if let Err(err) = workflow::runtime::init_nonroot_canister(role, payload) {
        lifecycle_trap(LifecyclePhase::Init, err);
    }

    // Schedule async bootstrap immediately after init returns.
    // Duration::ZERO ensures execution on the next tick without
    // blocking the init hook.
    TimerWorkflow::set(
        Duration::ZERO,
        "canic:bootstrap:init_nonroot_canister",
        async move {
            // Non-root bootstrap failures are logged but must not
            // abort canister initialization.
            if let Err(err) =
                workflow::bootstrap::nonroot::bootstrap_init_nonroot_canister(args).await
            {
                log!(
                    Topic::Init,
                    Error,
                    "non-root bootstrap failed (init): {err}"
                );
            }
        },
    );
}

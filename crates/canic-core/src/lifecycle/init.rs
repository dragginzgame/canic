//! IC init lifecycle adapters.
//!
//! This module adapts the synchronous IC `init` hook into the system’s
//! two-phase initialization model:
//! 1. synchronous runtime seeding
//! 2. asynchronous bootstrap workflows scheduled via the timer
//!
//! This layer must remain minimal and must not contain orchestration,
//! policy, or domain logic.

use crate::{
    dto::{abi::v1::CanisterInitPayload, subnet::SubnetIdentity},
    ids::CanisterRole,
    log,
    log::Topic,
    ops::runtime::timer::TimerOps,
    workflow,
};
use core::time::Duration;

pub fn init_root_canister(identity: SubnetIdentity) {
    // Perform minimal synchronous runtime seeding during IC init.
    workflow::runtime::init_root_canister(identity);

    // Schedule async bootstrap immediately after init returns.
    TimerOps::set(
        Duration::ZERO,
        "canic:bootstrap:init_root_canister",
        async {
            // Root bootstrap failure is fatal: the subnet must not
            // continue in a partially initialized state.
            if let Err(err) = workflow::bootstrap::root::bootstrap_init_root_canister().await {
                let msg = format!("root bootstrap failed: {err}");
                crate::cdk::api::trap(&msg);
            }
        },
    );
}

pub fn init_nonroot_canister(
    role: CanisterRole,
    payload: CanisterInitPayload,
    args: Option<Vec<u8>>,
) {
    // Perform minimal synchronous runtime seeding during IC init.
    workflow::runtime::init_nonroot_canister(role, payload);

    // Schedule async bootstrap immediately after init returns.
    // Duration::ZERO ensures execution on the next tick without
    // blocking the init hook.
    TimerOps::set(
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

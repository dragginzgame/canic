//! IC init lifecycle adapters.
//!
//! This module contains synchronous glue code that adapts the IC `init` hook
//! into async bootstrap workflows. It must remain minimal and side-effect
//! limited to environment seeding and state import.

use crate::{
    dto::{abi::v1::CanisterInitPayload, subnet::SubnetIdentity},
    ids::CanisterRole,
    ops::ic::timer::TimerOps,
    workflow,
};
use core::time::Duration;

pub fn nonroot_init(role: CanisterRole, payload: CanisterInitPayload, args: Option<Vec<u8>>) {
    workflow::runtime::nonroot_init(role, payload);

    // Spawn async bootstrap workflow
    TimerOps::set(Duration::ZERO, "canic:bootstrap:nonroot_init", async move {
        workflow::bootstrap::nonroot_init(args).await;
    });
}

pub fn root_init(identity: SubnetIdentity) {
    workflow::runtime::root_init(identity);

    // Spawn async bootstrap workflow
    TimerOps::set(Duration::ZERO, "canic:bootstrap:root_init", async {
        if let Err(err) = workflow::bootstrap::root_init().await {
            let msg = format!("root bootstrap failed: {err}");
            crate::cdk::api::trap(&msg);
        }
    });
}

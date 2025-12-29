//! IC init lifecycle adapters.
//!
//! This module contains synchronous glue code that adapts the IC `init` hook
//! into async bootstrap workflows. It must remain minimal and side-effect
//! limited to environment seeding and state import.

use crate::{
    dto::{abi::v1::CanisterInitPayload, subnet::SubnetIdentity},
    ids::CanisterRole,
    ops::{
        adapter,
        env::EnvOps,
        ic::timer::TimerOps,
        storage::directory::{AppDirectoryOps, SubnetDirectoryOps},
    },
    workflow,
};
use core::time::Duration;

pub fn nonroot_init(role: CanisterRole, payload: CanisterInitPayload, args: Option<Vec<u8>>) {
    EnvOps::init(payload.env, role);

    let app_dir = adapter::directory::app_directory_from_view(payload.app_directory);
    let subnet_dir = adapter::directory::subnet_directory_from_view(payload.subnet_directory);

    AppDirectoryOps::import(app_dir);
    SubnetDirectoryOps::import(subnet_dir);

    // Spawn async bootstrap workflow
    TimerOps::set(Duration::ZERO, "canic:bootstrap:nonroot_init", async move {
        workflow::bootstrap::nonroot_init(args).await;
    });
}

pub fn root_init(identity: SubnetIdentity) {
    EnvOps::init_root(identity);

    // Spawn async bootstrap workflow
    TimerOps::set(Duration::ZERO, "canic:bootstrap:root_init", async {
        workflow::bootstrap::root_init().await;
    });
}

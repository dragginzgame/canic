//! IC init lifecycle adapters.
//!
//! This module contains synchronous glue code that adapts the IC `init` hook
//! into async bootstrap workflows. It must remain minimal and side-effect
//! limited to environment seeding and state import.

use crate::{
    abi::CanisterInitPayload,
    ids::CanisterRole,
    ops::{
        env::EnvOps,
        storage::directory::{AppDirectoryOps, SubnetDirectoryOps},
    },
    workflow,
};

use crate::cdk::futures::spawn;
use crate::ops::storage::registry::SubnetIdentity;

pub fn nonroot_init(role: CanisterRole, payload: CanisterInitPayload, args: Option<Vec<u8>>) {
    EnvOps::init(payload.env, role);
    AppDirectoryOps::import(payload.app_directory);
    SubnetDirectoryOps::import(payload.subnet_directory);

    // Spawn async bootstrap workflow
    spawn(async move {
        workflow::bootstrap::nonroot_init(args).await;
    });
}

pub fn root_init(identity: SubnetIdentity) {
    EnvOps::init_root(identity);

    // Spawn async bootstrap workflow
    spawn(async {
        workflow::bootstrap::root_init().await;
    });
}

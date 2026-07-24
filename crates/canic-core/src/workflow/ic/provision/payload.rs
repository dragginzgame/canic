//! Module: workflow::ic::provision::payload
//!
//! Responsibility: build non-root canister initialization payloads.
//! Does not own: environment storage, index schemas, or install execution.
//! Boundary: snapshots current environment and indexes into init payload DTOs.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::{abi::v1::CanisterInitPayload, env::EnvBootstrapArgs},
    ids::CanisterRole,
    ops::{
        runtime::env::EnvOps,
        storage::{
            StorageOpsError,
            fleet_activation::FleetActivationOps,
            index::{app::AppIndexOps, subnet::SubnetIndexOps},
        },
    },
    workflow::ic::provision::ProvisionWorkflow,
};

impl ProvisionWorkflow {
    pub fn build_nonroot_init_payload(
        role: &CanisterRole,
        parent_pid: Principal,
    ) -> Result<CanisterInitPayload, InternalError> {
        let env = EnvBootstrapArgs {
            prime_root_pid: Some(EnvOps::prime_root_pid()?),
            subnet_role: Some(EnvOps::subnet_role()?),
            subnet_pid: Some(EnvOps::subnet_pid()?),
            root_pid: Some(EnvOps::root_pid()?),
            canister_role: Some(role.clone()),
            parent_pid: Some(parent_pid),
        };

        let fleet_directory = AppIndexOps::snapshot_args();
        let subnet_directory = SubnetIndexOps::snapshot_args();
        let identity = FleetActivationOps::status(EnvOps::is_root())
            .map_err(StorageOpsError::from)?
            .identity;

        Ok(CanisterInitPayload {
            fleet: identity.fleet,
            install_id: identity.operation_id,
            release_build_id: identity.release_build_id,
            env,
            fleet_directory,
            subnet_directory,
        })
    }
}

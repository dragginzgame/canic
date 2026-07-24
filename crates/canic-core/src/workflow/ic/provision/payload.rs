//! Module: workflow::ic::provision::payload
//!
//! Responsibility: build non-root canister initialization payloads.
//! Does not own: environment storage, Directory schemas, or install execution.
//! Boundary: snapshots current environment and Directories into init payload DTOs.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::{abi::v1::CanisterInitPayload, env::EnvBootstrapArgs},
    ids::CanisterRole,
    ops::{
        runtime::env::EnvOps,
        storage::{
            StorageOpsError,
            directory::{fleet::FleetDirectoryOps, subnet::SubnetDirectoryOps},
            fleet_activation::FleetActivationOps,
        },
        topology::directory::current_provenance,
    },
    workflow::ic::provision::ProvisionWorkflow,
};

impl ProvisionWorkflow {
    pub fn build_nonroot_init_payload(
        role: &CanisterRole,
        parent_pid: Principal,
    ) -> Result<CanisterInitPayload, InternalError> {
        let env = EnvBootstrapArgs {
            fleet_root_pid: Some(EnvOps::fleet_root_pid()?),
            subnet_slot: Some(EnvOps::subnet_slot()?),
            subnet_pid: Some(EnvOps::subnet_pid()?),
            root_pid: Some(EnvOps::root_pid()?),
            canister_role: Some(role.clone()),
            parent_pid: Some(parent_pid),
        };

        let provenance = current_provenance()?;
        let fleet_directory = FleetDirectoryOps::snapshot_args(provenance.clone());
        let subnet_directory = SubnetDirectoryOps::snapshot_args(provenance);
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

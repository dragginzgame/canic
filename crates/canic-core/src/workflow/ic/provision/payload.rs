//! Module: workflow::ic::provision::payload
//!
//! Responsibility: build non-root canister initialization payloads.
//! Does not own: environment storage, Directory schemas, or install execution.
//! Boundary: emits infrastructure authority only; Components use the root Registry lifecycle.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::{
        abi::v1::{CanisterInitAuthority, CanisterInitPayload},
        env::EnvBootstrapArgs,
    },
    ids::CanisterRole,
    ops::{
        runtime::env::EnvOps,
        storage::{StorageOpsError, fleet_activation::FleetActivationOps},
    },
    workflow::ic::provision::ProvisionWorkflow,
};

impl ProvisionWorkflow {
    pub fn build_nonroot_init_payload(
        target_pid: Principal,
        role: &CanisterRole,
        parent_pid: Principal,
    ) -> Result<CanisterInitPayload, InternalError> {
        if !role.is_wasm_store() {
            return Err(InternalError::unavailable(
                "application Canisters must be installed through the Component Registry lifecycle",
            ));
        }
        let env = EnvBootstrapArgs {
            fleet_root_pid: Some(EnvOps::fleet_root_pid()?),
            component_spec: None,
            subnet_pid: Some(EnvOps::subnet_pid()?),
            root_pid: Some(EnvOps::root_pid()?),
            canister_role: Some(role.clone()),
            parent_pid: Some(parent_pid),
        };

        let identity = FleetActivationOps::status(EnvOps::is_root())
            .map_err(StorageOpsError::from)?
            .identity;
        if target_pid == Principal::anonymous() {
            return Err(InternalError::invalid_input(
                "managed infrastructure target Canister is anonymous",
            ));
        }

        Ok(CanisterInitPayload {
            install_id: identity.operation_id,
            release_build_id: identity.release_build_id,
            authority: CanisterInitAuthority::Infrastructure {
                fleet: identity.fleet,
                env,
            },
        })
    }
}

use crate::{
    InternalError,
    dto::{abi::v1::CanisterInitPayload, env::EnvBootstrapArgs},
    ops::{
        runtime::env::EnvOps,
        storage::index::{app::AppIndexOps, subnet::SubnetIndexOps},
    },
    workflow::{ic::provision::ProvisionWorkflow, prelude::*},
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

        let app_index = AppIndexOps::snapshot_args();
        let subnet_index = SubnetIndexOps::snapshot_args();

        Ok(CanisterInitPayload {
            env,
            app_index,
            subnet_index,
        })
    }
}

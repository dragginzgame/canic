pub mod query;

use crate::{
    InternalError, InternalErrorOrigin,
    domain::policy::env::{EnvInput, EnvPolicyError, validate_or_default},
    dto::env::EnvView,
    ops::{
        ic::network::{BuildNetwork, NetworkOps},
        runtime::env::EnvOps,
    },
    storage::stable::env::EnvData,
    workflow::prelude::*,
};

///
/// EnvWorkflow
///

pub struct EnvWorkflow;

impl EnvWorkflow {
    pub fn init_env_from_view(env_view: EnvView, role: CanisterRole) -> Result<(), InternalError> {
        let mut data = view_to_data(env_view);
        data.canister_role = Some(role);

        let network = NetworkOps::build_network().unwrap_or(BuildNetwork::Local);
        let input = EnvInput {
            prime_root_pid: data.prime_root_pid,
            subnet_role: data.subnet_role,
            subnet_pid: data.subnet_pid,
            root_pid: data.root_pid,
            canister_role: data.canister_role,
            parent_pid: data.parent_pid,
        };

        let validated = match validate_or_default(network, input) {
            Ok(validated) => validated,
            Err(EnvPolicyError::MissingEnvFields(missing)) => {
                return Err(InternalError::invariant(
                    InternalErrorOrigin::Workflow,
                    format!("bootstrap failed: missing required env fields: {missing}"),
                ));
            }
        };

        EnvOps::import(EnvData {
            prime_root_pid: Some(validated.prime_root_pid),
            subnet_role: Some(validated.subnet_role),
            subnet_pid: Some(validated.subnet_pid),
            root_pid: Some(validated.root_pid),
            canister_role: Some(validated.canister_role),
            parent_pid: Some(validated.parent_pid),
        })
    }
}

pub fn view_to_data(view: EnvView) -> EnvData {
    EnvData {
        prime_root_pid: view.prime_root_pid,
        subnet_role: view.subnet_role,
        subnet_pid: view.subnet_pid,
        root_pid: view.root_pid,
        canister_role: view.canister_role,
        parent_pid: view.parent_pid,
    }
}

pub fn data_to_view(data: EnvData) -> EnvView {
    EnvView {
        prime_root_pid: data.prime_root_pid,
        subnet_role: data.subnet_role,
        subnet_pid: data.subnet_pid,
        root_pid: data.root_pid,
        canister_role: data.canister_role,
        parent_pid: data.parent_pid,
    }
}

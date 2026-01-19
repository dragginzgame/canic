pub mod query;

use crate::{
    InternalError, InternalErrorOrigin,
    domain::policy::env::{EnvInput, EnvPolicyError, validate_or_default},
    dto::env::EnvBootstrapArgs,
    ops::{ic::network::NetworkOps, runtime::env::EnvOps},
    workflow::prelude::*,
};

///
/// EnvWorkflow
///

pub struct EnvWorkflow;

impl EnvWorkflow {
    pub fn init_env_from_args(
        env_args: EnvBootstrapArgs,
        role: CanisterRole,
    ) -> Result<(), InternalError> {
        let network = NetworkOps::build_network().ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "failed to determine runtime network".to_string(),
            )
        })?;

        let input = EnvInput {
            prime_root_pid: env_args.prime_root_pid,
            subnet_role: env_args.subnet_role,
            subnet_pid: env_args.subnet_pid,
            root_pid: env_args.root_pid,
            canister_role: Some(role),
            parent_pid: env_args.parent_pid,
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

        EnvOps::import_validated(validated)
    }
}

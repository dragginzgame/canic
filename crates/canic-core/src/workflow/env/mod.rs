//! Module: workflow::env
//!
//! Responsibility: validate and initialize runtime environment state from bootstrap args.
//! Does not own: endpoint authorization, stable env storage, or env policy rules.
//! Boundary: workflow layer between lifecycle bootstrap input, policy, and env ops.

pub mod query;

use crate::{
    InternalError, InternalErrorOrigin,
    config::ComponentTopologyError,
    domain::policy::pure::env::{EnvInput, EnvPolicyError, validate_or_default},
    dto::env::EnvBootstrapArgs,
    ids::{
        CanisterRole, ComponentBinding, ComponentChildBinding, FleetSubnetRootBinding,
        ManagedCanisterBinding,
    },
    model::env::ValidatedEnv,
    ops::{
        config::ConfigOps,
        ic::{IcOps, build_network::BuildNetworkOps},
        runtime::env::EnvOps,
    },
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
        BuildNetworkOps::build_network().ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "build network unavailable; set ICP_ENVIRONMENT=local|ic at build time".to_string(),
            )
        })?;

        let input = EnvInput {
            fleet_subnet_root_pid: env_args.fleet_subnet_root_pid,
            component_spec: env_args.component_spec,
            subnet_pid: env_args.subnet_pid,
            root_pid: env_args.root_pid,
            canister_role: Some(role),
            parent_pid: env_args.parent_pid,
        };

        let validated = match validate_or_default(input) {
            Ok(validated) => validated,
            Err(EnvPolicyError::MissingEnvFields(missing)) => {
                return Err(InternalError::invariant(
                    InternalErrorOrigin::Workflow,
                    format!("env args missing {missing}; local builds require explicit env fields"),
                ));
            }
        };

        EnvOps::import_validated(validated)
    }

    /// Initialize a top-level Component from exact root-issued Registry authority.
    pub fn init_component(
        root: &FleetSubnetRootBinding,
        binding: ComponentBinding,
        compiled_role: &CanisterRole,
    ) -> Result<(), InternalError> {
        let topology = ConfigOps::component_topology()?;
        topology
            .validate_component_binding(root, &binding)
            .map_err(map_binding_error)?;
        if &binding.role != compiled_role || binding.canister_id != IcOps::canister_self() {
            return Err(InternalError::invalid_input(
                "Component init authority differs from the compiled role or target Canister",
            ));
        }

        EnvOps::import_validated(ValidatedEnv {
            managed_binding: Some(ManagedCanisterBinding::Component(binding.clone())),
            fleet_subnet_root_pid: root.fleet_subnet_root,
            component_spec: Some(binding.component_spec),
            subnet_pid: binding.placement_subnet.into_principal(),
            root_pid: root.fleet_subnet_root,
            canister_role: binding.role,
            parent_pid: root.fleet_subnet_root,
        })
    }

    /// Initialize one Component descendant from exact root-issued Registry authority.
    pub fn init_component_child(
        root: &FleetSubnetRootBinding,
        binding: ComponentChildBinding,
        compiled_role: &CanisterRole,
    ) -> Result<(), InternalError> {
        let topology = ConfigOps::component_topology()?;
        topology
            .validate_component_child_binding(root, &binding)
            .map_err(map_binding_error)?;
        if &binding.role != compiled_role || binding.canister_id != IcOps::canister_self() {
            return Err(InternalError::invalid_input(
                "Component-child init authority differs from the compiled role or target Canister",
            ));
        }

        EnvOps::import_validated(ValidatedEnv {
            managed_binding: Some(ManagedCanisterBinding::ComponentChild(binding.clone())),
            fleet_subnet_root_pid: root.fleet_subnet_root,
            component_spec: Some(binding.component.component_spec),
            subnet_pid: binding.component.placement_subnet.into_principal(),
            root_pid: root.fleet_subnet_root,
            canister_role: binding.role,
            parent_pid: binding.parent_canister_id,
        })
    }
}

fn map_binding_error(error: ComponentTopologyError) -> InternalError {
    InternalError::invalid_input(format!("managed init authority is invalid: {error}"))
}

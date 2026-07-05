//! Module: workflow::ic::provision::policy
//!
//! Responsibility: validate provisioning registry policy before installation.
//! Does not own: topology policy definitions, registry mutation, or canister creation.
//! Boundary: gathers targeted observations and delegates decisions to domain policy.

use crate::{
    InternalError,
    config::schema::{CanisterConfig, CanisterKind},
    domain::policy,
    domain::policy::topology::registry::{RegistryCanisterKind, RegistryCanisterShape},
    ops::{config::ConfigOps, storage::registry::subnet::SubnetRegistryOps},
    workflow::prelude::*,
};

// Validate create-time registry policy using targeted registry lookups instead of a full export.
pub(super) fn validate_registration_policy(
    role: &CanisterRole,
    parent_pid: Principal,
) -> Result<(), InternalError> {
    let canister_cfg = ConfigOps::current_subnet_canister(role)?;
    let parent_role = SubnetRegistryOps::role_parent(parent_pid)
        .map(|(role, _)| role)
        .ok_or(policy::topology::TopologyPolicyError::ParentNotFound(
            parent_pid,
        ))?;
    let parent_cfg = ConfigOps::current_subnet_canister(&parent_role)?;
    let canister_shape = registry_canister_shape(&canister_cfg);
    let parent_shape = registry_canister_shape(&parent_cfg);

    let observed = policy::topology::registry::RegistryRegistrationObservation {
        existing_role_pid: matches!(
            canister_shape.kind,
            RegistryCanisterKind::Root | RegistryCanisterKind::Service
        )
        .then(|| SubnetRegistryOps::find_pid_for_role(role))
        .flatten(),
        existing_singleton_under_parent_pid: matches!(
            canister_shape.kind,
            RegistryCanisterKind::Singleton
        )
        .then(|| {
            if role.is_wasm_store() {
                None
            } else {
                SubnetRegistryOps::find_child_pid_for_role(parent_pid, role)
            }
        })
        .flatten(),
    };

    policy::topology::registry::RegistryPolicy::can_register_role_observed(
        role,
        parent_pid,
        observed,
        canister_shape,
        &parent_role,
        parent_shape,
    )
    .map_err(policy::topology::TopologyPolicyError::from)?;

    Ok(())
}

const fn registry_canister_shape(cfg: &CanisterConfig) -> RegistryCanisterShape {
    RegistryCanisterShape {
        kind: registry_canister_kind(cfg.kind),
        has_scaling: cfg.scaling.is_some(),
        has_sharding: cfg.sharding.is_some(),
        has_directory: cfg.directory.is_some(),
    }
}

const fn registry_canister_kind(kind: CanisterKind) -> RegistryCanisterKind {
    match kind {
        CanisterKind::Root => RegistryCanisterKind::Root,
        CanisterKind::Service => RegistryCanisterKind::Service,
        CanisterKind::Singleton => RegistryCanisterKind::Singleton,
        CanisterKind::Replica => RegistryCanisterKind::Replica,
        CanisterKind::Shard => RegistryCanisterKind::Shard,
        CanisterKind::Instance => RegistryCanisterKind::Instance,
    }
}

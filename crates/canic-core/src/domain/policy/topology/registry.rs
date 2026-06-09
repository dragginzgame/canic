use crate::{
    cdk::types::Principal,
    config::schema::{CanisterConfig, CanisterKind},
    ids::CanisterRole,
};
use thiserror::Error as ThisError;

#[cfg(test)]
use super::RegistryPolicyInput;

///
/// RegistryPolicyError
/// Errors raised during registry kind evaluation.
///

#[derive(Debug, ThisError)]
pub enum RegistryPolicyError {
    #[error("role {role} already registered to {pid}")]
    RoleAlreadyRegistered { role: CanisterRole, pid: Principal },

    #[error("service role {role} must be created by root parent (parent role {parent_role})")]
    ServiceRequiresRootParent {
        role: CanisterRole,
        parent_role: CanisterRole,
    },

    #[error("singleton role {role} already registered under parent {parent_pid} (pid {pid})")]
    SingletonAlreadyRegisteredUnderParent {
        role: CanisterRole,
        parent_pid: Principal,
        pid: Principal,
    },

    #[error(
        "replica role {role} must be created by a service parent with scaling config (parent role {parent_role})"
    )]
    ReplicaRequiresServiceWithScaling {
        role: CanisterRole,
        parent_role: CanisterRole,
    },

    #[error(
        "shard role {role} must be created by a service parent with sharding config (parent role {parent_role})"
    )]
    ShardRequiresServiceWithSharding {
        role: CanisterRole,
        parent_role: CanisterRole,
    },

    #[error(
        "instance role {role} must be created by a service parent with directory config (parent role {parent_role})"
    )]
    InstanceRequiresServiceWithDirectory {
        role: CanisterRole,
        parent_role: CanisterRole,
    },
}

///
/// RegistryRegistrationObservation
/// Minimal observed registry facts needed to evaluate registration policy.
///
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RegistryRegistrationObservation {
    pub existing_role_pid: Option<Principal>,
    pub existing_singleton_under_parent_pid: Option<Principal>,
}

///
/// RegistryPolicy
///

pub struct RegistryPolicy;

impl RegistryPolicy {
    // Evaluate registration policy from a minimized observed registry view.
    pub fn can_register_role_observed(
        role: &CanisterRole,
        parent_pid: Principal,
        observed: RegistryRegistrationObservation,
        canister_cfg: &CanisterConfig,
        parent_role: &CanisterRole,
        parent_cfg: &CanisterConfig,
    ) -> Result<(), RegistryPolicyError> {
        Self::can_register_role_with_observation(
            role,
            parent_pid,
            observed,
            canister_cfg,
            parent_role,
            parent_cfg,
        )
    }

    // Evaluate registration policy from the full registry snapshot.
    #[cfg(test)]
    pub fn can_register_role(
        role: &CanisterRole,
        parent_pid: Principal,
        data: &RegistryPolicyInput,
        canister_cfg: &CanisterConfig,
        parent_role: &CanisterRole,
        parent_cfg: &CanisterConfig,
    ) -> Result<(), RegistryPolicyError> {
        let observed = RegistryRegistrationObservation {
            existing_role_pid: data
                .entries
                .iter()
                .find(|entry| entry.role == *role)
                .map(|entry| entry.pid),
            existing_singleton_under_parent_pid: data
                .entries
                .iter()
                .find(|entry| entry.role == *role && entry.parent_pid == Some(parent_pid))
                .map(|entry| entry.pid),
        };

        Self::can_register_role_with_observation(
            role,
            parent_pid,
            observed,
            canister_cfg,
            parent_role,
            parent_cfg,
        )
    }

    // Evaluate registration policy from the shared observed facts.
    fn can_register_role_with_observation(
        role: &CanisterRole,
        parent_pid: Principal,
        observed: RegistryRegistrationObservation,
        canister_cfg: &CanisterConfig,
        parent_role: &CanisterRole,
        parent_cfg: &CanisterConfig,
    ) -> Result<(), RegistryPolicyError> {
        match canister_cfg.kind {
            CanisterKind::Root => {
                if let Some(pid) = observed.existing_role_pid {
                    return Err(RegistryPolicyError::RoleAlreadyRegistered {
                        role: role.clone(),
                        pid,
                    });
                }
            }
            CanisterKind::Service => {
                if !parent_role.is_root() {
                    return Err(RegistryPolicyError::ServiceRequiresRootParent {
                        role: role.clone(),
                        parent_role: parent_role.clone(),
                    });
                }

                if let Some(pid) = observed.existing_role_pid {
                    return Err(RegistryPolicyError::RoleAlreadyRegistered {
                        role: role.clone(),
                        pid,
                    });
                }
            }
            CanisterKind::Singleton => {
                if role.is_wasm_store() {
                    return Ok(());
                }

                if let Some(pid) = observed.existing_singleton_under_parent_pid {
                    return Err(RegistryPolicyError::SingletonAlreadyRegisteredUnderParent {
                        role: role.clone(),
                        parent_pid,
                        pid,
                    });
                }
            }
            CanisterKind::Replica => {
                if !is_service_manager_parent_kind(parent_cfg.kind) || parent_cfg.scaling.is_none()
                {
                    return Err(RegistryPolicyError::ReplicaRequiresServiceWithScaling {
                        role: role.clone(),
                        parent_role: parent_role.clone(),
                    });
                }
            }
            CanisterKind::Shard => {
                if !is_service_manager_parent_kind(parent_cfg.kind) || parent_cfg.sharding.is_none()
                {
                    return Err(RegistryPolicyError::ShardRequiresServiceWithSharding {
                        role: role.clone(),
                        parent_role: parent_role.clone(),
                    });
                }
            }
            CanisterKind::Instance => {
                if !is_service_manager_parent_kind(parent_cfg.kind)
                    || parent_cfg.directory.is_none()
                {
                    return Err(RegistryPolicyError::InstanceRequiresServiceWithDirectory {
                        role: role.clone(),
                        parent_role: parent_role.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}

const fn is_service_manager_parent_kind(kind: CanisterKind) -> bool {
    matches!(kind, CanisterKind::Service)
}

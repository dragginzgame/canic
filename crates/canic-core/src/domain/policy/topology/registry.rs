use crate::{
    InternalError,
    cdk::candid::Principal,
    config::schema::{CanisterConfig, CanisterKind},
    domain::policy::topology::TopologyPolicyError,
    ids::CanisterRole,
    storage::stable::registry::subnet::SubnetRegistryData,
};
use thiserror::Error as ThisError;

///
/// RegistryPolicyError
/// Errors raised during registry kind evaluation.
///

#[derive(Debug, ThisError)]
pub enum RegistryPolicyError {
    #[error("role {role} already registered to {pid}")]
    RoleAlreadyRegistered { role: CanisterRole, pid: Principal },

    #[error("role {role} already registered under parent {parent_pid} (pid {pid})")]
    RoleAlreadyRegisteredUnderParent {
        role: CanisterRole,
        parent_pid: Principal,
        pid: Principal,
    },
}

impl From<RegistryPolicyError> for InternalError {
    fn from(err: RegistryPolicyError) -> Self {
        TopologyPolicyError::from(err).into()
    }
}

///
/// RegistryPolicy
///

pub struct RegistryPolicy;

impl RegistryPolicy {
    pub fn can_register_role(
        role: &CanisterRole,
        parent_pid: Principal,
        data: &SubnetRegistryData,
        canister_cfg: &CanisterConfig,
    ) -> Result<(), RegistryPolicyError> {
        match canister_cfg.kind {
            CanisterKind::Root => {
                if let Some((pid, _)) = data.entries.iter().find(|(_, entry)| entry.role == *role) {
                    return Err(RegistryPolicyError::RoleAlreadyRegistered {
                        role: role.clone(),
                        pid: *pid,
                    });
                }
            }
            CanisterKind::Node => {
                if let Some((pid, _)) = data
                    .entries
                    .iter()
                    .find(|(_, entry)| entry.role == *role && entry.parent_pid == Some(parent_pid))
                {
                    return Err(RegistryPolicyError::RoleAlreadyRegisteredUnderParent {
                        role: role.clone(),
                        parent_pid,
                        pid: *pid,
                    });
                }
            }
            CanisterKind::Worker | CanisterKind::Shard => {}
        }

        Ok(())
    }
}

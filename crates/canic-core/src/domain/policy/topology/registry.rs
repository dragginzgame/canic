use crate::{
    Error, ThisError,
    cdk::candid::Principal,
    config::schema::{CanisterCardinality, CanisterConfig},
    domain::policy::topology::TopologyPolicyError,
    ids::CanisterRole,
    ops::storage::registry::subnet::SubnetRegistrySnapshot,
};

///
/// RegistryPolicyError
/// Errors raised during registry cardinality evaluation.
///

#[derive(Debug, ThisError)]
pub enum RegistryPolicyError {
    #[error("role {role} already registered to {pid}")]
    RoleAlreadyRegistered { role: CanisterRole, pid: Principal },
}

impl From<RegistryPolicyError> for Error {
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
        snapshot: &SubnetRegistrySnapshot,
        canister_cfg: &CanisterConfig,
    ) -> Result<(), RegistryPolicyError> {
        if canister_cfg.cardinality == CanisterCardinality::One
            && let Some((pid, _)) = snapshot
                .entries
                .iter()
                .find(|(_, entry)| entry.role == *role)
        {
            return Err(RegistryPolicyError::RoleAlreadyRegistered {
                role: role.clone(),
                pid: *pid,
            });
        }

        Ok(())
    }
}

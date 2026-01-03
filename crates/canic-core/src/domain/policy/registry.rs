use crate::{
    Error, ThisError,
    config::schema::{CanisterCardinality, CanisterConfig},
    domain::policy::PolicyError,
    ids::CanisterRole,
    ops::storage::registry::subnet::SubnetRegistrySnapshot,
};
use candid::Principal;

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
        PolicyError::from(err).into()
    }
}

///
/// RegistryPolicy
///

pub struct RegistryPolicy;

impl RegistryPolicy {
    pub(crate) fn can_register_role(
        role: &CanisterRole,
        snapshot: &SubnetRegistrySnapshot,
        canister_cfg: &CanisterConfig,
    ) -> Result<(), Error> {
        if canister_cfg.cardinality == CanisterCardinality::Single
            && let Some((pid, _)) = snapshot
                .entries
                .iter()
                .find(|(_, entry)| entry.role == *role)
        {
            return Err(RegistryPolicyError::RoleAlreadyRegistered {
                role: role.clone(),
                pid: *pid,
            }
            .into());
        }

        Ok(())
    }
}

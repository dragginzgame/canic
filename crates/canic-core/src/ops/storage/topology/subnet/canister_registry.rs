pub use crate::model::memory::topology::SubnetCanisterRegistryView;

use crate::{
    Error, ThisError,
    cdk::types::Principal,
    config::schema::CanisterCardinality,
    ids::CanisterRole,
    model::memory::{CanisterEntry, CanisterSummary, topology::SubnetCanisterRegistry},
    ops::{config::ConfigOps, storage::topology::TopologyOpsError},
};

///
/// SubnetCanisterRegistryOpsError
///

#[derive(Debug, ThisError)]
pub enum SubnetCanisterRegistryOpsError {
    #[error("canister {0} not found in registry")]
    NotFound(Principal),

    #[error("canister role {0} not found in registry")]
    RoleNotFound(CanisterRole),

    #[error("canister {0} already registered")]
    AlreadyRegistered(Principal),

    #[error("role {role} already registered to {pid}")]
    RoleAlreadyRegistered { role: CanisterRole, pid: Principal },
}

impl From<SubnetCanisterRegistryOpsError> for Error {
    fn from(err: SubnetCanisterRegistryOpsError) -> Self {
        TopologyOpsError::from(err).into()
    }
}

///
/// SubnetCanisterRegistryOps
///

pub struct SubnetCanisterRegistryOps;

impl SubnetCanisterRegistryOps {
    pub(crate) fn register(
        pid: Principal,
        role: &CanisterRole,
        parent_pid: Principal,
        module_hash: Vec<u8>,
    ) -> Result<(), Error> {
        if SubnetCanisterRegistry::get(pid).is_some() {
            return Err(SubnetCanisterRegistryOpsError::AlreadyRegistered(pid).into());
        }

        if role_requires_singleton(role)
            && let Some(existing) = SubnetCanisterRegistry::get_type(role)
        {
            return Err(SubnetCanisterRegistryOpsError::RoleAlreadyRegistered {
                role: role.clone(),
                pid: existing.pid,
            }
            .into());
        }

        SubnetCanisterRegistry::register(pid, role, parent_pid, module_hash);

        Ok(())
    }

    #[must_use]
    pub fn get(pid: Principal) -> Option<CanisterEntry> {
        SubnetCanisterRegistry::get(pid)
    }

    #[must_use]
    pub fn get_parent(pid: Principal) -> Option<Principal> {
        SubnetCanisterRegistry::get_parent(pid)
    }

    #[must_use]
    pub fn get_type(role: &CanisterRole) -> Option<CanisterEntry> {
        SubnetCanisterRegistry::get_type(role)
    }

    #[must_use]
    pub(crate) fn children(pid: Principal) -> Vec<CanisterSummary> {
        SubnetCanisterRegistry::children(pid)
    }

    #[must_use]
    pub(crate) fn subtree(pid: Principal) -> Vec<CanisterSummary> {
        SubnetCanisterRegistry::subtree(pid)
    }

    #[must_use]
    pub(crate) fn remove(pid: &Principal) -> Option<CanisterEntry> {
        SubnetCanisterRegistry::remove(pid)
    }

    pub(crate) fn register_root(pid: Principal) {
        SubnetCanisterRegistry::register_root(pid);
    }

    pub fn try_get(pid: Principal) -> Result<CanisterEntry, SubnetCanisterRegistryOpsError> {
        SubnetCanisterRegistry::get(pid).ok_or(SubnetCanisterRegistryOpsError::NotFound(pid))
    }

    pub fn try_get_parent(pid: Principal) -> Result<Principal, SubnetCanisterRegistryOpsError> {
        SubnetCanisterRegistry::get_parent(pid).ok_or(SubnetCanisterRegistryOpsError::NotFound(pid))
    }

    pub fn try_get_type(
        role: &CanisterRole,
    ) -> Result<CanisterEntry, SubnetCanisterRegistryOpsError> {
        SubnetCanisterRegistry::get_type(role)
            .ok_or_else(|| SubnetCanisterRegistryOpsError::RoleNotFound(role.clone()))
    }

    pub(crate) fn update_module_hash(
        pid: Principal,
        module_hash: Vec<u8>,
    ) -> Result<(), SubnetCanisterRegistryOpsError> {
        if SubnetCanisterRegistry::update_module_hash(pid, module_hash) {
            Ok(())
        } else {
            Err(SubnetCanisterRegistryOpsError::NotFound(pid))
        }
    }

    #[must_use]
    pub fn export() -> Vec<CanisterEntry> {
        SubnetCanisterRegistry::export()
    }
}

fn role_requires_singleton(role: &CanisterRole) -> bool {
    let cfg = ConfigOps::current_subnet_canister(role);
    cfg.cardinality == CanisterCardinality::Single
}

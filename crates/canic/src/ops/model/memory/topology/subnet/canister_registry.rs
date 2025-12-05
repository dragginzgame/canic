pub use crate::model::memory::topology::SubnetCanisterRegistryView;

use crate::{
    Error, ThisError,
    core::types::Principal,
    model::memory::{CanisterEntry, CanisterSummary, topology::SubnetCanisterRegistry},
    ops::model::memory::topology::TopologyOpsError,
    types::CanisterType,
};

///
/// SubnetCanisterRegistryOpsError
///

#[derive(Debug, ThisError)]
pub enum SubnetCanisterRegistryOpsError {
    #[error("canister {0} not found in registry")]
    NotFound(Principal),

    #[error("canister type {0} not found in registry")]
    TypeNotFound(CanisterType),
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
    #[must_use]
    pub fn export() -> Vec<CanisterEntry> {
        SubnetCanisterRegistry::export()
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
    pub fn get_type(ty: &CanisterType) -> Option<CanisterEntry> {
        SubnetCanisterRegistry::get_type(ty)
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
    pub(crate) fn is_in_subtree(
        root_pid: Principal,
        entry: &CanisterSummary,
        all: &[CanisterSummary],
    ) -> bool {
        SubnetCanisterRegistry::is_in_subtree(root_pid, entry, all)
    }

    #[must_use]
    pub(crate) fn remove(pid: &Principal) -> Option<CanisterEntry> {
        SubnetCanisterRegistry::remove(pid)
    }

    pub(crate) fn register(
        pid: Principal,
        ty: &CanisterType,
        parent_pid: Principal,
        module_hash: Vec<u8>,
    ) {
        SubnetCanisterRegistry::register(pid, ty, parent_pid, module_hash);
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
        ty: &CanisterType,
    ) -> Result<CanisterEntry, SubnetCanisterRegistryOpsError> {
        SubnetCanisterRegistry::get_type(ty)
            .ok_or_else(|| SubnetCanisterRegistryOpsError::TypeNotFound(ty.clone()))
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
}

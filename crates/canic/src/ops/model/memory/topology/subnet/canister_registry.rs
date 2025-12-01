pub use crate::model::memory::topology::SubnetCanisterRegistryView;

use crate::{
    Error, ThisError,
    model::memory::{CanisterEntry, CanisterSummary, topology::SubnetCanisterRegistry},
    ops::model::memory::topology::TopologyOpsError,
    types::{CanisterType, Principal},
};

///
/// SubnetCanisterRegistryOpsError
///

#[derive(Debug, ThisError)]
pub enum SubnetCanisterRegistryOpsError {
    #[error("canister {0} not found in registry")]
    NotFound(Principal),
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
    pub fn children(pid: Principal) -> Vec<CanisterSummary> {
        SubnetCanisterRegistry::children(pid)
    }

    #[must_use]
    pub fn subtree(pid: Principal) -> Vec<CanisterSummary> {
        SubnetCanisterRegistry::subtree(pid)
    }

    #[must_use]
    pub fn is_in_subtree(
        root_pid: Principal,
        entry: &CanisterSummary,
        all: &[CanisterSummary],
    ) -> bool {
        SubnetCanisterRegistry::is_in_subtree(root_pid, entry, all)
    }

    #[must_use]
    pub fn remove(pid: &Principal) -> Option<CanisterEntry> {
        SubnetCanisterRegistry::remove(pid)
    }

    pub fn register(
        pid: Principal,
        ty: &CanisterType,
        parent_pid: Principal,
        module_hash: Vec<u8>,
    ) {
        SubnetCanisterRegistry::register(pid, ty, parent_pid, module_hash);
    }

    pub fn register_root(pid: Principal) {
        SubnetCanisterRegistry::register_root(pid);
    }

    pub fn try_get(pid: Principal) -> Result<CanisterEntry, Error> {
        SubnetCanisterRegistry::get(pid)
            .ok_or_else(|| SubnetCanisterRegistryOpsError::NotFound(pid).into())
    }
}

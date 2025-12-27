pub use crate::model::memory::registry::SubnetRegistryView;

use crate::{
    Error, ThisError,
    cdk::types::Principal,
    config::schema::CanisterCardinality,
    dto::topology::CanisterChildrenView,
    ids::CanisterRole,
    model::memory::{CanisterEntry, CanisterSummary, registry::SubnetRegistry},
    ops::{config::ConfigOps, storage::registry::RegistryOpsError},
};
use std::collections::HashSet;

///
/// SubnetRegistryOpsError
///
/// All semantic and invariant violations related to the subnet registry.
///

#[derive(Debug, ThisError)]
pub enum SubnetRegistryOpsError {
    // ---------------------------------------------------------------------
    // Registration errors
    // ---------------------------------------------------------------------
    #[error("canister {0} already registered")]
    AlreadyRegistered(Principal),

    #[error("role {role} already registered to {pid}")]
    RoleAlreadyRegistered { role: CanisterRole, pid: Principal },

    // ---------------------------------------------------------------------
    // Traversal / invariant errors
    // ---------------------------------------------------------------------
    #[error("canister {0} not found in subnet registry")]
    CanisterNotFound(Principal),

    #[error("parent chain contains a cycle at {0}")]
    ParentChainCycle(Principal),

    #[error("parent chain exceeded registry size ({0})")]
    ParentChainTooLong(usize),

    #[error("parent chain did not terminate at root (last pid: {0})")]
    ParentChainNotRootTerminated(Principal),
}

impl From<SubnetRegistryOpsError> for Error {
    fn from(err: SubnetRegistryOpsError) -> Self {
        RegistryOpsError::from(err).into()
    }
}

///
/// SubnetRegistryOps
///
/// Semantic operations over the subnet registry.
/// Enforces cardinality, parent-chain invariants, and traversal safety.
///

pub struct SubnetRegistryOps;

impl SubnetRegistryOps {
    // ---------------------------------------------------------------------
    // Mutation
    // ---------------------------------------------------------------------

    pub(crate) fn register(
        pid: Principal,
        role: &CanisterRole,
        parent_pid: Principal,
        module_hash: Vec<u8>,
    ) -> Result<(), Error> {
        if SubnetRegistry::get(pid).is_some() {
            return Err(SubnetRegistryOpsError::AlreadyRegistered(pid).into());
        }

        if role_requires_singleton(role)
            && let Some(existing) = SubnetRegistry::find_first_by_role(role)
        {
            return Err(SubnetRegistryOpsError::RoleAlreadyRegistered {
                role: role.clone(),
                pid: existing.pid,
            }
            .into());
        }

        SubnetRegistry::register(pid, role, parent_pid, module_hash);
        Ok(())
    }

    pub(crate) fn remove(pid: &Principal) -> Option<CanisterEntry> {
        SubnetRegistry::remove(pid)
    }

    pub(crate) fn register_root(pid: Principal) {
        SubnetRegistry::register_root(pid);
    }

    pub(crate) fn update_module_hash(pid: Principal, module_hash: Vec<u8>) -> bool {
        SubnetRegistry::update_module_hash(pid, module_hash)
    }

    // ---------------------------------------------------------------------
    // Queries
    // ---------------------------------------------------------------------

    #[must_use]
    pub fn get(pid: Principal) -> Option<CanisterEntry> {
        SubnetRegistry::get(pid)
    }

    #[must_use]
    pub fn get_parent(pid: Principal) -> Option<Principal> {
        SubnetRegistry::get_parent(pid)
    }

    #[must_use]
    pub(crate) fn children(pid: Principal) -> Vec<CanisterSummary> {
        SubnetRegistry::children(pid)
    }

    #[must_use]
    pub(crate) fn subtree(pid: Principal) -> Vec<CanisterSummary> {
        SubnetRegistry::subtree(pid)
    }

    #[must_use]
    pub fn export_view() -> CanisterChildrenView {
        SubnetRegistry::export().into()
    }

    // ---------------------------------------------------------------------
    // Traversal
    // ---------------------------------------------------------------------

    /// Return the canonical parent chain for a canister.
    ///
    /// The returned vector is ordered: root → ... → target.
    ///
    /// Invariants enforced:
    /// - No cycles
    /// - Bounded by registry size
    /// - Terminates at a ROOT canister
    pub fn parent_chain(target: Principal) -> Result<Vec<CanisterSummary>, Error> {
        let registry_len = SubnetRegistry::export().len();

        let mut chain = Vec::new();
        let mut seen: HashSet<Principal> = HashSet::new();
        let mut pid = target;

        loop {
            if !seen.insert(pid) {
                return Err(SubnetRegistryOpsError::ParentChainCycle(pid).into());
            }

            let Some(entry) = SubnetRegistry::get(pid) else {
                return Err(SubnetRegistryOpsError::CanisterNotFound(pid).into());
            };

            if seen.len() > registry_len {
                return Err(SubnetRegistryOpsError::ParentChainTooLong(seen.len()).into());
            }

            let summary: CanisterSummary = entry.clone().into();
            let parent = entry.parent_pid;

            chain.push(summary);

            if let Some(parent_pid) = parent {
                pid = parent_pid;
            } else {
                if entry.role != CanisterRole::ROOT {
                    return Err(SubnetRegistryOpsError::ParentChainNotRootTerminated(pid).into());
                }
                break;
            }
        }

        chain.reverse();
        Ok(chain)
    }
}

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

fn role_requires_singleton(role: &CanisterRole) -> bool {
    let cfg = ConfigOps::current_subnet_canister(role);
    cfg.cardinality == CanisterCardinality::Single
}

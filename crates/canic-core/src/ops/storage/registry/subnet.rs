use crate::{
    Error, ThisError,
    cdk::{types::Principal, utils::time::now_secs},
    config::schema::CanisterCardinality,
    ids::CanisterRole,
    ops::{config::ConfigOps, storage::registry::RegistryOpsError},
    storage::{
        canister::{CanisterEntry, CanisterSummary},
        memory::registry::subnet::{SubnetRegistry, SubnetRegistryData},
    },
};
use std::collections::HashSet;

///
/// SubnetRegistryOpsError
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
/// SubnetRegistrySnapshot
/// Internal, operational snapshot of the subnet registry.
///

#[derive(Clone, Debug)]
pub struct SubnetRegistrySnapshot {
    pub entries: Vec<(Principal, CanisterEntry)>,
}

impl From<SubnetRegistryData> for SubnetRegistrySnapshot {
    fn from(data: SubnetRegistryData) -> Self {
        Self {
            entries: data.entries,
        }
    }
}

impl From<SubnetRegistrySnapshot> for SubnetRegistryData {
    fn from(snapshot: SubnetRegistrySnapshot) -> Self {
        Self {
            entries: snapshot.entries,
        }
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

        if role_requires_singleton(role)?
            && let Some((existing_pid, _)) = SubnetRegistry::find_first_by_role(role)
        {
            return Err(SubnetRegistryOpsError::RoleAlreadyRegistered {
                role: role.clone(),
                pid: existing_pid,
            }
            .into());
        }

        let created_at = now_secs();
        SubnetRegistry::register(pid, role, parent_pid, module_hash, created_at);
        Ok(())
    }

    pub(crate) fn remove(pid: &Principal) -> Option<CanisterEntry> {
        SubnetRegistry::remove(pid)
    }

    pub(crate) fn register_root(pid: Principal) {
        let created_at = now_secs();
        SubnetRegistry::register_root(pid, created_at);
    }

    pub(crate) fn update_module_hash(pid: Principal, module_hash: Vec<u8>) -> bool {
        SubnetRegistry::update_module_hash(pid, module_hash)
    }

    // ---------------------------------------------------------------------
    // Queries (canonical data)
    // ---------------------------------------------------------------------

    #[must_use]
    pub(crate) fn get(pid: Principal) -> Option<CanisterEntry> {
        SubnetRegistry::get(pid)
    }

    #[must_use]
    pub(crate) fn is_registered(pid: Principal) -> bool {
        SubnetRegistry::get(pid).is_some()
    }

    #[must_use]
    pub fn get_parent(pid: Principal) -> Option<Principal> {
        SubnetRegistry::get_parent(pid)
    }

    /// Direct children (one level).
    #[must_use]
    pub(crate) fn children(pid: Principal) -> Vec<(Principal, CanisterSummary)> {
        SubnetRegistry::children(pid)
    }

    /// Full subtree rooted at `pid`.
    #[must_use]
    pub(crate) fn subtree(pid: Principal) -> Vec<(Principal, CanisterSummary)> {
        SubnetRegistry::subtree(pid)
    }

    // -------------------------------------------------------------
    // Snapshot
    // -------------------------------------------------------------

    #[must_use]
    pub fn snapshot() -> SubnetRegistrySnapshot {
        SubnetRegistry::export().into()
    }

    // -------------------------------------------------------------
    // Narrow projections (ops-safe)
    // -------------------------------------------------------------

    #[must_use]
    pub(crate) fn export_roles() -> Vec<(Principal, CanisterRole)> {
        SubnetRegistry::export()
            .entries
            .into_iter()
            .map(|(pid, entry)| (pid, entry.role))
            .collect()
    }

    // ---------------------------------------------------------------------
    // Traversal / invariants
    // ---------------------------------------------------------------------

    /// Return the canonical parent chain for a canister.
    ///
    /// Returned order: root → … → target
    ///
    /// Invariants enforced:
    /// - no cycles
    /// - bounded by registry size
    /// - terminates at ROOT
    pub(crate) fn parent_chain(
        target: Principal,
    ) -> Result<Vec<(Principal, CanisterSummary)>, Error> {
        let registry_len = SubnetRegistry::export().entries.len();

        let mut chain: Vec<(Principal, CanisterSummary)> = Vec::new();
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

            let summary = CanisterSummary::from(&entry);
            let parent = entry.parent_pid;

            chain.push((pid, summary));

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

fn role_requires_singleton(role: &CanisterRole) -> Result<bool, Error> {
    let cfg = ConfigOps::current_subnet_canister(role)?;
    Ok(cfg.cardinality == CanisterCardinality::Single)
}

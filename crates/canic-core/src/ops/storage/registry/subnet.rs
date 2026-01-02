use crate::{
    Error, ThisError,
    cdk::{types::Principal, utils::time::now_secs},
    config::schema::CanisterCardinality,
    ids::CanisterRole,
    ops::{config::ConfigOps, storage::StorageOpsError},
    storage::{
        canister::{CanisterEntry as ModelCanisterEntry, CanisterSummary as ModelCanisterSummary},
        memory::registry::subnet::{SubnetRegistry, SubnetRegistryData},
    },
};
use std::collections::{HashMap, HashSet};

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
        StorageOpsError::from(err).into()
    }
}

///
/// SubnetRegistrySnapshot
/// Internal, operational snapshot of the subnet registry.
///

#[derive(Clone, Debug)]
pub struct SubnetRegistrySnapshot {
    pub entries: Vec<(Principal, CanisterEntrySnapshot)>,
}

impl From<SubnetRegistryData> for SubnetRegistrySnapshot {
    fn from(data: SubnetRegistryData) -> Self {
        Self {
            entries: data
                .entries
                .into_iter()
                .map(|(pid, entry)| (pid, entry.into()))
                .collect(),
        }
    }
}

impl From<SubnetRegistrySnapshot> for SubnetRegistryData {
    fn from(snapshot: SubnetRegistrySnapshot) -> Self {
        Self {
            entries: snapshot
                .entries
                .into_iter()
                .map(|(pid, entry)| (pid, entry.into()))
                .collect(),
        }
    }
}

impl SubnetRegistrySnapshot {
    /// Return the canonical parent chain for a canister, using this snapshot.
    ///
    /// Returned order: root → … → target
    ///
    /// Invariants enforced:
    /// - no cycles
    /// - bounded by registry size
    /// - terminates at ROOT
    pub(crate) fn parent_chain(
        &self,
        target: Principal,
    ) -> Result<Vec<(Principal, CanisterSummarySnapshot)>, Error> {
        let registry_len = self.entries.len();
        let mut index = HashMap::new();

        for (pid, entry) in &self.entries {
            index.insert(*pid, entry.clone());
        }

        let mut chain: Vec<(Principal, CanisterSummarySnapshot)> = Vec::new();
        let mut seen: HashSet<Principal> = HashSet::new();
        let mut pid = target;

        loop {
            if !seen.insert(pid) {
                return Err(SubnetRegistryOpsError::ParentChainCycle(pid).into());
            }

            let entry = index
                .get(&pid)
                .ok_or(SubnetRegistryOpsError::CanisterNotFound(pid))?;

            if seen.len() > registry_len {
                return Err(SubnetRegistryOpsError::ParentChainTooLong(seen.len()).into());
            }

            let summary = CanisterSummarySnapshot::from(entry);
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

///
/// SubnetRegistryOps
///
/// Semantic operations over the subnet registry.
/// Enforces cardinality, parent-chain invariants, and traversal safety.
///
/// Invariant: non-root workflows must not call SubnetRegistryOps directly.
/// Non-root fanout should use the children cache populated by topology cascade.
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

    pub(crate) fn remove(pid: &Principal) -> Option<CanisterEntrySnapshot> {
        SubnetRegistry::remove(pid).map(Into::into)
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
    pub(crate) fn get(pid: Principal) -> Option<CanisterEntrySnapshot> {
        SubnetRegistry::get(pid).map(Into::into)
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
    pub(crate) fn children(pid: Principal) -> Vec<(Principal, CanisterSummarySnapshot)> {
        SubnetRegistry::children(pid)
            .into_iter()
            .map(|(pid, summary)| (pid, summary.into()))
            .collect()
    }

    /// Full subtree rooted at `pid`.
    #[must_use]
    pub(crate) fn subtree(pid: Principal) -> Vec<(Principal, CanisterSummarySnapshot)> {
        SubnetRegistry::subtree(pid)
            .into_iter()
            .map(|(pid, summary)| (pid, summary.into()))
            .collect()
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
}

#[derive(Clone, Debug)]
pub struct CanisterEntrySnapshot {
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
    pub created_at: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanisterSummarySnapshot {
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
}

impl From<ModelCanisterEntry> for CanisterEntrySnapshot {
    fn from(entry: ModelCanisterEntry) -> Self {
        Self {
            role: entry.role,
            parent_pid: entry.parent_pid,
            module_hash: entry.module_hash,
            created_at: entry.created_at,
        }
    }
}

impl From<CanisterEntrySnapshot> for ModelCanisterEntry {
    fn from(entry: CanisterEntrySnapshot) -> Self {
        Self {
            role: entry.role,
            parent_pid: entry.parent_pid,
            module_hash: entry.module_hash,
            created_at: entry.created_at,
        }
    }
}

impl From<ModelCanisterSummary> for CanisterSummarySnapshot {
    fn from(summary: ModelCanisterSummary) -> Self {
        Self {
            role: summary.role,
            parent_pid: summary.parent_pid,
        }
    }
}

impl From<&CanisterEntrySnapshot> for CanisterSummarySnapshot {
    fn from(entry: &CanisterEntrySnapshot) -> Self {
        Self {
            role: entry.role.clone(),
            parent_pid: entry.parent_pid,
        }
    }
}

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

fn role_requires_singleton(role: &CanisterRole) -> Result<bool, Error> {
    let cfg = ConfigOps::current_subnet_canister(role)?;
    Ok(cfg.cardinality == CanisterCardinality::Single)
}

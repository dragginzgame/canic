//! Module: discovery
//!
//! Responsibility: convert registry observations into backup target sets.
//! Does not own: snapshot IO, manifest validation, or persisted backup state.
//! Boundary: prepares discovery projections consumed by backup planning.

#[cfg(test)]
mod tests;

use crate::registry::RegistryEntry;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use thiserror::Error as ThisError;

///
/// SnapshotTarget
///
/// Registry target selected for snapshot capture.
/// Owned by backup discovery and consumed by snapshot planning.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotTarget {
    pub canister_id: String,
    pub role: Option<String>,
    pub parent_canister_id: Option<String>,
    pub module_hash: Option<String>,
}

///
/// DiscoveryError
///
/// Typed discovery failure returned while selecting backup targets.
/// Owned by backup discovery and surfaced to snapshot and planning callers.
///

#[derive(Debug, ThisError)]
pub enum DiscoveryError {
    #[error("registry JSON did not contain the requested canister {0}")]
    CanisterNotInRegistry(String),
}

/// Resolve selected target and children from registry entries.
pub fn targets_from_registry(
    registry: &[RegistryEntry],
    canister_id: &str,
    recursive: bool,
) -> Result<Vec<SnapshotTarget>, DiscoveryError> {
    let by_pid = registry
        .iter()
        .map(|entry| (entry.pid.as_str(), entry))
        .collect::<BTreeMap<_, _>>();

    let root = by_pid
        .get(canister_id)
        .ok_or_else(|| DiscoveryError::CanisterNotInRegistry(canister_id.to_string()))?;

    let mut targets = Vec::new();
    let mut seen = BTreeSet::new();
    targets.push(SnapshotTarget {
        canister_id: root.pid.clone(),
        role: root.role.clone(),
        parent_canister_id: root.parent_pid.clone(),
        module_hash: root.module_hash.clone(),
    });
    seen.insert(root.pid.clone());

    let mut queue = VecDeque::from([root.pid.clone()]);
    while let Some(parent) = queue.pop_front() {
        for child in registry
            .iter()
            .filter(|entry| entry.parent_pid.as_deref() == Some(parent.as_str()))
        {
            if seen.insert(child.pid.clone()) {
                targets.push(SnapshotTarget {
                    canister_id: child.pid.clone(),
                    role: child.role.clone(),
                    parent_canister_id: child.parent_pid.clone(),
                    module_hash: child.module_hash.clone(),
                });
                if recursive {
                    queue.push_back(child.pid.clone());
                }
            }
        }
    }

    Ok(targets)
}

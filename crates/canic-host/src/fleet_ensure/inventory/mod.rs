//! Module: fleet_ensure::inventory
//!
//! Responsibility: project terminal current-generation ensure state for operator tooling.
//! Does not own: historical install evidence, live observation, or topology decisions.
//! Boundary: only a terminal exact current ensure journal may become an operator inventory.

use crate::{
    fleet_ensure::{
        model::{DesiredCanisterKind, FleetEnsureCompletion, FleetEnsurePlan},
        ops::{EnsurePaths, EnsureStateError, read_journal, read_plan, read_state},
    },
    registry::RegistryEntry,
};
use std::{collections::BTreeMap, path::Path};
use thiserror::Error as ThisError;

/// Current terminal Fleet inventory shared by backup and read-only operator commands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentFleetInventory {
    pub active_registry: Option<canic_core::dto::fleet_registry::FleetRegistry>,
    pub entries: Vec<RegistryEntry>,
    pub roots: Vec<String>,
}

/// Terminal ensure inventory projected for current operator commands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentFleetResolution {
    pub active_registry: Option<canic_core::dto::fleet_registry::FleetRegistry>,
    pub plan: FleetEnsurePlan,
    pub registry: CurrentFleetRegistry,
    pub topology: CurrentFleetTopology,
}

/// Current registry-shaped entry list retained by the sole ensure owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentFleetRegistry {
    pub entries: Vec<RegistryEntry>,
}

/// Current typed role/topology projection for operator target selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentFleetTopology {
    pub coordinator_canister_id: String,
    pub fleet_subnet_root_canister_ids: Vec<String>,
    pub children_by_parent: BTreeMap<Option<String>, Vec<String>>,
    pub roles_by_canister: BTreeMap<String, String>,
}

/// Typed current-inventory resolution failure.
#[derive(Debug, ThisError)]
pub enum CurrentFleetInventoryError {
    #[error("Fleet {fleet} has no terminal current ensure operation in environment {environment}")]
    NotConverged { environment: String, fleet: String },

    #[error("current Fleet topology references missing parent {parent} for {canister}")]
    MissingParent { canister: String, parent: String },

    #[error("current Fleet topology has no retained Principal for {0}")]
    MissingPrincipal(String),

    #[error("current Fleet topology has no exact Coordinator")]
    MissingCoordinator,

    #[error("current Fleet topology retains one Principal under more than one role")]
    DuplicatePrincipal,

    #[error("current Fleet {fleet} has no exact retained ensure plan")]
    MissingPlan { fleet: String },

    #[error("current Fleet {fleet} terminal journal and retained plan authority conflict")]
    PlanAuthorityConflict { fleet: String },

    #[error("current Fleet {fleet} has no unique typed active Registry authority")]
    ProtocolAuthorityMissing { fleet: String },

    #[error("current Fleet {fleet} typed Registry authority conflicts with terminal topology")]
    ProtocolAuthorityConflict { fleet: String },

    #[error("current Fleet {fleet} has {root_count} Roots; select one exact Root principal")]
    AmbiguousFleetSubnetRoot { fleet: String, root_count: usize },

    #[error(transparent)]
    State(#[from] EnsureStateError),
}

impl CurrentFleetTopology {
    /// Return the only current Root for an operation whose maintained scope is singular.
    pub fn unique_fleet_subnet_root<'a>(
        &'a self,
        fleet: &str,
    ) -> Result<&'a str, CurrentFleetInventoryError> {
        match self.fleet_subnet_root_canister_ids.as_slice() {
            [root] => Ok(root),
            roots => Err(CurrentFleetInventoryError::AmbiguousFleetSubnetRoot {
                fleet: fleet.to_string(),
                root_count: roots.len(),
            }),
        }
    }
}

impl CurrentFleetResolution {
    /// Return the exact typed active Registry verified at terminal convergence.
    pub fn initial_active_registry(
        &self,
        fleet: &str,
    ) -> Result<&canic_core::dto::fleet_registry::FleetRegistry, CurrentFleetInventoryError> {
        let registry = self.active_registry.as_ref().ok_or_else(|| {
            CurrentFleetInventoryError::ProtocolAuthorityMissing {
                fleet: fleet.to_string(),
            }
        })?;
        let mut registry_roots = registry
            .fleet_subnet_roots
            .iter()
            .filter(|root| {
                root.status != canic_core::dto::fleet_registry::FleetSubnetRootStatus::Removed
            })
            .map(|root| root.fleet_subnet_root.to_text())
            .collect::<Vec<_>>();
        registry_roots.sort_unstable();
        if registry.authority.binding.coordinator.to_text() != self.topology.coordinator_canister_id
            || registry_roots != self.topology.fleet_subnet_root_canister_ids
        {
            return Err(CurrentFleetInventoryError::ProtocolAuthorityConflict {
                fleet: fleet.to_string(),
            });
        }
        Ok(registry)
    }
}

/// Resolve current operator targeting exclusively from a terminal ensure journal.
pub fn resolve_current_fleet(
    root: &Path,
    environment: &str,
    fleet: &str,
) -> Result<CurrentFleetResolution, CurrentFleetInventoryError> {
    let inventory = read_current_fleet_inventory(root, environment, fleet)?;
    let paths = EnsurePaths::under(root, environment, fleet);
    let journal =
        read_journal(&paths)?.ok_or_else(|| CurrentFleetInventoryError::NotConverged {
            environment: environment.to_string(),
            fleet: fleet.to_string(),
        })?;
    let plan = read_plan(&paths)?.ok_or_else(|| CurrentFleetInventoryError::MissingPlan {
        fleet: fleet.to_string(),
    })?;
    if plan.plan_sha256 != journal.plan_sha256
        || plan.environment != environment
        || plan.fleet != fleet
        || plan.operation_id != journal.operation_id
    {
        return Err(CurrentFleetInventoryError::PlanAuthorityConflict {
            fleet: fleet.to_string(),
        });
    }
    let coordinator_canister_id = inventory
        .entries
        .iter()
        .find(|entry| entry.role.as_deref() == Some("fleet_coordinator"))
        .map(|entry| entry.pid.clone())
        .ok_or(CurrentFleetInventoryError::MissingCoordinator)?;
    let mut children_by_parent = BTreeMap::<Option<String>, Vec<String>>::new();
    let mut roles_by_canister = BTreeMap::new();
    for entry in &inventory.entries {
        children_by_parent
            .entry(entry.parent_pid.clone())
            .or_default()
            .push(entry.pid.clone());
        if let Some(role) = &entry.role {
            roles_by_canister.insert(entry.pid.clone(), role.clone());
        }
    }
    for children in children_by_parent.values_mut() {
        children.sort_unstable();
    }
    Ok(CurrentFleetResolution {
        active_registry: inventory.active_registry,
        plan,
        registry: CurrentFleetRegistry {
            entries: inventory.entries,
        },
        topology: CurrentFleetTopology {
            coordinator_canister_id,
            fleet_subnet_root_canister_ids: inventory.roots,
            children_by_parent,
            roles_by_canister,
        },
    })
}

/// Read one terminal inventory without consulting deleted install plans or recovery state.
pub fn read_current_fleet_inventory(
    root: &Path,
    environment: &str,
    fleet: &str,
) -> Result<CurrentFleetInventory, CurrentFleetInventoryError> {
    let paths = EnsurePaths::under(root, environment, fleet);
    let journal =
        read_journal(&paths)?.ok_or_else(|| CurrentFleetInventoryError::NotConverged {
            environment: environment.to_string(),
            fleet: fleet.to_string(),
        })?;
    if journal.completion != FleetEnsureCompletion::Converged {
        return Err(CurrentFleetInventoryError::NotConverged {
            environment: environment.to_string(),
            fleet: fleet.to_string(),
        });
    }
    let state = read_state(&paths, fleet)?;
    project_current_fleet_inventory(&state)
}

/// Validate and project one candidate terminal state before it becomes public authority.
pub(super) fn project_current_fleet_inventory(
    state: &crate::fleet_ensure::model::FleetEnsureStateRecord,
) -> Result<CurrentFleetInventory, CurrentFleetInventoryError> {
    let mut entries = Vec::with_capacity(state.topology.len());
    let mut roots = Vec::new();
    for (name, topology) in &state.topology {
        let principal = state
            .principals
            .get(name)
            .ok_or_else(|| CurrentFleetInventoryError::MissingPrincipal(name.clone()))?;
        let parent_pid = topology
            .parent
            .as_ref()
            .map(|parent| {
                state.principals.get(parent).cloned().ok_or_else(|| {
                    CurrentFleetInventoryError::MissingParent {
                        canister: name.clone(),
                        parent: parent.clone(),
                    }
                })
            })
            .transpose()?;
        if topology.kind == DesiredCanisterKind::Root {
            roots.push(principal.clone());
        }
        entries.push(RegistryEntry {
            module_hash: topology.module_hash.clone(),
            parent_pid,
            pid: principal.clone(),
            protocol_binding: topology.protocol_binding.clone(),
            role: topology
                .role
                .clone()
                .or_else(|| Some(default_role(topology.kind, name))),
        });
    }
    entries.sort_by(|left, right| left.pid.cmp(&right.pid));
    if entries.windows(2).any(|pair| pair[0].pid == pair[1].pid) {
        return Err(CurrentFleetInventoryError::DuplicatePrincipal);
    }
    roots.sort();
    if let Some(registry) = &state.active_registry {
        let coordinator = entries
            .iter()
            .filter(|entry| entry.role.as_deref() == Some("fleet_coordinator"))
            .map(|entry| entry.pid.as_str())
            .collect::<Vec<_>>();
        let mut registry_roots = registry
            .fleet_subnet_roots
            .iter()
            .filter(|root| {
                root.status != canic_core::dto::fleet_registry::FleetSubnetRootStatus::Removed
            })
            .map(|root| root.fleet_subnet_root.to_text())
            .collect::<Vec<_>>();
        registry_roots.sort_unstable();
        if coordinator.as_slice() != [registry.authority.binding.coordinator.to_text()]
            || registry_roots != roots
        {
            return Err(CurrentFleetInventoryError::ProtocolAuthorityConflict {
                fleet: state.fleet.clone(),
            });
        }
    }
    Ok(CurrentFleetInventory {
        active_registry: state.active_registry.clone(),
        entries,
        roots,
    })
}

fn default_role(kind: DesiredCanisterKind, name: &str) -> String {
    match kind {
        DesiredCanisterKind::Coordinator => "fleet_coordinator".to_string(),
        DesiredCanisterKind::Root => "root".to_string(),
        DesiredCanisterKind::Store => "wasm_store".to_string(),
        DesiredCanisterKind::Pool => "canister_pool_asset".to_string(),
        DesiredCanisterKind::Auxiliary | DesiredCanisterKind::Component => name.to_string(),
    }
}

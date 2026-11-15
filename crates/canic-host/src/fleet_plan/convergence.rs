//! Resolve bounded serial child upgrades from retained fleet custody, without new effect owners.
use super::{FleetPlan, Slot, invalid, root_receipt};
use crate::{
    Error, Plan, Receipt, Result, managed, plan,
    store::{self, Store},
    supplier,
};
use crate::{PlanAction, UpgradeTarget};
use serde::Serialize;
use std::path::Path;

/// Result of comparing declared artifacts with this fleet's retained operations.
/// No-change is historical completion evidence, not a fresh network observation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum FleetPlanning {
    /// Create or resume the original Root-plus-child approval.
    Create {
        /// Original approval digest.
        operation: String,
        /// Exact immutable creation envelope.
        fleet_plan: FleetPlan,
    },
    /// Upgrade or resume the child through its original Root.
    Upgrade {
        /// Independently retained upgrade digest.
        operation: String,
        /// Existing single-child plan contract.
        plan: Plan,
    },
    /// Upgrade Root while retaining its complete child journal.
    Maintenance {
        /// Independently retained maintenance approval digest.
        operation: String,
        /// Direct upgrade with exact custody to preserve.
        plan: Plan,
    },
    /// Declared artifacts match the latest completed operation retained locally.
    Unchanged {
        /// Completion evidence used for the comparison.
        operation: String,
    },
}

/// A receipt alone cannot substitute for retained authenticated Ready evidence.
fn completed(dir: &Path, value: &Plan) -> Result<Option<Receipt>> {
    if !dir.join("receipt.json").try_exists().map_err(supplier)? {
        return Ok(None);
    }
    let receipt: Receipt = store::json(&dir.join("receipt.json"))?;
    let ready = match &value.action {
        PlanAction::MaintainRoot { target, .. } => {
            crate::maintenance::retained(dir, value)? && target.canister == receipt.canister
        }
        PlanAction::UpgradeChild { target, .. } => {
            managed::upgrade::evidence(dir, value)?
                .is_some_and(|s| s.phase == canic_management_protocol::upgrade::Phase::Ready)
                && target.canister == receipt.canister
        }
        PlanAction::CreateChild(_) => managed::evidence(dir, value)?.is_some_and(|s| {
            s.phase == canic_management_protocol::Phase::Ready
                && s.worker.as_ref() == Some(&receipt.canister)
        }),
        _ => return Err(invalid()),
    };
    if !ready
        || receipt.operation != value.digest()?
        || receipt.module_hash != value.artifact.sha256
        || receipt.controller != value.application_controller()
    {
        return Err(invalid());
    }
    Ok(Some(receipt))
}

fn child_completion(dir: &Path, fleet: &FleetPlan, index: Slot) -> Result<Option<(Plan, Receipt)>> {
    let Some(root) = root_receipt(dir, fleet)? else {
        return Ok(None);
    };
    if !dir
        .join(super::record::child_leaf(index))
        .join("plan.json")
        .try_exists()
        .map_err(supplier)?
    {
        return Ok(None);
    }
    let child = fleet.child_plan(&root, index)?;
    Ok(
        completed(&dir.join(super::record::child_leaf(index)), &child)?
            .map(|receipt| (child, receipt)),
    )
}

/// The creation reservation admits only its completed child's first upgrade.
/// Called inside the same journal lock used to publish the independent approval.
pub(crate) fn permits_upgrade(dir: &Path, fleet: &FleetPlan, proposed: &Plan) -> Result<bool> {
    let Some((child, receipt)) = child_completion(dir, fleet, Slot::First)? else {
        return Ok(false);
    };
    Ok(proposed.deployment == child.deployment
        && proposed.network == child.network
        && proposed.controller == child.controller
        && proposed.target() == Some(&receipt.canister)
        && proposed.previous_module() == Some(&receipt.module_hash)
        && proposed.creation_cycles() == 0
        && proposed
            .management()
            .zip(child.management())
            .is_some_and(|(new, old)| {
                new.root == old.root && new.root_module == old.root_module && new.slot == old.slot
            }))
}

struct Child {
    plan: Plan,
    receipt: Receipt,
    custody: canic_management_protocol::custody::Custody,
    module: String,
    dispatch: u64,
    version: Option<u64>,
    upgrade_bytes: usize,
}

pub(super) fn plan(
    store: &Store,
    id: &str,
    fleet: FleetPlan,
    desired: FleetPlan,
    bytes: (&[u8], &[Vec<u8>]),
    individual: &[(String, Plan)],
) -> Result<FleetPlanning> {
    let dir = store.operation(id)?;
    let mut children = Vec::new();
    for definition in &fleet.children {
        let index = definition.slot;
        let Some((child, receipt)) = child_completion(&dir, &fleet, index)? else {
            if !individual.is_empty()
                || desired.root.artifact != fleet.root.artifact
                || !desired
                    .children()
                    .map(|(_, c)| &c.artifact)
                    .eq(fleet.children().map(|(_, c)| &c.artifact))
            {
                return Err(Error::Blocked(format!(
                    "fleet creation {id} is unfinished; resume its original artifacts"
                )));
            }
            return Ok(FleetPlanning::Create {
                operation: id.into(),
                fleet_plan: fleet,
            });
        };
        let mut creation = managed::evidence(&dir.join(super::record::child_leaf(index)), &child)?
            .ok_or_else(invalid)?;
        creation.stable_pages = 0;
        creation.maintenance_candidate = false;
        children.push(Child {
            module: receipt.module_hash.clone(),
            plan: child,
            receipt,
            custody: canic_management_protocol::custody::Custody {
                creation,
                upgrades: Vec::new(),
            },
            dispatch: 0,
            version: None,
            upgrade_bytes: 0,
        });
    }
    let root = root_receipt(&dir, &fleet)?.ok_or_else(invalid)?.canister;
    let mut root_module = fleet.root.artifact.sha256.clone();
    let mut ordered: Vec<_> = individual.iter().collect();
    ordered.sort_by_key(|(_, value)| value.created_at_ns);
    let mut last_created = fleet.root.created_at_ns;
    let mut latest_id = id;
    for (position, (operation, value)) in ordered.iter().enumerate() {
        if value.created_at_ns <= last_created {
            return Err(invalid());
        }
        last_created = value.created_at_ns;
        if let Some(expected) = value.maintenance() {
            if !expected
                .iter()
                .eq(children.iter().map(|child| &child.custody))
                || value.deployment != fleet.root.deployment
                || value.network != fleet.root.network
                || value.controller != fleet.root.controller
                || value.target() != Some(&root)
                || value.previous_module() != Some(&root_module)
            {
                return Err(invalid());
            }
            if completed(&store.operation(operation)?, value)?.is_none() {
                if position + 1 != ordered.len() {
                    return Err(invalid());
                }
                if value.artifact != desired.root.artifact
                    || changed_children(&children, &desired).next().is_some()
                {
                    return Err(Error::Blocked(format!(
                        "Root maintenance {operation} is unfinished; resume its original artifacts"
                    )));
                }
                return Ok(FleetPlanning::Maintenance {
                    operation: operation.clone(),
                    plan: value.clone(),
                });
            }
            root_module = value.artifact.sha256.clone();
            latest_id = operation;
            continue;
        }
        let index = children
            .iter()
            .position(|c| c.plan.deployment == value.deployment)
            .ok_or_else(invalid)?;
        let child = &mut children[index];
        if value.network != child.plan.network
            || value.controller != child.plan.controller
            || value.target() != Some(&child.receipt.canister)
            || value.creation_cycles() != 0
            || !value
                .management()
                .zip(child.plan.management())
                .is_some_and(|(new, old)| {
                    new.root == old.root
                        && new.root_module == root_module
                        && new.slot == old.slot
                        && new.execute_at_ns > child.dispatch
                })
            || value.previous_module() != Some(&child.module)
            || child.custody.upgrades.len() >= canic_management_protocol::upgrade::MAX_UPGRADES
        {
            return Err(Error::Invalid(
                "upgrade history is not an ordered child/slot/code chain".into(),
            ));
        }
        let evidence = managed::upgrade::evidence(&store.operation(operation)?, value)?;
        if let (Some(old), Some(new)) = (
            child.version,
            evidence.as_ref().and_then(|s| s.previous_version),
        ) && new < old
        {
            return Err(invalid());
        }
        if completed(&store.operation(operation)?, value)?.is_none() {
            if position + 1 != ordered.len() {
                return Err(Error::Invalid(
                    "an unfinished upgrade has a successor".into(),
                ));
            }
            if value.artifact
                != desired
                    .children()
                    .nth(index)
                    .ok_or_else(invalid)?
                    .1
                    .artifact
                || desired.root.artifact.sha256 != root_module
                || changed_children(&children, &desired).any(|i| i != index)
            {
                return Err(Error::Blocked(format!(
                    "upgrade {operation} is unfinished; resume its original authority"
                )));
            }
            return Ok(FleetPlanning::Upgrade {
                operation: operation.clone(),
                plan: value.clone(),
            });
        }
        child.module = value.artifact.sha256.clone();
        child.dispatch = value.management().expect("validated owner").execute_at_ns;
        child.version = evidence.as_ref().and_then(|s| s.observed_version);
        child.upgrade_bytes = value.artifact.bytes;
        child.custody.upgrades.push(evidence.ok_or_else(invalid)?);
        latest_id = operation;
    }
    if desired.root.created_at_ns <= last_created {
        return Err(invalid());
    }
    let changed: Vec<_> = changed_children(&children, &desired).collect();
    if desired.root.artifact.sha256 != root_module {
        if !changed.is_empty() {
            return Err(Error::Blocked(
                "change Root and child artifacts in separate reviewed maintenance windows".into(),
            ));
        }
        let maintenance = Plan {
            action: PlanAction::MaintainRoot {
                target: UpgradeTarget {
                    canister: root,
                    module: root_module,
                },
                children: children.iter().map(|c| c.custody.clone()).collect(),
            },
            ..desired.root
        };
        let (operation, plan) = plan::publish_locked(store, maintenance, bytes.0)?;
        return Ok(FleetPlanning::Maintenance { operation, plan });
    }
    let Some(&index) = changed.first() else {
        return Ok(FleetPlanning::Unchanged {
            operation: latest_id.into(),
        });
    };
    if changed.len() != 1 {
        return Err(Error::Blocked(
            "change one child's artifact per reviewed upgrade approval".into(),
        ));
    }
    let selected = desired.children().nth(index).ok_or_else(invalid)?.1;
    let custody_bytes: usize = children
        .iter()
        .enumerate()
        .map(|(i, c)| {
            c.plan.artifact.bytes
                + if i == index {
                    selected.artifact.bytes
                } else {
                    c.upgrade_bytes
                }
        })
        .sum();
    if custody_bytes > canic_management_protocol::children::MAX_ARTIFACT_BYTES {
        return Err(Error::Invalid(
            "combined creation/current-upgrade artifacts exceed Root custody capacity".into(),
        ));
    }
    let child = children.remove(index);
    if child.custody.upgrades.len() == canic_management_protocol::upgrade::MAX_UPGRADES {
        return Err(Error::Blocked(
            "Root's eight-upgrade child history is full; preserve all evidence".into(),
        ));
    }
    if selected.execute_at_ns <= child.dispatch {
        return Err(Error::Invalid(
            "successor dispatch must follow the preceding upgrade".into(),
        ));
    }
    let mut owner = child.plan.management().cloned().ok_or_else(invalid)?;
    owner.execute_at_ns = selected.execute_at_ns;
    owner.root_module = root_module;
    let upgrade = Plan {
        artifact: selected.artifact.clone(),
        action: PlanAction::UpgradeChild {
            target: UpgradeTarget {
                canister: child.receipt.canister,
                module: child.module,
            },
            owner,
        },
        created_at_ns: desired.root.created_at_ns,
        expires_at_ns: desired.root.expires_at_ns,
        ..child.plan
    };
    let (operation, plan) = plan::publish_locked(store, upgrade, &bytes.1[index])?;
    Ok(FleetPlanning::Upgrade { operation, plan })
}
fn changed_children<'a>(
    children: &'a [Child],
    desired: &'a FleetPlan,
) -> impl Iterator<Item = usize> + 'a {
    desired
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, c)| (c.artifact.sha256 != children[i].module).then_some(i))
}

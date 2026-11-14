//! Resolve bounded serial child upgrades from retained fleet custody, without new effect owners.
use super::{FleetPlan, invalid, root_receipt};
use crate::{
    Error, Plan, Receipt, Result, managed, plan,
    store::{self, Store},
    supplier,
};
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
    let ready = if value.maintenance.is_some() {
        crate::maintenance::retained(dir, value)?
            && value.target.as_ref() == Some(&receipt.canister)
    } else if value.target.is_some() {
        managed::upgrade::evidence(dir, value)?
            .is_some_and(|s| s.phase == canic_management_protocol::upgrade::Phase::Ready)
            && value.target.as_ref() == Some(&receipt.canister)
    } else {
        managed::evidence(dir, value)?.is_some_and(|s| {
            s.phase == canic_management_protocol::Phase::Ready
                && s.worker.as_ref() == Some(&receipt.canister)
        })
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

fn child_completion(dir: &Path, fleet: &FleetPlan) -> Result<Option<(Plan, Receipt)>> {
    let Some(root) = root_receipt(dir, fleet)? else {
        return Ok(None);
    };
    if !dir.join("child/plan.json").try_exists().map_err(supplier)? {
        return Ok(None);
    }
    let child = fleet.child_plan(&root)?;
    Ok(completed(&dir.join("child"), &child)?.map(|receipt| (child, receipt)))
}

/// The creation reservation admits only its completed child's first upgrade.
/// Called inside the same journal lock used to publish the independent approval.
pub(crate) fn permits_upgrade(dir: &Path, fleet: &FleetPlan, proposed: &Plan) -> Result<bool> {
    let Some((child, receipt)) = child_completion(dir, fleet)? else {
        return Ok(false);
    };
    Ok(proposed.deployment == child.deployment
        && proposed.network == child.network
        && proposed.controller == child.controller
        && proposed.target.as_ref() == Some(&receipt.canister)
        && proposed.previous_module.as_ref() == Some(&receipt.module_hash)
        && proposed.creation_cycles == 0
        && proposed
            .management
            .as_ref()
            .zip(child.management.as_ref())
            .is_some_and(|(new, old)| new.root == old.root && new.root_module == old.root_module))
}

pub(super) fn plan(
    store: &Store,
    id: &str,
    fleet: FleetPlan,
    desired: FleetPlan,
    bytes: (&[u8], &[u8]),
    individual: &[(String, Plan)],
) -> Result<FleetPlanning> {
    let dir = store.operation(id)?;
    let Some((child, receipt)) = child_completion(&dir, &fleet)? else {
        if !individual.is_empty()
            || desired.child.artifact != fleet.child.artifact
            || desired.root.artifact != fleet.root.artifact
        {
            return Err(Error::Blocked(format!(
                "fleet creation {id} is unfinished; resume its original artifacts before planning an upgrade"
            )));
        }
        return Ok(FleetPlanning::Create {
            operation: id.into(),
            fleet_plan: fleet,
        });
    };
    // Publication time orders Root maintenance and child approvals under one journal lock.
    let mut ordered: Vec<_> = individual.iter().collect();
    ordered.sort_by_key(|(_, value)| value.created_at_ns);
    if ordered
        .iter()
        .filter(|(_, value)| value.management.is_some())
        .count()
        > canic_management_protocol::upgrade::MAX_UPGRADES
    {
        return Err(Error::Invalid("upgrade history bound exceeded".into()));
    }
    let mut root_module = fleet.root.artifact.sha256.clone();
    let root = child.management.as_ref().ok_or_else(invalid)?.root.clone();
    let mut custody = canic_management_protocol::custody::Custody {
        creation: managed::evidence(&dir.join("child"), &child)?.ok_or_else(invalid)?,
        upgrades: Vec::new(),
    };
    // Live page counts and the executable rehearsal marker are not durable journal fields.
    custody.creation.stable_pages = 0;
    custody.creation.maintenance_candidate = false;
    let mut last_created = fleet.root.created_at_ns;
    let mut previous = receipt.module_hash.clone();
    let mut last_dispatch = 0;
    let mut last_version = None;
    let mut latest_id = id;
    for (index, (upgrade_id, upgrade)) in ordered.iter().enumerate() {
        if upgrade.created_at_ns <= last_created {
            return Err(invalid());
        }
        last_created = upgrade.created_at_ns;
        if let Some(expected) = &upgrade.maintenance {
            if expected != &custody
                || upgrade.deployment != fleet.root.deployment
                || upgrade.network != child.network
                || upgrade.controller != child.controller
                || upgrade.target.as_ref() != Some(&root)
                || upgrade.previous_module.as_ref() != Some(&root_module)
            {
                return Err(invalid());
            }
            if completed(&store.operation(upgrade_id)?, upgrade)?.is_none() {
                if index + 1 != ordered.len() {
                    return Err(invalid());
                }
                if upgrade.artifact != desired.root.artifact
                    || desired.child.artifact.sha256 != previous
                {
                    return Err(Error::Blocked(format!(
                        "Root maintenance {upgrade_id} is unfinished; resume its original artifacts"
                    )));
                }
                return Ok(FleetPlanning::Maintenance {
                    operation: upgrade_id.clone(),
                    plan: upgrade.clone(),
                });
            }
            root_module = upgrade.artifact.sha256.clone();
            latest_id = upgrade_id;
            continue;
        }
        if upgrade.deployment != child.deployment
            || upgrade.network != child.network
            || upgrade.controller != child.controller
            || upgrade.target.as_ref() != Some(&receipt.canister)
            || upgrade.creation_cycles != 0
            || !upgrade
                .management
                .as_ref()
                .zip(child.management.as_ref())
                .is_some_and(|(new, old)| new.root == old.root && new.root_module == root_module)
            || upgrade.previous_module.as_ref() != Some(&previous)
            || upgrade
                .management
                .as_ref()
                .is_none_or(|o| o.execute_at_ns <= last_dispatch)
        {
            return Err(Error::Invalid(
                "upgrade history is not one ordered child/code chain".into(),
            ));
        }
        let evidence = managed::upgrade::evidence(&store.operation(upgrade_id)?, upgrade)?;
        if let (Some(version), Some(observed)) = (
            last_version,
            evidence.as_ref().and_then(|s| s.previous_version),
        ) && observed < version
        {
            return Err(invalid());
        }
        if completed(&store.operation(upgrade_id)?, upgrade)?.is_none() {
            if index + 1 != ordered.len() {
                return Err(Error::Invalid(
                    "an unfinished upgrade has a successor".into(),
                ));
            }
            if upgrade.artifact != desired.child.artifact
                || desired.root.artifact.sha256 != root_module
            {
                return Err(Error::Blocked(format!(
                    "upgrade {upgrade_id} is unfinished; resume its original authority"
                )));
            }
            return Ok(FleetPlanning::Upgrade {
                operation: upgrade_id.clone(),
                plan: upgrade.clone(),
            });
        }
        previous = upgrade.artifact.sha256.clone();
        last_dispatch = upgrade
            .management
            .as_ref()
            .expect("admitted owner")
            .execute_at_ns;
        last_version = evidence.as_ref().and_then(|s| s.observed_version);
        custody.upgrades.push(evidence.ok_or_else(invalid)?);
        latest_id = upgrade_id;
    }
    if desired.root.created_at_ns <= last_created {
        return Err(invalid());
    }
    if desired.root.artifact.sha256 != root_module {
        if desired.child.artifact.sha256 != previous {
            return Err(Error::Blocked(
                "change Root and child artifacts in separate reviewed maintenance windows".into(),
            ));
        }
        let maintenance = Plan {
            target: Some(root),
            previous_module: Some(root_module),
            creation_cycles: 0,
            maintenance: Some(custody),
            ..desired.root
        };
        let (operation, plan) = plan::publish_locked(store, maintenance, bytes.0)?;
        return Ok(FleetPlanning::Maintenance { operation, plan });
    }
    if desired.child.artifact.sha256 == previous {
        return Ok(FleetPlanning::Unchanged {
            operation: latest_id.into(),
        });
    }
    if custody.upgrades.len() == canic_management_protocol::upgrade::MAX_UPGRADES {
        return Err(Error::Blocked(
            "Root's eight-upgrade history is full; preserve all evidence".into(),
        ));
    }
    if desired.child.execute_at_ns <= last_dispatch {
        return Err(Error::Invalid(
            "successor dispatch must follow the preceding upgrade".into(),
        ));
    }
    let upgrade = Plan {
        artifact: desired.child.artifact,
        target: Some(receipt.canister),
        previous_module: Some(previous),
        creation_cycles: 0,
        created_at_ns: desired.root.created_at_ns,
        expires_at_ns: desired.root.expires_at_ns,
        management: child.management.map(|mut owner| {
            owner.execute_at_ns = desired.child.execute_at_ns;
            owner.root_module = root_module;
            owner
        }),
        ..child
    };
    let (operation, plan) = plan::publish_locked(store, upgrade, bytes.1)?;
    Ok(FleetPlanning::Upgrade { operation, plan })
}

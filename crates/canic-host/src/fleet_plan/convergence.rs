//! Resolve ordered child, Root and Store changes from retained fleet custody.
use super::{
    FleetPlan,
    history::{Child, History, store_custody},
    invalid,
};
use crate::{Error, Plan, PlanAction, Result, UpgradeTarget, plan, store::Store};
use serde::Serialize;

/// Result of comparing declared artifacts with this fleet's retained operations.
/// No-change is historical completion evidence, not a fresh network observation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum FleetPlanning {
    /// Offline review of a pool grant for supplied infrastructure. Creates no journal operation.
    ShardPool {
        /// Application and fleet whose pool would be configured.
        deployment: crate::DeploymentName,
        /// Configured replica and trust binding.
        network: crate::Network,
        /// Proposed controller, without verification of on-chain authority.
        controller: String,
        /// Complete input to the bound Root's configure_shards endpoint.
        approval: Box<canic_management_protocol::stored::ShardPool>,
        /// Exact Wire request bytes for the bound Root, ready for local qualification.
        request: Vec<u8>,
    },
    /// Independent pool approvals for a fleet with dedicated instances.
    PlacementPools {
        /// Application name.
        app: String,
        /// Fleet name.
        fleet: String,
        /// Selected network and trust anchor.
        network: crate::Network,
        /// Proposed controller.
        controller: String,
        /// Shared-shard grant when configured.
        #[serde(skip_serializing_if = "Option::is_none")]
        shards: Option<Box<crate::PoolGrant<canic_management_protocol::stored::ShardPool>>>,
        /// Dedicated-instance grant.
        instances: Box<crate::PoolGrant<canic_management_protocol::stored::InstancePool>>,
    },
    /// Create or resume the original Root-plus-child approval.
    Create {
        /// Original approval digest.
        operation: String,
        /// Exact immutable creation envelope.
        fleet_plan: FleetPlan,
    },
    /// Add one child under a separate approval without changing the original fleet envelope.
    AddChild {
        /// Independently retained creation digest.
        operation: String,
        /// Exact child identity, Root authority, artifact and finite creation grant.
        plan: Plan,
    },
    /// Upgrade or resume a child or Store through its original Root.
    Upgrade {
        /// Independently retained upgrade digest.
        operation: String,
        /// Existing Plan with the selected Root-owned upgrade authority.
        plan: Plan,
    },
    /// Publish a changed child artifact before approving its installation.
    Publication {
        /// Independently retained publication digest.
        operation: String,
        /// Exact release and Store authority, with no installation effect.
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

pub(super) fn plan(
    store: &Store,
    id: &str,
    fleet: FleetPlan,
    desired: FleetPlan,
    bytes: (&[u8], &[Vec<u8>], Option<&[u8]>),
    individual: &[(String, Plan)],
    publications: &[(String, Plan)],
) -> Result<FleetPlanning> {
    let Some(history) = super::history::load(store, id, &fleet, individual)? else {
        if !individual.is_empty()
            || desired.store.as_ref().map(|s| &s.artifact)
                != fleet.store.as_ref().map(|s| &s.artifact)
            || desired.root.artifact != fleet.root.artifact
            || !desired
                .children
                .iter()
                .map(|c| &c.artifact)
                .eq(fleet.children.iter().map(|c| &c.artifact))
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
    let History {
        mut children,
        root,
        root_module,
        store_binding,
        store_operation,
        store_history,
        last_created,
        latest_id,
        pending,
    } = history;
    if let Some((operation, plan)) = pending {
        let selected = desired
            .children
            .iter()
            .position(|c| plan.deployment.as_ref() == Some(&c.deployment));
        let child_matches = selected.is_some_and(|i| desired.children[i].artifact == plan.artifact)
            && changed_children(&children, &desired).all(|i| Some(i) == selected);
        let root_matches = desired.root.artifact.sha256 == root_module;
        let store_matches = desired.store.as_ref().map(|s| &s.artifact.sha256)
            == store_binding.as_ref().map(|s| &s.module);
        let result = match &plan.action {
            PlanAction::CreateChild(_) if child_matches && root_matches && store_matches => {
                FleetPlanning::AddChild { operation, plan }
            }
            PlanAction::UpgradeChild { .. } if child_matches && root_matches && store_matches => {
                FleetPlanning::Upgrade { operation, plan }
            }
            PlanAction::UpgradeStore { .. }
                if desired
                    .store
                    .as_ref()
                    .is_some_and(|s| s.artifact == plan.artifact)
                    && root_matches
                    && changed_children(&children, &desired).next().is_none() =>
            {
                FleetPlanning::Upgrade { operation, plan }
            }
            PlanAction::MaintainRoot { .. }
                if desired.root.artifact == plan.artifact
                    && store_matches
                    && changed_children(&children, &desired).next().is_none() =>
            {
                FleetPlanning::Maintenance { operation, plan }
            }
            _ => {
                return Err(Error::Blocked(format!(
                    "fleet operation {operation} is unfinished; resume its original authority"
                )));
            }
        };
        return Ok(result);
    }
    if desired.root.created_at_ns <= last_created {
        return Err(invalid());
    }
    let changed: Vec<_> = changed_children(&children, &desired).collect();
    if let Some(selected) = desired.children.get(children.len()) {
        if !changed.is_empty()
            || desired.root.artifact.sha256 != root_module
            || desired.store.as_ref().map(|s| &s.artifact.sha256)
                != store_binding.as_ref().map(|s| &s.module)
        {
            return Err(Error::Blocked(
                "add a child and change existing artifacts in separate reviewed windows".into(),
            ));
        }
        if store_binding.is_none()
            && selected.artifact.bytes
                + children
                    .iter()
                    .map(|c| c.plan.artifact.bytes + c.upgrade_bytes)
                    .sum::<usize>()
                > canic_management_protocol::children::MAX_ARTIFACT_BYTES
        {
            return Err(Error::Invalid(
                "combined creation/current-upgrade artifacts exceed Root custody capacity".into(),
            ));
        }
        let source = store_binding
            .map(|store| -> Result<_> {
                Ok(crate::StoredArtifact {
                    store,
                    manifest: selected.release.clone().ok_or_else(invalid)?,
                    maintenance: store_operation.as_ref().map(|s| s.approval.attempt.clone()),
                })
            })
            .transpose()?;
        let addition = Plan {
            artifact: selected.artifact.clone(),
            deployment: Some(selected.deployment.clone()),
            action: PlanAction::CreateChild(crate::ManagedOwner {
                arguments: None,
                blob_attempt: None,
                root,
                root_module,
                slot: Some(selected.slot),
                execute_at_ns: selected.execute_at_ns,
                source,
            }),
            ..desired.root
        };
        if let Some(publication) = publication(
            store,
            &addition,
            &bytes.1[children.len()],
            publications,
            &store_history,
            store_operation.as_ref(),
        )? {
            return Ok(publication);
        }
        let (operation, plan) = plan::publish_locked(store, addition, &bytes.1[children.len()])?;
        return Ok(FleetPlanning::AddChild { operation, plan });
    }
    if desired.store.as_ref().map(|s| &s.artifact.sha256)
        != store_binding.as_ref().map(|s| &s.module)
    {
        let sequence = store_history.len() + usize::from(store_operation.is_some()) + 1;
        if sequence > canic_management_protocol::store_maintenance::MAX_UPGRADES {
            return Err(Error::Blocked(
                "Root Store maintenance history is full; retained evidence cannot be evicted"
                    .into(),
            ));
        }
        if !changed.is_empty() || desired.root.artifact.sha256 != root_module {
            return Err(Error::Blocked(
                "change Store, Root and child artifacts in separate reviewed windows".into(),
            ));
        }
        let binding = store_binding.as_ref().ok_or_else(invalid)?;
        for (id, pending) in publications {
            if pending.network == fleet.root.network
                && pending.publication().is_some_and(|p| {
                    p.root.canister == root && p.store.canister == binding.canister
                })
            {
                let path = store.operation(id)?;
                let (_, bytes) = plan::load(&path, id)?;
                if crate::publication::receipt(
                    &path,
                    pending,
                    id,
                    &crate::publication::manifest(pending, &bytes)?,
                )?
                .is_none()
                {
                    return Err(Error::Blocked(format!(
                        "publication {id} is unfinished; complete it before Store maintenance"
                    )));
                }
            }
        }
        let selected = desired.store.as_ref().ok_or_else(invalid)?;
        let upgrade = Plan {
            artifact: selected.artifact.clone(),
            deployment: Some(selected.deployment.clone()),
            action: PlanAction::UpgradeStore {
                sequence: Some(sequence as u64),
                target: binding.clone(),
                root: UpgradeTarget {
                    canister: root,
                    module: root_module,
                },
            },
            ..desired.root
        };
        let (operation, plan) = plan::publish_locked(store, upgrade, bytes.2.ok_or_else(invalid)?)?;
        return Ok(FleetPlanning::Upgrade { operation, plan });
    }
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
                store: store_custody(
                    &children,
                    store_binding.as_ref(),
                    store_operation.as_ref(),
                    &store_history,
                )?
                .map(Box::new),
            },
            ..desired.root
        };
        let (operation, plan) = plan::publish_locked(store, maintenance, bytes.0)?;
        return Ok(FleetPlanning::Maintenance { operation, plan });
    }
    let Some(&index) = changed.first() else {
        return Ok(FleetPlanning::Unchanged {
            operation: latest_id,
        });
    };
    if changed.len() != 1 {
        return Err(Error::Blocked(
            "change one child's artifact per reviewed upgrade approval".into(),
        ));
    }
    let selected = desired.children().nth(index).ok_or_else(invalid)?.1;
    super::history::check_upgrade(
        &children,
        index,
        selected.artifact.bytes,
        selected.execute_at_ns,
    )?;
    let child = children.remove(index);
    let mut owner = child.plan.management().cloned().ok_or_else(invalid)?;
    owner.execute_at_ns = selected.execute_at_ns;
    owner.root_module = root_module;
    if let Some(source) = &mut owner.source {
        source.store = store_binding.ok_or_else(invalid)?;
        source.manifest = selected.release.clone().ok_or_else(invalid)?;
        source.maintenance = store_operation.as_ref().map(|s| s.approval.attempt.clone());
    }
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
    if let Some(publication) = publication(
        store,
        &upgrade,
        &bytes.1[index],
        publications,
        &store_history,
        store_operation.as_ref(),
    )? {
        return Ok(publication);
    }
    let (operation, plan) = plan::publish_locked(store, upgrade, &bytes.1[index])?;
    Ok(FleetPlanning::Upgrade { operation, plan })
}

fn publication(
    store: &Store,
    child: &Plan,
    bytes: &[u8],
    publications: &[(String, Plan)],
    history: &[canic_management_protocol::store_maintenance::Status],
    maintenance: Option<&canic_management_protocol::store_maintenance::Status>,
) -> Result<Option<FleetPlanning>> {
    if let Some(source) = child.management().and_then(|o| o.source.as_ref()) {
        let publication = super::releases::publication_plan(child, source)?;
        crate::publication::removal::permit_publication(&store.path, &publication)?;
        // Completed publication is artifact evidence, not permission to renew an old mutation.
        let mut published = false;
        for (id, old) in publications {
            if crate::publication::completed_release(
                &store.operation(id)?,
                old,
                &publication,
                &source.manifest,
                history.iter().chain(maintenance),
            )? {
                published = true;
                break;
            }
        }
        if !published {
            let (operation, plan) = plan::retain_locked(store, publication, bytes)?;
            if crate::publication::receipt(
                &store.operation(&operation)?,
                &plan,
                &operation,
                &source.manifest,
            )?
            .is_none()
            {
                return Ok(Some(FleetPlanning::Publication { operation, plan }));
            }
        }
    }
    Ok(None)
}

fn changed_children<'a>(
    children: &'a [Child],
    desired: &'a FleetPlan,
) -> impl Iterator<Item = usize> + 'a {
    desired
        .children
        .iter()
        .zip(children)
        .enumerate()
        .filter_map(|(i, (desired, child))| (desired.artifact.sha256 != child.module).then_some(i))
}

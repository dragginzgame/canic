//! One read-only reconstruction of fleet authority, shared by whole-fleet and named planning.
use super::{ChildId, FleetPlan, invalid, root_receipt};
use crate::{
    Error, Plan, PlanAction, Receipt, Result, UpgradeTarget, managed,
    store::{self, Store},
    supplier,
};
use std::path::Path;

/// Named upgrades use the same ordered custody as whole-fleet convergence.
/// The caller holds the journal lock through validation and retention.
pub(crate) fn permits_upgrade(
    store: &Store,
    id: &str,
    fleet: &FleetPlan,
    proposed: &mut Plan,
) -> Result<Option<Vec<String>>> {
    if !matches!(proposed.action, PlanAction::UpgradeChild { .. }) {
        return Ok(None);
    }
    let (ids, issues) = crate::fleet::operation_ids(&store.path, true)?;
    if !issues.is_empty() {
        return Err(invalid());
    }
    let mut individual = Vec::new();
    for operation in ids {
        let path = store.operation(&operation)?;
        if super::exists(&path)? {
            continue;
        }
        let (plan, _) = crate::plan::load(&path, &operation)?;
        if plan.application_call().is_none()
            && plan.auth().is_none()
            && plan.removal().is_none()
            && plan.publication().is_none()
            && plan
                .deployment
                .as_ref()
                .is_some_and(|name| fleet.reserves(name))
        {
            individual.push((operation, plan));
        }
    }
    let Some(history) = load(store, id, fleet, &individual)? else {
        return Ok(None);
    };
    let Some(index) = history
        .children
        .iter()
        .position(|c| c.plan.deployment == proposed.deployment)
    else {
        return Ok(None);
    };
    let child = &history.children[index];
    if proposed.network != child.plan.network
        || proposed.controller != child.plan.controller
        || proposed.target() != Some(&child.receipt.canister)
        || history.pending.as_ref().is_some_and(|(_, pending)| {
            !matches!(pending.action, PlanAction::UpgradeChild { .. })
                || pending.deployment != proposed.deployment
        })
    {
        return Ok(None);
    }
    let old = child.plan.management().ok_or_else(invalid)?;
    let PlanAction::UpgradeChild { owner, .. } = &mut proposed.action else {
        return Ok(None);
    };
    if owner.root != history.root
        || owner.root_module != history.root_module
        || owner.slot != old.slot
        || owner.source.store != history.store_binding
    {
        return Ok(None);
    }
    {
        let source = &mut owner.source;
        let maintenance = history
            .store_operation
            .as_ref()
            .map(|s| s.approval.attempt.clone());
        if source.maintenance.is_some() && source.maintenance != maintenance {
            return Ok(None);
        }
        source.maintenance = maintenance;
    }
    let latest = individual
        .iter()
        .filter(|(_, plan)| {
            matches!(plan.action, PlanAction::UpgradeChild { .. })
                && plan.deployment == proposed.deployment
        })
        .max_by_key(|(_, plan)| plan.created_at_ns);
    let replay = latest
        .filter(|(_, plan)| crate::plan::same_authority(plan, proposed))
        .map(|(id, _)| id);
    if replay.is_none() {
        if history.pending.is_some()
            || proposed.previous_module() != Some(&child.module)
            || proposed.created_at_ns <= history.last_created
        {
            return Ok(None);
        }
        check_upgrade(
            &history.children,
            index,
            proposed.management().ok_or_else(invalid)?.execute_at_ns,
        )?;
    }
    // Only completed predecessors of this child may stop reserving a successor's approval.
    // The latest matching operation stays in the ordinary exact-replay comparison below.
    Ok(Some(
        individual
            .iter()
            .filter(|(id, plan)| {
                Some(id) != replay
                    && plan.deployment == proposed.deployment
                    && matches!(plan.action, PlanAction::UpgradeChild { .. })
            })
            .map(|(id, _)| id.clone())
            .collect(),
    ))
}

pub(super) fn check_upgrade(children: &[Child], index: usize, dispatch: u64) -> Result<()> {
    let child = &children[index];

    if dispatch <= child.dispatch {
        return Err(Error::Invalid(
            "successor dispatch must follow the preceding upgrade".into(),
        ));
    }
    Ok(())
}

pub(super) struct History {
    pub(super) children: Vec<Child>,
    pub(super) root: String,
    pub(super) root_module: String,
    pub(super) store_binding: UpgradeTarget,
    pub(super) store_operation: Option<canic_management_protocol::store_maintenance::Status>,
    pub(super) store_history: Vec<canic_management_protocol::store_maintenance::Status>,
    pub(super) last_created: u64,
    pub(super) latest_id: String,
    pub(super) pending: Option<(String, Plan)>,
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

fn child_completion(
    dir: &Path,
    fleet: &FleetPlan,
    index: ChildId,
) -> Result<Option<(Plan, Receipt)>> {
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
    let child = fleet.resolved_child(
        dir,
        &root,
        &super::store_receipt(dir, fleet)?.ok_or_else(invalid)?,
        index,
    )?;
    Ok(
        completed(&dir.join(super::record::child_leaf(index)), &child)?
            .map(|receipt| (child, receipt)),
    )
}

pub(super) struct Child {
    pub(super) plan: Plan,
    pub(super) receipt: Receipt,
    pub(super) custody: canic_management_protocol::custody::Custody,
    sources: canic_management_protocol::stored::Sources,
    pub(super) module: String,
    pub(super) dispatch: u64,
    version: Option<u64>,
}

impl Child {
    fn retained(dir: &Path, plan: Plan, receipt: Receipt) -> Result<Self> {
        let (mut creation, source) =
            managed::evidence_with_source(dir, &plan)?.ok_or_else(invalid)?;
        let creation_source = source.ok_or_else(invalid)?;
        creation.stable_pages = 0;
        Ok(Self {
            module: receipt.module_hash.clone(),
            plan,
            receipt,
            custody: canic_management_protocol::custody::Custody {
                creation,
                upgrades: Vec::new(),
            },
            sources: canic_management_protocol::stored::Sources {
                creation: creation_source,
                upgrades: Vec::new(),
            },
            dispatch: 0,
            version: None,
        })
    }
}

/// Missing initial completion is distinct from an invalid or unfinished successor chain.
pub(super) fn load(
    store: &Store,
    id: &str,
    fleet: &FleetPlan,
    individual: &[(String, Plan)],
) -> Result<Option<History>> {
    let dir = store.operation(id)?;
    let Some(store_receipt) = super::store_receipt(&dir, fleet)? else {
        return Ok(None);
    };
    let mut children = Vec::new();
    for definition in &fleet.children {
        let Some((child, receipt)) = child_completion(&dir, fleet, definition.slot)? else {
            return Ok(None);
        };
        children.push(Child::retained(
            &dir.join(super::record::child_leaf(definition.slot)),
            child,
            receipt,
        )?);
    }
    let root = root_receipt(&dir, fleet)?.ok_or_else(invalid)?.canister;
    let mut root_module = fleet.root.artifact.sha256.clone();
    let mut store_binding = UpgradeTarget {
        canister: store_receipt.canister,
        module: store_receipt.module_hash,
    };
    let mut store_operation: Option<canic_management_protocol::store_maintenance::Status> = None;
    let mut store_history = Vec::new();
    let mut ordered: Vec<_> = individual.iter().collect();
    ordered.sort_by_key(|(_, value)| value.created_at_ns);
    let mut last_created = fleet.root.created_at_ns;
    let mut latest_id = id.to_owned();
    let mut pending = None;
    for (operation, value) in &ordered {
        if value.created_at_ns <= last_created {
            return Err(invalid());
        }
        last_created = value.created_at_ns;
        if let PlanAction::CreateChild(owner) = &value.action {
            let name = value.deployment.as_ref().ok_or_else(invalid)?;
            if owner.root != root
                || owner.root_module != root_module
                || Some(owner.slot) != ChildId::service(children.len())
                || !fleet.reserves(name)
                || value.deployment == fleet.root.deployment
                || fleet.store.deployment == *name
                || children
                    .iter()
                    .any(|c| c.plan.deployment == value.deployment)
                || value.network != fleet.root.network
                || value.controller != fleet.root.controller
                || owner.source.store != store_binding
                || owner.source.maintenance.as_ref()
                    != store_operation.as_ref().map(|s| &s.approval.attempt)
            {
                return Err(invalid());
            }
            let path = store.operation(operation)?;
            if let Some(receipt) = completed(&path, value)? {
                children.push(Child::retained(&path, value.clone(), receipt)?);
                latest_id = operation.clone();
                continue;
            }
            pending = Some((operation.clone(), value.clone()));
            break;
        }
        if let Some(owner) = value.store_upgrade_root() {
            let binding = &mut store_binding;
            if value.store_upgrade_sequence().ok_or_else(invalid)?
                != (store_history.len() + usize::from(store_operation.is_some()) + 1) as u64
                || owner.canister != root
                || owner.module != root_module
                || value.target() != Some(&binding.canister)
                || value.previous_module() != Some(&binding.module)
                || value.deployment.as_ref() != Some(&fleet.store.deployment)
                || value.network != fleet.root.network
                || value.controller != fleet.root.controller
            {
                return Err(invalid());
            }
            if let Some((receipt, evidence)) =
                crate::store_upgrade::receipt(&store.operation(operation)?, value)?
            {
                binding.module = receipt.module_hash;
                if let Some(previous) = store_operation.replace(evidence) {
                    store_history.push(previous);
                }
                latest_id = operation.clone();
                continue;
            }
            pending = Some((operation.clone(), value.clone()));
            break;
        }
        if let Some(expected) = value.maintenance() {
            if value.maintenance_store()
                != Some(&store_custody(
                    &children,
                    &store_binding,
                    store_operation.as_ref(),
                    &store_history,
                ))
                || !expected
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
                pending = Some((operation.clone(), value.clone()));
                break;
            }
            root_module = value.artifact.sha256.clone();
            latest_id = operation.clone();
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
                        && new.source.store == store_binding
                        && new.source.maintenance.as_ref()
                            == store_operation.as_ref().map(|s| &s.approval.attempt)
                        && new.execute_at_ns > child.dispatch
                })
            || value.previous_module() != Some(&child.module)
        {
            return Err(Error::Invalid(
                "upgrade history is not an ordered child/slot/code chain".into(),
            ));
        }
        let retained = managed::upgrade::evidence_with_source(&store.operation(operation)?, value)?;
        let evidence = retained.as_ref().map(|(status, _)| status);
        if let (Some(old), Some(new)) = (child.version, evidence.and_then(|s| s.previous_version))
            && new < old
        {
            return Err(invalid());
        }
        if completed(&store.operation(operation)?, value)?.is_none() {
            pending = Some((operation.clone(), value.clone()));
            break;
        }
        child.module = value.artifact.sha256.clone();
        child.dispatch = value.management().expect("validated owner").execute_at_ns;
        child.version = evidence.and_then(|s| s.observed_version);
        let (evidence, source) = retained.ok_or_else(invalid)?;
        child.sources.upgrades.push(source.ok_or_else(invalid)?);
        child.custody.upgrades.push(evidence);
        latest_id = operation.clone();
    }
    if pending
        .as_ref()
        .is_some_and(|(id, _)| ordered.last().is_some_and(|(last, _)| last != id))
    {
        return Err(Error::Invalid(
            "an unfinished fleet operation has a successor".into(),
        ));
    }
    Ok(Some(History {
        children,
        root,
        root_module,
        store_binding,
        store_operation,
        store_history,
        last_created,
        latest_id,
        pending,
    }))
}

pub(super) fn store_custody(
    children: &[Child],
    binding: &UpgradeTarget,
    operation: Option<&canic_management_protocol::store_maintenance::Status>,
    history: &[canic_management_protocol::store_maintenance::Status],
) -> crate::StoreMaintenance {
    crate::StoreMaintenance {
        operation: operation.cloned(),
        history: history.to_vec(),
        store: binding.clone(),
        sources: children.iter().map(|c| Some(c.sources.clone())).collect(),
    }
}

//! Host binding to an already published release; Root owns pins and installation effects.
use crate::{
    Error, Plan, PlanInput, Result, SelectedFleet, UpgradeTarget, network::principal, operation,
    plan, store, supplier,
};
use canic_management_protocol::{
    Status, application,
    children::{ChildId, Request},
    stored as wire, upgrade,
};
use ic_platform::native::ReadClient;
use ic_wasm_store_protocol::Manifest;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Immutable artifact supplier selected by a managed deployment approval.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StoredArtifact {
    /// Root-controlled Store identity and exact reviewed code.
    pub store: UpgradeTarget,
    /// Complete published release, including ordered content commitments.
    pub manifest: Manifest,
    /// Completed Store replacement selected from authenticated fleet history.
    /// Selects complete Root custody; absence preserves the original read and digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintenance: Option<ic_wasm_store_protocol::MaintenanceAttempt>,
}
impl StoredArtifact {
    fn reference(&self) -> wire::Reference {
        wire::Reference {
            store: self.store.canister.clone(),
            manifest: self.manifest.clone(),
        }
    }
}

fn invalid() -> Error {
    Error::Invalid(
        "Store release differs from the approved child, supplier or retained custody".into(),
    )
}

pub(crate) fn validate(plan: &Plan, source: &StoredArtifact) -> Result<()> {
    let owner = plan.management().ok_or_else(invalid)?;
    let id = principal(&source.store.canister)?;
    if !source.manifest.valid()
        || source.manifest.bytes != plan.artifact.bytes as u64
        || crate::hex(&source.manifest.hash) != plan.artifact.sha256
        || plan
            .deployment
            .as_ref()
            .is_none_or(|d| d.canister != source.manifest.release.name)
        || owner.root == source.store.canister
        || id.to_text() != source.store.canister
        || id == principal("aaaaa-aa")?
        || id == principal("2vxsx-fae")?
        || crate::unhex(&source.store.module)?.len() != 32
        || source.store.module != source.store.module.to_ascii_lowercase()
        || source.maintenance.as_ref().is_some_and(|attempt| {
            !(1..=canic_management_protocol::store_maintenance::MAX_UPGRADES as u64)
                .contains(&attempt.sequence)
                || attempt.operation == [0; 32]
        })
    {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn select(
    journal: &Path,
    id: &str,
    input: &PlanInput,
    selection: &SelectedFleet,
    canister: &str,
    root: &str,
    root_module: &str,
) -> Result<StoredArtifact> {
    let journal = store::Store::open(journal)?;
    let path = journal.operation(id)?;
    let (publication, bytes) = plan::load(&path, id)?;
    let binding = publication.publication().ok_or_else(invalid)?;
    let name = publication.deployment.as_ref().ok_or_else(invalid)?;
    if publication.network != input.network
        || publication.controller != input.controller
        || name.app != selection.app
        || name.fleet != selection.fleet
        || name.canister != canister
        || binding.root.canister != root
        || binding.root.module != root_module
        || !selection.config.canisters.values().any(|s| {
            s.role.as_deref() == Some("wasm-store")
                && s.id.as_ref() == Some(&binding.store.canister)
                && s.parent == selection.config.canisters[canister].parent
        })
    {
        return Err(invalid());
    }
    crate::publication::removal::permit_publication(&journal.path, &publication)?;
    let manifest = crate::publication::manifest(&publication, &bytes)?;
    if crate::publication::receipt(&path, &publication, id, &manifest)?.is_none() {
        return Err(Error::Blocked(
            "release publication is incomplete; apply or reconcile its original operation".into(),
        ));
    }
    Ok(StoredArtifact {
        store: binding.store.clone(),
        manifest,
        maintenance: None,
    })
}

pub(super) async fn verify(plan: &Plan, client: &ReadClient) -> Result<()> {
    if let Some(source) = plan.management().map(|m| &m.source) {
        verify_store(client, plan.application_controller(), &source.store).await?;
    }
    Ok(())
}

pub(crate) async fn verify_store(
    client: &ReadClient,
    root: &str,
    store: &UpgradeTarget,
) -> Result<()> {
    let id = principal(&store.canister)?;
    operation::controllers(client, id, root).await?;
    if crate::network::subnet(client, &store.canister).await?
        != crate::network::subnet(client, root).await?
    {
        return Err(Error::Blocked(
            "Store delivery requires the approved Root and Store on the same subnet".into(),
        ));
    }
    if operation::module(client, id).await? != store.module {
        return Err(Error::Blocked(
            "Store code differs from the approved artifact supplier".into(),
        ));
    }
    Ok(())
}

fn stage<A>(plan: &Plan, approval: A) -> Result<wire::Stage<A>> {
    Ok(wire::Stage {
        approval,
        reference: plan
            .management()
            .map(|m| &m.source)
            .ok_or_else(invalid)?
            .reference(),
    })
}

fn slot(plan: &Plan) -> Result<ChildId> {
    plan.management().map(|m| m.slot).ok_or_else(invalid)
}

pub(super) fn arguments(plan: &Plan, method: u64) -> Result<Vec<u8>> {
    use application::Deployment::{Application, Stored};
    let frame = ic_wire::Encoding::Frame {
        method,
        direction: ic_wire::Direction::Request,
    };
    let slot = slot(plan)?;
    let configured = plan.management().is_some_and(|m| m.arguments.is_some());
    match method {
        application::ACTIVATE => ic_wire::encode(
            &Request {
                slot,
                value: application_upgrade(plan)?,
            },
            frame,
            application::LIMITS,
        )
        .map_err(supplier),
        application::CUSTODY | application::CHILD_EVIDENCE => {
            ic_wire::encode(&(), frame, application::LIMITS).map_err(supplier)
        }
        application::STAGE_CHILD_CREATION | application::RECONCILE_CHILD_CREATION => {
            let approval = super::approval(plan)?;
            let value: application::Creation = if configured {
                Application(application_stage(plan, approval)?)
            } else {
                Stored(stage(plan, approval)?)
            };
            ic_wire::encode(&Request { slot, value }, frame, wire::LIMITS).map_err(supplier)
        }
        application::STAGE_CHILD_UPGRADE | application::RECONCILE_CHILD_UPGRADE => {
            let value: application::Replacement = if configured {
                Application(application_upgrade(plan)?)
            } else {
                Stored(stage(plan, super::upgrade::approval(plan)?)?)
            };
            ic_wire::encode(&Request { slot, value }, frame, wire::LIMITS).map_err(supplier)
        }
        _ => Err(invalid()),
    }
}

fn custody(plan: &Plan, bytes: &[u8]) -> Result<crate::root_custody::Snapshot> {
    let source = plan
        .management()
        .map(|owner| &owner.source)
        .ok_or_else(invalid)?;
    let value = {
        let value = crate::root_custody::decode(super::Method::Observe.id(plan), bytes)?;
        let lifecycle = value
            .applications
            .get(slot(plan)?.service_index().ok_or_else(invalid)?)
            .ok_or_else(invalid)?;
        if let Some(lifecycle) = lifecycle {
            if plan.target().is_none() {
                if Some(lifecycle.creation.as_slice())
                    != plan.management().and_then(|m| m.arguments.as_deref())
                {
                    return Err(invalid());
                }
            } else if let Some(index) = value
                .root
                .current
                .stored
                .children
                .get(slot(plan)?.service_index().ok_or_else(invalid)?)
                .ok_or_else(invalid)?
                .upgrades
                .iter()
                .position(|s| s.approval.as_ref() == super::upgrade::approval(plan).ok().as_ref())
            {
                let maintenance = lifecycle.upgrades.get(index).ok_or_else(invalid)?;
                if Some(maintenance.arguments.as_slice())
                    != plan.management().and_then(|m| m.arguments.as_deref())
                    || Some(maintenance.attempt) != plan.management().and_then(|m| m.blob_attempt)
                    || maintenance.operator != plan.controller
                {
                    return Err(invalid());
                }
            }
        } else if plan.management().is_some_and(|m| m.arguments.is_some())
            && value
                .root
                .current
                .stored
                .children
                .get(slot(plan)?.service_index().ok_or_else(invalid)?)
                .ok_or_else(invalid)?
                .creation
                != Status::default()
        {
            return Err(invalid());
        }
        crate::root_custody::Snapshot {
            children: crate::root_custody::children(value.root.current.stored)?,
            store_operation: value.root.current.store_operation,
            store_history: value.root.store_history,
        }
    };
    if let Some(attempt) = &source.maintenance {
        if value.store_history.len() as u64 + 1 != attempt.sequence {
            return Err(invalid());
        }
        let operation = value.store_operation.as_ref().ok_or_else(invalid)?;
        if &operation.approval.attempt != attempt
            || operation.approval.store != source.store.canister
            || crate::hex(&operation.approval.module_hash) != source.store.module
            || operation.operator != plan.controller
            || !crate::store_upgrade::ready(operation)
        {
            return Err(invalid());
        }
    }
    if source.maintenance.is_none()
        && (value.store_operation.is_some() || !value.store_history.is_empty())
    {
        return Err(invalid());
    }
    Ok(value)
}

pub(crate) fn delivery<A: Serialize>(
    slot: ChildId,
    expected: &wire::Stage<A>,
    method: u64,
    actual: &wire::Delivery,
    ready: bool,
) -> Result<()> {
    if actual.reference != expected.reference
        || actual.pin() != expected.pin(method, slot)
        || (ready && actual.state != wire::PinState::Retained)
    {
        return Err(invalid());
    }
    Ok(())
}

fn application_stage<A>(plan: &Plan, approval: A) -> Result<application::Stage<A>> {
    Ok(application::Stage {
        source: stage(plan, approval)?,
        arguments: application::Arguments::new(
            plan.management()
                .and_then(|m| m.arguments.clone())
                .ok_or_else(invalid)?,
        )
        .map_err(supplier)?,
    })
}

fn application_upgrade(plan: &Plan) -> Result<application::Upgrade> {
    Ok(application::Upgrade {
        source: application_stage(plan, super::upgrade::approval(plan)?)?,
        attempt: plan
            .management()
            .and_then(|m| m.blob_attempt)
            .ok_or_else(invalid)?,
    })
}

pub(super) fn maintenance(plan: &Plan, bytes: &[u8]) -> Result<Option<application::Maintenance>> {
    custody(plan, bytes)?;
    let value: application::Custody = ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: super::Method::Observe.id(plan),
            direction: ic_wire::Direction::Response,
        },
        application::LIMITS,
    )
    .map_err(supplier)?;
    let expected = super::upgrade::approval(plan)?;
    let index = value
        .root
        .current
        .stored
        .children
        .get(slot(plan)?.service_index().ok_or_else(invalid)?)
        .ok_or_else(invalid)?
        .upgrades
        .iter()
        .position(|s| s.approval.as_ref() == Some(&expected));
    index
        .map(|i| {
            value
                .applications
                .get(slot(plan)?.service_index().ok_or_else(invalid)?)
                .ok_or_else(invalid)?
                .as_ref()
                .and_then(|l| l.upgrades.get(i))
                .cloned()
                .ok_or_else(invalid)
        })
        .transpose()
}

pub(crate) fn maintenance_evidence(
    dir: &Path,
    plan: &Plan,
) -> Result<Option<application::Maintenance>> {
    if plan.management().is_none_or(|m| m.blob_attempt.is_none()) {
        return Ok(None);
    }
    let mut latest = None;
    for index in 0..super::OBSERVATIONS {
        let path = dir.join(format!("observe/{index}/reply.bin"));
        if path.try_exists().map_err(supplier)?
            && let Some(value) = maintenance(plan, &store::read(&path, application::LIMITS.bytes)?)?
        {
            if !value.valid() {
                return Err(invalid());
            }
            if let Some(old) = &latest {
                let old: &application::Maintenance = old;
                if old.phase as u8 > value.phase as u8
                    || old
                        .requests
                        .iter()
                        .zip(value.requests)
                        .any(|(a, b)| a.is_some() && *a != b)
                    || old
                        .before
                        .as_ref()
                        .is_some_and(|b| Some(b) != value.before.as_ref())
                    || old
                        .after
                        .as_ref()
                        .is_some_and(|b| Some(b) != value.after.as_ref())
                    || old
                        .activator
                        .as_ref()
                        .is_some_and(|a| Some(a) != value.activator.as_ref())
                {
                    return Err(invalid());
                }
            }
            latest = Some(value);
        }
    }
    Ok(latest)
}

/// Combine already source-validated observations without forgetting a committed
/// pin or replacing the original cache deadline. Partial recovery replies carry
/// no delivery fields; they cannot revoke evidence retained by an earlier read.
pub(super) fn retain_delivery(
    previous: Option<&wire::Delivery>,
    observed: Option<wire::Delivery>,
) -> Result<Option<wire::Delivery>> {
    let Some(observed) = observed else {
        return Ok(previous.cloned());
    };
    if let Some(previous) = previous
        && (matches!(
            (previous.state, observed.state),
            (wire::PinState::Issued, wire::PinState::Pending)
                | (
                    wire::PinState::Retained,
                    wire::PinState::Pending | wire::PinState::Issued
                )
        ) || (previous.cache_deadline.is_some()
            && previous.cache_deadline != observed.cache_deadline))
    {
        return Err(Error::Invalid(
            "Root delivery evidence regressed or replaced the original cache deadline".into(),
        ));
    }
    Ok(Some(observed))
}

pub(super) fn creation(plan: &Plan, bytes: &[u8]) -> Result<Status> {
    creation_evidence(plan, bytes).map(|(status, _)| status)
}

pub(super) fn creation_evidence(
    plan: &Plan,
    bytes: &[u8],
) -> Result<(Status, Option<wire::Delivery>)> {
    let value = custody(plan, bytes)?;
    let child = value.child(slot(plan)?)?;
    let status = &child.journal.creation;
    if status == &Status::default() && child.sources.is_none() {
        return Ok((status.clone(), None));
    }
    let source = child.sources.as_ref().ok_or_else(invalid)?;
    if plan.management().is_some_and(|m| m.arguments.is_some()) {
        let expected = application_stage(plan, super::approval(plan)?)?;
        if source.creation.reference != expected.source.reference
            || source.creation.pin() != expected.pin(application::STAGE, slot(plan)?)
            || (status.phase == canic_management_protocol::Phase::Ready
                && source.creation.state != wire::PinState::Retained)
        {
            return Err(invalid());
        }
    } else {
        delivery(
            slot(plan)?,
            &stage(plan, super::approval(plan)?)?,
            wire::STAGE,
            &source.creation,
            status.phase == canic_management_protocol::Phase::Ready,
        )?;
    }
    Ok((status.clone(), Some(source.creation.clone())))
}

pub(super) fn upgraded(plan: &Plan, bytes: &[u8]) -> Result<upgrade::Status> {
    upgrade_evidence(plan, bytes).map(|(status, _)| status)
}

pub(super) fn upgrade_evidence(
    plan: &Plan,
    bytes: &[u8],
) -> Result<(upgrade::Status, Option<wire::Delivery>)> {
    let value = custody(plan, bytes)?;
    let child = value.child(slot(plan)?)?;
    let journal = &child.journal;
    let expected = super::upgrade::approval(plan)?;
    let Some(position) = journal
        .upgrades
        .iter()
        .position(|s| s.approval.as_ref() == Some(&expected))
    else {
        return Ok((upgrade::Status::default(), None));
    };
    let sources = child.sources.as_ref().ok_or_else(invalid)?;
    if sources.upgrades.len() != journal.upgrades.len() {
        return Err(invalid());
    }
    let status = &journal.upgrades[position];
    if plan.management().is_some_and(|m| m.arguments.is_some()) {
        let expected = application_upgrade(plan)?;
        if sources.upgrades[position].pin() != expected.pin(slot(plan)?)
            || sources.upgrades[position].reference != expected.source.source.reference
            || (status.phase == upgrade::Phase::Ready
                && sources.upgrades[position].state != wire::PinState::Retained)
        {
            return Err(invalid());
        }
    } else {
        delivery(
            slot(plan)?,
            &stage(plan, expected)?,
            wire::STAGE_UPGRADE,
            &sources.upgrades[position],
            status.phase == upgrade::Phase::Ready,
        )?;
    }
    Ok((status.clone(), Some(sources.upgrades[position].clone())))
}

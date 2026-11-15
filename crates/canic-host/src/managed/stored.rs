//! Host binding to an already published release; Root owns pins and installation effects.
use crate::{
    Error, Plan, PlanInput, Result, SelectedFleet, UpgradeTarget, network::principal, operation,
    plan, store, supplier,
};
use canic_management_protocol::{
    Status,
    children::{Request, Slot},
    stored as wire, upgrade,
};
use canic_wasm_store_protocol::Manifest;
use ic_platform::agent::Agent;
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
    pub maintenance: Option<canic_wasm_store_protocol::MaintenanceAttempt>,
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
        || source
            .maintenance
            .as_ref()
            .is_some_and(|attempt| attempt.sequence != 1 || attempt.operation == [0; 32])
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

pub(super) async fn verify(plan: &Plan, agent: &Agent) -> Result<()> {
    if let Some(source) = plan.management().and_then(|m| m.source.as_ref()) {
        verify_store(agent, plan.application_controller(), &source.store).await?;
    }
    Ok(())
}
pub(crate) async fn verify_store(agent: &Agent, root: &str, store: &UpgradeTarget) -> Result<()> {
    let id = principal(&store.canister)?;
    operation::controllers(agent, id, root).await?;
    if crate::network::subnet(agent, &store.canister).await?
        != crate::network::subnet(agent, root).await?
    {
        return Err(Error::Blocked(
            "Store delivery requires the approved Root and Store on the same subnet".into(),
        ));
    }
    if operation::module(agent, id).await? != store.module {
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
            .and_then(|m| m.source.as_ref())
            .ok_or_else(invalid)?
            .reference(),
    })
}
fn slot(plan: &Plan) -> Slot {
    plan.management()
        .and_then(|m| m.slot)
        .unwrap_or(Slot::First)
}
pub(super) fn arguments(plan: &Plan, method: u64) -> Result<Vec<u8>> {
    match method {
        wire::CUSTODY | canic_management_protocol::custody::ROOT => {
            ic_wire::native::encode_request(method, &(), wire::LIMITS).map_err(supplier)
        }
        wire::STAGE | wire::RECONCILE => ic_wire::native::encode_request(
            method,
            &Request {
                slot: slot(plan),
                value: stage(plan, super::approval(plan)?)?,
            },
            wire::LIMITS,
        )
        .map_err(supplier),
        wire::STAGE_UPGRADE | wire::RECONCILE_UPGRADE => ic_wire::native::encode_request(
            method,
            &Request {
                slot: slot(plan),
                value: stage(plan, super::upgrade::approval(plan)?)?,
            },
            wire::LIMITS,
        )
        .map_err(supplier),
        _ => Err(invalid()),
    }
}
fn custody(plan: &Plan, bytes: &[u8]) -> Result<wire::Custody> {
    let source = plan
        .management()
        .and_then(|owner| owner.source.as_ref())
        .ok_or_else(invalid)?;
    if let Some(attempt) = &source.maintenance {
        let value: canic_management_protocol::custody::Root = ic_wire::native::decode_response(
            canic_management_protocol::custody::ROOT,
            bytes,
            canic_management_protocol::custody::ROOT_LIMITS,
        )
        .map_err(supplier)?;
        let operation = value.store_operation.as_ref().ok_or_else(invalid)?;
        if &operation.approval.attempt != attempt
            || operation.approval.store != source.store.canister
            || crate::hex(&operation.approval.module_hash) != source.store.module
            || operation.operator != plan.controller
            || !crate::store_upgrade::ready(operation)
        {
            return Err(invalid());
        }
        return Ok(value.stored);
    }
    ic_wire::native::decode_response(wire::CUSTODY, bytes, wire::LIMITS).map_err(supplier)
}
pub(crate) fn delivery<A: Serialize>(
    slot: Slot,
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
pub(super) fn creation(plan: &Plan, bytes: &[u8]) -> Result<Status> {
    creation_evidence(plan, bytes).map(|(status, _)| status)
}
pub(super) fn creation_evidence(
    plan: &Plan,
    bytes: &[u8],
) -> Result<(Status, Option<wire::Delivery>)> {
    let value = custody(plan, bytes)?;
    let index = slot(plan).index();
    let status = &value.children[index].creation;
    if status == &Status::default() && value.sources[index].is_none() {
        return Ok((status.clone(), None));
    }
    let source = value.sources[index].as_ref().ok_or_else(invalid)?;
    delivery(
        slot(plan),
        &stage(plan, super::approval(plan)?)?,
        wire::STAGE,
        &source.creation,
        status.phase == canic_management_protocol::Phase::Ready,
    )?;
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
    let index = slot(plan).index();
    let journal = &value.children[index];
    let expected = super::upgrade::approval(plan)?;
    let Some(position) = journal
        .upgrades
        .iter()
        .position(|s| s.approval.as_ref() == Some(&expected))
    else {
        return Ok((upgrade::Status::default(), None));
    };
    let sources = value.sources[index].as_ref().ok_or_else(invalid)?;
    if sources.upgrades.len() != journal.upgrades.len() {
        return Err(invalid());
    }
    let status = &journal.upgrades[position];
    delivery(
        slot(plan),
        &stage(plan, expected)?,
        wire::STAGE_UPGRADE,
        &sources.upgrades[position],
        status.phase == upgrade::Phase::Ready,
    )?;
    Ok((status.clone(), Some(sources.upgrades[position].clone())))
}

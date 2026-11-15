//! Root admission and authenticated child evidence. No host-owned child effects.

use crate::{
    ApplyOutcome, DeploymentName, Error, ManagedOwner, Plan, PlanInput, Receipt, Result,
    SelectedFleet,
    network::principal,
    operation, plan,
    signed::{self, Call},
    store, supplier,
};
use canic_management_protocol::{Approval, LIMITS, Phase, Refusal, STAGE_LIMITS, Stage, Status};
use ic_platform::agent::Agent;
use std::path::Path;

pub(crate) mod recovery;
pub(crate) mod stored;
mod wire;
use wire::Method;
pub(crate) use wire::{observation, same_slot};
#[cfg(test)]
mod tests;
pub(crate) mod upgrade;

pub(crate) const OBSERVATIONS: usize = 32;

/// Retain one Root-owned creation or first upgrade for a declared parent/child pair.
/// The parent must be bound or have a completed named deployment in this journal.
/// `execute_at_ns` selects a new plan's schedule (default: ready once approved);
/// reopening preserves the original schedule.
/// `release` selects a completed publication operation in this journal. Its exact
/// bytes, app/fleet and owners must match; absence retains inline delivery.
///
/// # Errors
/// Refuses ambiguous/occupied slots, missing parent custody, invalid budgets or artifacts.
#[expect(
    clippy::too_many_arguments,
    reason = "named planning adds an explicit publication selector without introducing a second planning API"
)]
pub fn create_managed_plan(
    journal: &Path,
    wasm: &Path,
    input: PlanInput,
    selection: &SelectedFleet,
    canister: &str,
    root_module: &str,
    execute_at_ns: Option<u64>,
    release: Option<&str>,
) -> Result<(String, Plan)> {
    selection.validate()?;
    let child = selection
        .config
        .canisters
        .get(canister)
        .ok_or_else(|| Error::Invalid("unknown child slot".into()))?;
    let parent = child
        .parent
        .as_ref()
        .ok_or_else(|| Error::Invalid("Root-owned work requires a declared parent".into()))?;
    let owner = &selection.config.canisters[parent];
    if child.role.as_deref() == Some("wasm-store") {
        return Err(Error::Invalid(
            "Store bootstrap is not qualified through the application-child contract".into(),
        ));
    }
    if input.network != selection.config.network
        || (input.target.is_none() && (child.id.is_some() || input.previous_module.is_some()))
        || owner.role.as_deref() != Some("root")
        || owner.parent.is_some()
        || selection
            .config
            .canisters
            .values()
            .filter(|slot| {
                slot.parent.as_ref() == Some(parent) && slot.role.as_deref() != Some("wasm-store")
            })
            .count()
            != 1
    {
        return Err(Error::Invalid(
            "select one child of a top-level Root on this fleet's network; creation requires an unbound slot".into(),
        ));
    }
    let inventory = crate::fleet::inventory(journal, true, crate::now()?)?;
    if !inventory.issues.is_empty() {
        return Err(Error::Invalid(
            "repair unreadable journal evidence before resolving Root".into(),
        ));
    }
    let mut roots = std::collections::BTreeSet::new();
    let mut creation = None;
    if let Some(id) = &owner.id {
        roots.insert(id.clone());
    }
    for (record, network) in inventory.records {
        if input.target.is_some()
            && record.management.is_some()
            && record.deployment.as_ref().is_some_and(|name| {
                name.app == selection.app
                    && name.fleet == selection.fleet
                    && name.canister == canister
            })
        {
            if network != input.network
                || !record.receipt_retained
                || record.canister != input.target
                || input.previous_module.as_ref() != Some(&record.approved_module)
                || child
                    .id
                    .as_ref()
                    .is_some_and(|id| Some(id) != input.target.as_ref())
            {
                return Err(Error::Invalid(
                    "upgrade needs this child's completed creation receipt and original module"
                        .into(),
                ));
            }
            if creation
                .replace(record.approved_controller.clone())
                .is_some()
            {
                return Err(Error::Invalid("ambiguous child creation evidence".into()));
            }
        }
        if record.deployment.as_ref().is_some_and(|name| {
            name.app == selection.app && name.fleet == selection.fleet && name.canister == *parent
        }) {
            if network != input.network || !record.receipt_retained {
                return Err(Error::Blocked(
                    "parent deployment is incomplete or on another network".into(),
                ));
            }
            roots.insert(
                record
                    .canister
                    .ok_or_else(|| Error::Invalid("parent receipt has no canister".into()))?,
            );
        }
    }
    if roots.len() != 1 {
        return Err(Error::Invalid(
            "parent needs one unambiguous configured or retained deployment identity".into(),
        ));
    }
    if input.target.is_some() && creation.as_ref() != roots.first() {
        return Err(Error::Invalid(
            "child upgrade requires retained creation custody under this Root".into(),
        ));
    }
    let root = roots
        .pop_first()
        .ok_or_else(|| Error::Invalid("missing Root identity".into()))?;
    let source = release
        .map(|id| stored::select(journal, id, &input, selection, canister, &root, root_module))
        .transpose()?;
    let now = crate::now()?;
    let execute_at_ns = execute_at_ns.unwrap_or(now);
    plan::retain(
        journal,
        wasm,
        input,
        Some(DeploymentName {
            app: selection.app.clone(),
            fleet: selection.fleet.clone(),
            canister: canister.into(),
        }),
        Some(ManagedOwner {
            slot: None,
            root,
            source,
            root_module: root_module.into(),
            execute_at_ns,
        }),
        now,
    )
}

pub(crate) fn approval(plan: &Plan) -> Result<Approval> {
    let crate::PlanAction::CreateChild(owner) = &plan.action else {
        return Err(Error::Invalid(
            "expected Root-owned creation approval".into(),
        ));
    };
    Ok(Approval {
        manager: owner.root.clone(),
        module_hash: crate::unhex(&plan.artifact.sha256)?
            .try_into()
            .map_err(|_| Error::Invalid("invalid child module hash".into()))?,
        creation_cycles: plan.creation_cycles(),
        execute_at_ns: owner.execute_at_ns,
        expires_at_ns: plan.expires_at_ns,
    })
}

fn validate(plan: &Plan, previous: Option<&Status>, status: &Status) -> Result<()> {
    if status.approval.as_ref() != Some(&approval(plan)?)
        || status.phase == Phase::Empty
        || status.attempts.iter().any(|count| *count > 1)
        || (status.worker.is_some() && status.attempts[0] != 1)
        || status
            .observed_module
            .as_ref()
            .is_some_and(|hash| crate::hex(hash) != plan.artifact.sha256)
        || (status.phase == Phase::Ready
            && (status.worker.is_none()
                || status.observed_module.is_none()
                || status.attempts != [1; 3]
                || status.failure.is_some()))
    {
        return Err(Error::Invalid(
            "Root evidence does not match the approved child operation".into(),
        ));
    }
    if let Some(child) = &status.worker {
        let child = principal(child)?;
        if child == principal(plan.application_controller())?
            || child == principal("aaaaa-aa")?
            || child == principal("2vxsx-fae")?
        {
            return Err(Error::Invalid("invalid returned child principal".into()));
        }
    }
    if let Some(old) = previous
        && ((old.worker.is_some() && old.worker != status.worker)
            || (old.phase as u8 > status.phase as u8)
            || (matches!(old.phase, Phase::Expired | Phase::ReserveBlocked)
                && old.phase != status.phase)
            || old
                .attempts
                .iter()
                .zip(status.attempts)
                .any(|(old, new)| *old > new)
            || old
                .requests
                .iter()
                .zip(status.requests)
                .any(|(old, new)| old.is_some() && *old != new)
            || (old.creation_refund.is_some() && old.creation_refund != status.creation_refund)
            || (old.observed_module.is_some() && old.observed_module != status.observed_module)
            || (old.phase == Phase::Ready && status.phase != Phase::Ready))
    {
        return Err(Error::Invalid(
            "Root evidence regressed or replaced retained outcome custody".into(),
        ));
    }
    Ok(())
}

fn stage_call(plan: &Plan, wasm: &[u8]) -> Result<Call<'static>> {
    if plan.management().is_some_and(|m| m.source.is_some()) {
        return Method::Stage.call(plan, &(), LIMITS);
    }
    Method::Stage.call(
        plan,
        &Stage {
            approval: approval(plan)?,
            wasm: wasm.to_vec(),
        },
        STAGE_LIMITS,
    )
}

fn stage_reply(plan: &Plan, bytes: &[u8]) -> Result<Status> {
    let reply: std::result::Result<Status, Refusal> = ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: Method::Stage.id(plan),
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .map_err(supplier)?;
    reply.map_err(|refusal| {
        Error::Blocked(format!(
            "Root refused admission: {refusal:?}; original attempt retained"
        ))
    })
}

/// Read only authenticated retained replies; public queries never establish output custody.
pub(crate) fn evidence(dir: &Path, plan: &Plan) -> Result<Option<Status>> {
    evidence_with_source(dir, plan).map(|value| value.map(|(status, _)| status))
}
pub(crate) fn evidence_with_source(
    dir: &Path,
    plan: &Plan,
) -> Result<Option<(Status, Option<canic_management_protocol::stored::Delivery>)>> {
    let mut latest = None;
    let stage = dir.join("stage/reply.bin");
    if stage.try_exists().map_err(supplier)? {
        match stage_reply(plan, &store::read(&stage, LIMITS.bytes)?) {
            Ok(status) => {
                validate(plan, None, &status)?;
                latest = Some((status, None));
            }
            Err(Error::Blocked(_)) => {}
            Err(error) => return Err(error),
        }
    }
    for index in 0..OBSERVATIONS {
        for (name, method) in [
            ("reconcile/reply.bin", Method::Reconcile.id(plan)),
            ("reply.bin", Method::Observe.id(plan)),
        ] {
            let reply = dir.join(format!("observe/{index}/{name}"));
            if reply.try_exists().map_err(supplier)? {
                let bytes = store::read(
                    &reply,
                    canic_management_protocol::custody::ROOT_LIMITS.bytes,
                )?;
                let (status, source) = if matches!(
                    method,
                    canic_management_protocol::stored::CUSTODY
                        | canic_management_protocol::custody::ROOT
                ) {
                    stored::creation_evidence(plan, &bytes)?
                } else {
                    (decode_observation(plan, method, &bytes)?, None)
                };
                if status.phase == Phase::Empty && status.approval.is_none() && latest.is_none() {
                    continue;
                }
                validate(plan, latest.as_ref().map(|(s, _)| s), &status)?;
                latest = Some((status, source));
            }
        }
    }
    Ok(latest)
}

pub(crate) async fn apply(
    dir: &Path,
    id: &str,
    plan: &Plan,
    wasm: &[u8],
    identity: &str,
    icp: &Path,
    agent: &Agent,
) -> Result<ApplyOutcome> {
    let owner = match &plan.action {
        crate::PlanAction::CreateChild(owner) => owner,
        crate::PlanAction::UpgradeChild { .. } => {
            return upgrade::apply(dir, id, plan, wasm, identity, icp, agent).await;
        }
        _ => return Err(Error::Invalid("expected Root-owned child operation".into())),
    };
    let root = principal(&owner.root)?;
    operation::controllers(agent, root, &plan.controller).await?;
    if operation::module(agent, root).await? != owner.root_module {
        return Err(Error::Blocked(
            "Root code differs from the reviewed management owner".into(),
        ));
    }
    let retained = evidence(dir, plan)?;
    if let Some(status) = &retained
        && status.phase == Phase::Ready
    {
        return finish(dir, id, plan, status, agent).await;
    }
    stored::verify(plan, agent).await?;
    let stage = dir.join("stage");
    let status = if !stage.join("signed.json").try_exists().map_err(supplier)? {
        if crate::now()?
            >= plan::admission_deadline(plan.created_at_ns, owner.execute_at_ns, plan.expires_at_ns)
        {
            return Err(Error::Blocked(
                "Root admission time passed; no new stage attempt was signed".into(),
            ));
        }
        stage_reply(
            plan,
            &signed::execute(&stage, plan, &stage_call(plan, wasm)?, identity, icp, agent).await?,
        )?
    } else {
        // A replicated read can recover admission even when the staging response was lost.
        let mut path = None;
        for index in 0..OBSERVATIONS {
            let candidate = dir.join(format!("observe/{index}"));
            if !candidate.join("reply.bin").try_exists().map_err(supplier)? {
                path = Some(candidate);
                break;
            }
        }
        let path = path.ok_or_else(|| {
            Error::Blocked(
                "32 retained observation attempts exhausted; preserve this journal".into(),
            )
        })?;
        let call = observation(plan)?;
        recovery::require_settled(&path)?;
        let bytes = signed::observe(&path, plan, &call, identity, icp, agent).await?;
        let observed = decode_observation(plan, Method::Observe.id(plan), &bytes)?;
        if observed.phase == Phase::Empty && observed.approval.is_none() && retained.is_none() {
            // Empty is not proof the original staging attempt cannot still execute.
            stage_reply(
                plan,
                &signed::execute(&stage, plan, &stage_call(plan, wasm)?, identity, icp, agent)
                    .await?,
            )?
        } else {
            observed
        }
    };
    validate(plan, retained.as_ref(), &status)?;
    if status.phase == Phase::Ready {
        finish(dir, id, plan, &status, agent).await
    } else {
        Ok(ApplyOutcome::Pending {
            operation: id.into(),
            root: owner.root.clone(),
            status: Box::new(status),
        })
    }
}

async fn finish(
    dir: &Path,
    id: &str,
    plan: &Plan,
    status: &Status,
    agent: &Agent,
) -> Result<ApplyOutcome> {
    let child = status
        .worker
        .as_ref()
        .ok_or_else(|| Error::Invalid("ready evidence has no child".into()))?;
    complete(dir, id, plan, child, agent).await
}

async fn complete(
    dir: &Path,
    id: &str,
    plan: &Plan,
    child: &str,
    agent: &Agent,
) -> Result<ApplyOutcome> {
    let receipt = Receipt {
        operation: id.into(),
        canister: child.into(),
        module_hash: plan.artifact.sha256.clone(),
        controller: plan.application_controller().into(),
    };
    operation::verify_receipt(agent, plan, id, &receipt).await?;
    if dir.join("receipt.json").try_exists().map_err(supplier)? {
        if store::json::<Receipt>(&dir.join("receipt.json"))? != receipt {
            return Err(Error::Invalid(
                "receipt substituted retained child evidence".into(),
            ));
        }
    } else {
        store::put_json(&dir.join("receipt.json"), &receipt)?;
    }
    Ok(ApplyOutcome::Complete(receipt))
}

fn decode_observation(plan: &Plan, method: u64, bytes: &[u8]) -> Result<Status> {
    if matches!(
        method,
        canic_management_protocol::stored::CUSTODY | canic_management_protocol::custody::ROOT
    ) {
        stored::creation(plan, bytes)
    } else {
        ic_wire::decode(
            bytes,
            ic_wire::Encoding::Frame {
                method,
                direction: ic_wire::Direction::Response,
            },
            LIMITS,
        )
        .map_err(supplier)
    }
}

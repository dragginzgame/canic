//! Authenticated one-use child upgrade custody through the existing plan and signer.
use super::{OBSERVATIONS, complete, wire::Method};
use crate::{
    ApplyOutcome, Error, Plan, Result,
    network::principal,
    operation,
    signed::{self},
    store, supplier,
};
use canic_management_protocol::{
    LIMITS, Refusal, STAGE_LIMITS,
    upgrade::{Approval, OBSERVE, Phase, STATUS, Stage, Status},
};
use ic_platform::agent::Agent;
use std::path::Path;

pub(crate) fn approval(plan: &Plan) -> Result<Approval> {
    let crate::PlanAction::UpgradeChild { target, owner } = &plan.action else {
        return Err(Error::Invalid("expected Root-owned child upgrade".into()));
    };
    let hash = |value: &str| {
        crate::unhex(value)?
            .try_into()
            .map_err(|_| Error::Invalid("invalid module hash".into()))
    };
    Ok(Approval {
        manager: owner.root.clone(),
        child: target.canister.clone(),
        previous_module: hash(&target.module)?,
        module_hash: hash(&plan.artifact.sha256)?,
        execute_at_ns: owner.execute_at_ns,
        expires_at_ns: plan.expires_at_ns,
    })
}
pub(super) fn validate(plan: &Plan, previous: Option<&Status>, status: &Status) -> Result<()> {
    if status.approval.as_ref() != Some(&approval(plan)?)
        || status.phase == Phase::Empty
        || status.attempts.iter().any(|n| *n > 1)
        || status
            .attempts
            .iter()
            .zip(status.requests)
            .any(|(n, hash)| (*n == 1) != hash.is_some())
        || (status.attempts[0] == 1 && status.previous_version.is_none())
        || (status.attempts[1] == 1 && status.attempts[0] != 1)
        || (status.observed_version.is_some()
            && status.observed_version != status.previous_version.and_then(|v| v.checked_add(1)))
        || (status.phase == Phase::Ready
            && (status.attempts != [1; 2]
                || status.observed_version.is_none()
                || status.failure.is_some()))
    {
        return Err(Error::Invalid(
            "Root upgrade evidence differs from approval or dispatch custody".into(),
        ));
    }
    if let Some(old) = previous
        && (old.phase as u8 > status.phase as u8
            || (matches!(
                old.phase,
                Phase::Expired | Phase::ReserveBlocked | Phase::Ready
            ) && old.phase != status.phase)
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
            || (old.previous_version.is_some() && old.previous_version != status.previous_version)
            || (old.observed_version.is_some() && old.observed_version != status.observed_version))
    {
        return Err(Error::Invalid("Root upgrade evidence regressed".into()));
    }
    Ok(())
}
fn stage_reply(plan: &Plan, bytes: &[u8]) -> Result<Status> {
    let reply: std::result::Result<Status, Refusal> =
        ic_wire::native::decode_response(Method::StageUpgrade.id(plan), bytes, LIMITS)
            .map_err(supplier)?;
    reply.map_err(|refusal| {
        Error::Blocked(format!(
            "Root refused upgrade: {refusal:?}; original attempt retained"
        ))
    })
}
pub(crate) fn evidence(dir: &Path, plan: &Plan) -> Result<Option<Status>> {
    evidence_with_source(dir, plan).map(|value| value.map(|(status, _)| status))
}
pub(crate) fn evidence_with_source(
    dir: &Path,
    plan: &Plan,
) -> Result<Option<(Status, Option<canic_management_protocol::stored::Delivery>)>> {
    let mut latest = None;
    let path = dir.join("stage/reply.bin");
    if path.try_exists().map_err(supplier)? {
        match stage_reply(plan, &store::read(&path, LIMITS.bytes)?) {
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
            ("reconcile/reply.bin", Method::ReconcileUpgrade.id(plan)),
            ("reply.bin", Method::ObserveUpgrade.id(plan)),
        ] {
            let path = dir.join(format!("observe/{index}/{name}"));
            if path.try_exists().map_err(supplier)? {
                let bytes =
                    store::read(&path, canic_management_protocol::custody::ROOT_LIMITS.bytes)?;
                let (status, source) = if matches!(
                    method,
                    canic_management_protocol::stored::CUSTODY
                        | canic_management_protocol::custody::ROOT
                ) {
                    super::stored::upgrade_evidence(plan, &bytes)?
                } else {
                    (
                        ic_wire::native::decode_response(method, &bytes, LIMITS)
                            .or_else(|error| {
                                // Historical on-disk unit observations remain readable; no live fallback.
                                if method == OBSERVE {
                                    ic_wire::native::decode_response(STATUS, &bytes, LIMITS)
                                } else {
                                    Err(error)
                                }
                            })
                            .map_err(supplier)?,
                        None,
                    )
                };
                if status == Status::default() && latest.is_none() {
                    continue;
                }
                validate(plan, latest.as_ref().map(|(s, _)| s), &status)?;
                latest = Some((status, source));
            }
        }
    }
    Ok(latest)
}

pub(super) async fn apply(
    dir: &Path,
    id: &str,
    plan: &Plan,
    wasm: &[u8],
    identity: &str,
    icp: &Path,
    agent: &Agent,
) -> Result<ApplyOutcome> {
    let owner = plan.management().expect("managed upgrade dispatcher");
    let root = principal(&owner.root)?;
    operation::controllers(agent, root, &plan.controller).await?;
    if operation::module(agent, root).await? != owner.root_module {
        return Err(Error::Blocked(
            "Root code differs from reviewed upgrade owner".into(),
        ));
    }
    let retained = evidence(dir, plan)?;
    let child = plan.target().expect("upgrade target");
    if retained
        .as_ref()
        .is_some_and(|status| status.phase == Phase::Ready)
    {
        return complete(dir, id, plan, child, agent).await;
    }
    super::stored::verify(plan, agent).await?;
    let stage = dir.join("stage");
    let call = if owner.source.is_some() {
        Method::StageUpgrade.call(plan, &(), LIMITS)?
    } else {
        Method::StageUpgrade.call(
            plan,
            &Stage {
                approval: approval(plan)?,
                wasm: wasm.to_vec(),
            },
            STAGE_LIMITS,
        )?
    };
    let status = if !stage.join("signed.json").try_exists().map_err(supplier)? {
        if crate::now()?
            >= crate::plan::admission_deadline(
                plan.created_at_ns,
                owner.execute_at_ns,
                plan.expires_at_ns,
            )
        {
            return Err(Error::Blocked(
                "Root upgrade admission time passed; no new attempt signed".into(),
            ));
        }
        operation::controllers(agent, principal(child)?, &owner.root).await?;
        if operation::module(agent, principal(child)?).await?
            != *plan.previous_module().expect("old code")
        {
            return Err(Error::Blocked(
                "upgrade precondition: previous module hash changed".into(),
            ));
        }
        stage_reply(
            plan,
            &signed::execute(&stage, plan, &call, identity, icp, agent).await?,
        )?
    } else {
        let mut path = None;
        for index in 0..OBSERVATIONS {
            let candidate = dir.join(format!("observe/{index}"));
            if !candidate.join("reply.bin").try_exists().map_err(supplier)? {
                path = Some(candidate);
                break;
            }
        }
        let path = path.ok_or_else(|| {
            Error::Blocked("32 upgrade observations exhausted; preserve journal".into())
        })?;
        let read = super::observation(plan)?;
        super::recovery::require_settled(&path)?;
        let bytes = signed::observe(&path, plan, &read, identity, icp, agent).await?;
        let observed: Status = if owner.source.is_some() {
            super::stored::upgraded(plan, &bytes)?
        } else {
            ic_wire::native::decode_response(Method::ObserveUpgrade.id(plan), &bytes, LIMITS)
                .map_err(supplier)?
        };
        if observed == Status::default() && retained.is_none() {
            stage_reply(
                plan,
                &signed::execute(&stage, plan, &call, identity, icp, agent).await?,
            )?
        } else {
            observed
        }
    };
    validate(plan, retained.as_ref(), &status)?;
    if status.phase == Phase::Ready {
        complete(dir, id, plan, child, agent).await
    } else {
        Ok(ApplyOutcome::UpgradePending {
            operation: id.into(),
            root: owner.root.clone(),
            upgrade: Box::new(status),
        })
    }
}

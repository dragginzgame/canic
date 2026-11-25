//! Authenticated one-use child upgrade custody through the existing plan and signer.
use super::{complete, stage_evidence, wire::Method};
use crate::{ApplyOutcome, Error, Plan, Result, network::principal, operation, supplier};
use canic_management_protocol::{
    LIMITS, Refusal,
    upgrade::{Approval, OBSERVE, Phase, STATUS, Status},
};
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
        || (matches!(status.phase, Phase::StopIssued | Phase::Stopped)
            && (status.attempts != [0; 2]
                || status.previous_version.is_some()
                || status.observed_version.is_some()))
        || (status.observed_version.is_some()
            && status.observed_version != status.previous_version.and_then(|v| v.checked_add(1)))
        || (matches!(
            status.phase,
            Phase::Verified | Phase::StartIssued | Phase::Started | Phase::Ready
        ) && (status.attempts != [1; 2] || status.observed_version.is_none()))
        || (status.phase == Phase::Ready && status.failure.is_some())
    {
        return Err(Error::Invalid(
            "Root upgrade evidence differs from approval or dispatch custody".into(),
        ));
    }
    if let Some(old) = previous
        && (progress(old.phase) > progress(status.phase)
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

// Codec discriminants preserve historical bytes; appended stop/start variants
// occur earlier in execution. Observation ordering must not use their encoding.
fn progress(phase: Phase) -> u8 {
    match phase {
        Phase::Empty => 0,
        Phase::Scheduled => 1,
        Phase::StopIssued => 2,
        Phase::Stopped => 3,
        Phase::Checked => 4,
        Phase::UploadIssued => 5,
        Phase::Uploaded => 6,
        Phase::InstallIssued => 7,
        Phase::Installed => 8,
        Phase::Verified => 9,
        Phase::StartIssued => 10,
        Phase::Started => 11,
        Phase::Ready => 12,
        Phase::Expired | Phase::ReserveBlocked => 13,
    }
}

fn stage_reply(plan: &Plan, bytes: &[u8]) -> Result<Status> {
    let reply: std::result::Result<Status, Refusal> = ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: Method::StageUpgrade.id(plan),
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
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
    super::fold_evidence(
        dir,
        [Method::ReconcileUpgrade, Method::ObserveUpgrade],
        stage_evidence(dir, |bytes| stage_reply(plan, bytes))?,
        |old, new| validate(plan, old, new),
        |status| *status == Status::default(),
        |method, bytes| {
            let method = method.id(plan);
            if matches!(
                method,
                canic_management_protocol::application::CHILD_EVIDENCE
                    | canic_management_protocol::application::CUSTODY
            ) {
                super::stored::upgrade_evidence(plan, bytes)
            } else {
                Ok((
                    ic_wire::decode(
                        bytes,
                        ic_wire::Encoding::Frame {
                            method,
                            direction: ic_wire::Direction::Response,
                        },
                        LIMITS,
                    )
                    .or_else(|error| {
                        // Historical on-disk unit observations remain readable; no live fallback.
                        if method == OBSERVE {
                            ic_wire::decode(
                                bytes,
                                ic_wire::Encoding::Frame {
                                    method: STATUS,
                                    direction: ic_wire::Direction::Response,
                                },
                                LIMITS,
                            )
                        } else {
                            Err(error)
                        }
                    })
                    .map_err(supplier)?,
                    None,
                ))
            }
        },
    )
}

pub(super) async fn apply(admission: &super::Admission<'_>, id: &str) -> Result<ApplyOutcome> {
    let super::Admission {
        dir, plan, client, ..
    } = *admission;
    let owner = plan.management().expect("managed upgrade dispatcher");
    let retained = evidence(dir, plan)?;
    let child = plan.target().expect("upgrade target");
    if retained
        .as_ref()
        .is_some_and(|status| status.phase == Phase::Ready)
    {
        return complete(dir, id, plan, child, client).await;
    }
    let status = admission
        .stage(
            retained.is_some(),
            async {
                operation::controllers(client, principal(child)?, &owner.root).await?;
                if operation::module(client, principal(child)?).await?
                    != *plan.previous_module().expect("old code")
                {
                    return Err(Error::Blocked(
                        "upgrade precondition: previous module hash changed".into(),
                    ));
                }
                Ok(())
            },
            || Method::StageUpgrade.call(plan),
            |bytes| stage_reply(plan, bytes),
            |bytes| super::stored::upgraded(plan, bytes),
            |status| *status == Status::default(),
        )
        .await?;
    validate(plan, retained.as_ref(), &status)?;
    if status.phase == Phase::Ready {
        complete(dir, id, plan, child, client).await
    } else {
        Ok(ApplyOutcome::UpgradePending {
            maintenance: super::stored::maintenance_evidence(dir, plan)?.map(Box::new),
            operation: id.into(),
            root: owner.root.clone(),
            upgrade: Box::new(status),
        })
    }
}

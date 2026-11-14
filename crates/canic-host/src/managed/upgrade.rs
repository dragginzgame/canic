//! Authenticated one-use child upgrade custody through the existing plan and signer.
use super::{OBSERVATIONS, complete};
use crate::{
    ApplyOutcome, Error, Plan, Result,
    network::principal,
    operation,
    signed::{self, Call},
    store, supplier,
};
use canic_management_protocol::{
    LIMITS, Refusal, STAGE_LIMITS,
    upgrade::{Approval, Phase, STAGE, STATUS, Stage, Status},
};
use ic_platform::agent::Agent;
use std::path::Path;

pub(crate) fn approval(plan: &Plan) -> Result<Approval> {
    let owner = plan
        .management
        .as_ref()
        .ok_or_else(|| Error::Invalid("upgrade requires Root".into()))?;
    let hash = |value: &str| {
        crate::unhex(value)?
            .try_into()
            .map_err(|_| Error::Invalid("invalid module hash".into()))
    };
    Ok(Approval {
        manager: owner.root.clone(),
        child: plan
            .target
            .clone()
            .ok_or_else(|| Error::Invalid("upgrade requires child".into()))?,
        previous_module: hash(
            plan.previous_module
                .as_deref()
                .ok_or_else(|| Error::Invalid("upgrade requires old code".into()))?,
        )?,
        module_hash: hash(&plan.artifact.sha256)?,
        execute_at_ns: owner.execute_at_ns,
        expires_at_ns: plan.expires_at_ns,
    })
}
fn validate(plan: &Plan, previous: Option<&Status>, status: &Status) -> Result<()> {
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
fn stage_reply(bytes: &[u8]) -> Result<Status> {
    let reply: std::result::Result<Status, Refusal> =
        ic_wire::native::decode_response(STAGE, bytes, LIMITS).map_err(supplier)?;
    reply.map_err(|refusal| {
        Error::Blocked(format!(
            "Root refused upgrade: {refusal:?}; original attempt retained"
        ))
    })
}
pub(crate) fn evidence(dir: &Path, plan: &Plan) -> Result<Option<Status>> {
    let mut latest = None;
    let path = dir.join("stage/reply.bin");
    if path.try_exists().map_err(supplier)? {
        match stage_reply(&store::read(&path, LIMITS.bytes)?) {
            Ok(status) => {
                validate(plan, None, &status)?;
                latest = Some(status);
            }
            Err(Error::Blocked(_)) => {}
            Err(error) => return Err(error),
        }
    }
    for index in 0..OBSERVATIONS {
        let path = dir.join(format!("observe/{index}/reply.bin"));
        if path.try_exists().map_err(supplier)? {
            let status: Status = ic_wire::native::decode_response(
                STATUS,
                &store::read(&path, LIMITS.bytes)?,
                LIMITS,
            )
            .map_err(supplier)?;
            if status == Status::default() && latest.is_none() {
                continue;
            }
            validate(plan, latest.as_ref(), &status)?;
            latest = Some(status);
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
    let owner = plan
        .management
        .as_ref()
        .expect("managed upgrade dispatcher");
    let root = principal(&owner.root)?;
    operation::controllers(agent, root, &plan.controller).await?;
    if operation::module(agent, root).await? != owner.root_module {
        return Err(Error::Blocked(
            "Root code differs from reviewed upgrade owner".into(),
        ));
    }
    let retained = evidence(dir, plan)?;
    let child = plan.target.as_ref().expect("upgrade target");
    if retained
        .as_ref()
        .is_some_and(|status| status.phase == Phase::Ready)
    {
        return complete(dir, id, plan, child, agent).await;
    }
    let stage = dir.join("stage");
    let call = Call {
        canister: root,
        effective: root,
        method: "stage_upgrade",
        args: ic_wire::native::encode_request(
            STAGE,
            &Stage {
                approval: approval(plan)?,
                wasm: wasm.to_vec(),
            },
            STAGE_LIMITS,
        )
        .map_err(supplier)?,
    };
    let status = if !stage.join("signed.json").try_exists().map_err(supplier)? {
        if crate::now()? >= owner.execute_at_ns {
            return Err(Error::Blocked(
                "Root upgrade admission time passed; no new attempt signed".into(),
            ));
        }
        operation::controllers(agent, principal(child)?, &owner.root).await?;
        if operation::module(agent, principal(child)?).await?
            != *plan.previous_module.as_ref().expect("old code")
        {
            return Err(Error::Blocked(
                "upgrade precondition: previous module hash changed".into(),
            ));
        }
        stage_reply(&signed::execute(&stage, plan, &call, identity, icp, agent).await?)?
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
        let read = Call {
            canister: root,
            effective: root,
            method: "observe_upgrade",
            args: ic_wire::native::encode_request(STATUS, &(), LIMITS).map_err(supplier)?,
        };
        let bytes = signed::observe(&path, plan, &read, identity, icp, agent).await?;
        let observed: Status =
            ic_wire::native::decode_response(STATUS, &bytes, LIMITS).map_err(supplier)?;
        if observed == Status::default() && retained.is_none() {
            stage_reply(&signed::execute(&stage, plan, &call, identity, icp, agent).await?)?
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

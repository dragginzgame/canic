//! Explicit recovery of one retained approval; Root continues to own every effect.

use super::{OBSERVATIONS, upgrade};
use crate::{ApplyOutcome, Error, Plan, Result, network::principal, operation, signed, supplier};
use canic_management_protocol::{LIMITS, RECONCILE, Status};
use ic_platform::agent::Agent;
use std::path::{Path, PathBuf};

pub(crate) fn call(plan: &Plan) -> Result<signed::Call<'static>> {
    let owner = plan
        .management
        .as_ref()
        .ok_or_else(|| Error::Invalid("reconciliation requires a Root-owned operation".into()))?;
    let root = principal(&owner.root)?;
    let (method, args) = if plan.target.is_some() {
        (
            "reconcile_upgrade",
            ic_wire::native::encode_request(
                canic_management_protocol::upgrade::RECONCILE,
                &upgrade::approval(plan)?,
                LIMITS,
            )
            .map_err(supplier)?,
        )
    } else {
        (
            "reconcile",
            ic_wire::native::encode_request(RECONCILE, &super::approval(plan)?, LIMITS)
                .map_err(supplier)?,
        )
    };
    Ok(signed::Call {
        canister: root,
        effective: root,
        method,
        args,
    })
}

fn attempt(dir: &Path) -> Result<PathBuf> {
    for index in 0..OBSERVATIONS {
        let path = dir.join(format!("observe/{index}"));
        if !path.join("reply.bin").try_exists().map_err(supplier)? {
            // Never insert recovery ahead of an already signed observation.
            if path.join("signed.json").try_exists().map_err(supplier)? {
                return Err(Error::Blocked("resume the original apply to settle its retained observation before reconciliation".into()));
            }
            return Ok(path.join("reconcile"));
        }
    }
    Err(Error::Blocked(
        "32 retained observation slots exhausted; preserve this journal".into(),
    ))
}

pub(super) fn require_settled(path: &Path) -> Result<()> {
    if path
        .join("reconcile/authority.json")
        .try_exists()
        .map_err(supplier)?
        && !path
            .join("reconcile/reply.bin")
            .try_exists()
            .map_err(supplier)?
    {
        return Err(Error::Blocked("resume fleet reconcile for this operation; its retained recovery request must settle before another observation".into()));
    }
    Ok(())
}

pub(crate) async fn reconcile(
    dir: &Path,
    id: &str,
    plan: &Plan,
    wasm: &[u8],
    identity: &str,
    icp: &Path,
    agent: &Agent,
) -> Result<ApplyOutcome> {
    let call = call(plan)?;
    if !dir
        .join("stage/signed.json")
        .try_exists()
        .map_err(supplier)?
    {
        return Err(Error::Blocked(
            "no retained admission attempt; apply this original operation first".into(),
        ));
    }
    // Only replicated retained evidence establishes admission. Public queries do not.
    let creation = if plan.target.is_none() {
        super::evidence(dir, plan)?
    } else {
        None
    };
    let upgrading = if plan.target.is_some() {
        upgrade::evidence(dir, plan)?
    } else {
        None
    };
    if creation.is_none() && upgrading.is_none() {
        return Err(Error::Blocked("admission remains unknown; apply this original operation to retain a replicated observation first".into()));
    }
    let ready = creation
        .as_ref()
        .is_some_and(|s| s.phase == canic_management_protocol::Phase::Ready)
        || upgrading
            .as_ref()
            .is_some_and(|s| s.phase == canic_management_protocol::upgrade::Phase::Ready);
    if !ready {
        operation::controllers(agent, call.canister, &plan.controller).await?;
        if operation::module(agent, call.canister).await?
            != plan.management.as_ref().expect("bound call").root_module
        {
            return Err(Error::Blocked(
                "Root code differs from the reviewed management owner".into(),
            ));
        }
        let path = attempt(dir)?;
        let bytes = signed::reconcile(&path, plan, identity, icp, agent).await?;
        if let Some(previous) = creation.as_ref() {
            let current: Status =
                ic_wire::native::decode_response(RECONCILE, &bytes, LIMITS).map_err(supplier)?;
            super::validate(plan, Some(previous), &current)?;
        } else {
            let current: canic_management_protocol::upgrade::Status =
                ic_wire::native::decode_response(
                    canic_management_protocol::upgrade::RECONCILE,
                    &bytes,
                    LIMITS,
                )
                .map_err(supplier)?;
            upgrade::validate(plan, upgrading.as_ref(), &current)?;
        }
    }
    // Keep one chronological stream of authenticated progress. Existing apply
    // observes an already signed admission; it cannot create a replacement stage.
    super::apply(dir, id, plan, wasm, identity, icp, agent).await
}

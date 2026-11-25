//! Explicit recovery of one retained approval; Root continues to own every effect.

use super::{OBSERVATIONS, upgrade, wire::Method};
use crate::{ApplyOutcome, Error, Plan, Result, operation, signed, supplier};
use canic_management_protocol::{LIMITS, Status};
use ic_platform::native::ReadClient;
use std::path::{Path, PathBuf};

pub(crate) fn call(plan: &Plan) -> Result<signed::Call<'static>> {
    match plan.action {
        crate::PlanAction::UpgradeChild { .. } => Method::ReconcileUpgrade.call(plan),
        crate::PlanAction::CreateChild(_) => Method::Reconcile.call(plan),
        _ => Err(Error::Invalid("expected Root-owned child recovery".into())),
    }
}

pub(super) fn next_observation(dir: &Path) -> Result<PathBuf> {
    crate::store::next_observation(&dir.join("observe"), OBSERVATIONS, &["reply.bin"])?
        .map(|(_, path)| path)
        .ok_or_else(|| {
            Error::Blocked(format!(
                "{OBSERVATIONS} retained observation slots exhausted; preserve this journal"
            ))
        })
}

fn attempt(dir: &Path) -> Result<PathBuf> {
    let path = next_observation(dir)?;
    // Never insert recovery ahead of an already signed observation.
    if path.join("signed.json").try_exists().map_err(supplier)? {
        return Err(Error::Blocked(
            "resume the original apply to settle its retained observation before reconciliation"
                .into(),
        ));
    }
    Ok(path.join("reconcile"))
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
    identity: &str,
    icp: &Path,
    client: &ReadClient,
) -> Result<ApplyOutcome> {
    if plan
        .management()
        .is_some_and(|owner| owner.source.is_none())
    {
        return Err(Error::Blocked(
            "inline child approvals require their original Root and Host".into(),
        ));
    }
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
    let (creation, upgrading) = match plan.action {
        crate::PlanAction::CreateChild(_) => (super::evidence(dir, plan)?, None),
        crate::PlanAction::UpgradeChild { .. } => (None, upgrade::evidence(dir, plan)?),
        _ => return Err(Error::Invalid("expected Root-owned child recovery".into())),
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
        super::stored::verify(plan, client).await?;
        operation::controllers(client, call.canister, &plan.controller).await?;
        if operation::module(client, call.canister).await?
            != plan.management().expect("bound call").root_module
        {
            return Err(Error::Blocked(
                "Root code differs from the reviewed management owner".into(),
            ));
        }
        let path = attempt(dir)?;
        let bytes = signed::reconcile(&path, plan, identity, icp, client).await?;
        if let Some(previous) = creation.as_ref() {
            let current: Status = ic_wire::decode(
                &bytes,
                ic_wire::Encoding::Frame {
                    method: Method::Reconcile.id(plan),
                    direction: ic_wire::Direction::Response,
                },
                LIMITS,
            )
            .map_err(supplier)?;
            super::validate(plan, Some(previous), &current)?;
        } else {
            let current: canic_management_protocol::upgrade::Status = ic_wire::decode(
                &bytes,
                ic_wire::Encoding::Frame {
                    method: Method::ReconcileUpgrade.id(plan),
                    direction: ic_wire::Direction::Response,
                },
                LIMITS,
            )
            .map_err(supplier)?;
            upgrade::validate(plan, upgrading.as_ref(), &current)?;
        }
    }
    // Keep one chronological stream of authenticated progress. Existing apply
    // observes an already signed admission; it cannot create a replacement stage.
    super::apply(dir, id, plan, identity, icp, client).await
}

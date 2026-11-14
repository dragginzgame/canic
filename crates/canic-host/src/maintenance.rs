//! Root maintenance checks around the existing direct installation owner.
use crate::{Error, Plan, Result, network::principal, operation, signed, store, supplier};
use canic_management_protocol::{
    Phase,
    custody::{self, Custody},
    upgrade,
};
use ic_platform::agent::Agent;
use std::path::Path;

pub(crate) fn validate(plan: &Plan) -> Result<()> {
    let expected = plan.maintenance.as_ref().ok_or_else(invalid)?;
    let creation = expected.creation.approval.as_ref().ok_or_else(invalid)?;
    if plan.version != 2
        || plan.management.is_some()
        || plan.creation_cycles != 0
        || plan.target.as_ref() != Some(&creation.manager)
        || plan.previous_module.as_ref() == Some(&plan.artifact.sha256)
        || expected.creation.phase != Phase::Ready
        || expected.creation.failure.is_some()
        || expected.creation.worker.is_none()
        || expected.creation.stable_pages != 0
        || expected.creation.maintenance_candidate
        || expected.upgrades.len() > upgrade::MAX_UPGRADES
        || expected.upgrades.iter().any(|s| {
            s.phase != upgrade::Phase::Ready
                || s.failure.is_some()
                || s.approval.as_ref().is_none_or(|a| {
                    a.manager != creation.manager
                        || Some(&a.child) != expected.creation.worker.as_ref()
                })
        })
    {
        return Err(invalid());
    }
    ic_wire::native::encode(expected, custody::LIMITS).map_err(supplier)?;
    Ok(())
}

fn invalid() -> Error {
    Error::Invalid(
        "Root maintenance requires a bounded completed child journal and exact Root target".into(),
    )
}

pub(crate) fn retained(dir: &Path, plan: &Plan) -> Result<bool> {
    let Some(expected) = &plan.maintenance else {
        return Ok(true);
    };
    Ok(retained_at(dir, expected, "custody-before")?
        && retained_at(dir, expected, "custody-after")?)
}

fn retained_at(dir: &Path, expected: &Custody, step: &str) -> Result<bool> {
    let path = dir.join(step).join("reply.bin");
    if !path.try_exists().map_err(supplier)? {
        return Ok(false);
    }
    let observed: Custody = ic_wire::native::decode_response(
        custody::OBSERVE,
        &store::read(&path, custody::LIMITS.bytes)?,
        custody::LIMITS,
    )
    .map_err(supplier)?;
    if &observed != expected {
        return Err(invalid());
    }
    Ok(true)
}

pub(crate) fn require_before(dir: &Path, plan: &Plan) -> Result<()> {
    if let Some(expected) = &plan.maintenance
        && !retained_at(dir, expected, "custody-before")?
    {
        return Err(Error::Blocked(
            "maintenance install lacks its retained preflight custody; preserve this operation"
                .into(),
        ));
    }
    Ok(())
}

pub(crate) async fn observe(
    dir: &Path,
    plan: &Plan,
    after: bool,
    identity: &str,
    icp: &Path,
    agent: &Agent,
) -> Result<()> {
    let Some(expected) = &plan.maintenance else {
        return Ok(());
    };
    let target = principal(plan.target.as_deref().ok_or_else(invalid)?)?;
    operation::controllers(agent, target, &plan.controller).await?;
    let hash = if after {
        &plan.artifact.sha256
    } else {
        plan.previous_module.as_ref().ok_or_else(invalid)?
    };
    if operation::module(agent, target).await? != *hash {
        return Err(Error::Blocked(
            "Root maintenance code binding changed".into(),
        ));
    }
    let call = signed::Call {
        canister: target,
        effective: target,
        method: "observe_custody",
        args: ic_wire::native::encode_request(custody::OBSERVE, &(), custody::LIMITS)
            .map_err(supplier)?,
    };
    let path = dir.join(if after {
        "custody-after"
    } else {
        "custody-before"
    });
    let bytes = signed::observe(&path, plan, &call, identity, icp, agent).await?;
    let observed: Custody =
        ic_wire::native::decode_response(custody::OBSERVE, &bytes, custody::LIMITS)
            .map_err(supplier)?;
    if &observed != expected {
        return Err(Error::Blocked(
            "Root journal differs from approved maintenance custody; retain this operation".into(),
        ));
    }
    // The child's live identity, code and controller also remain fixed by this window.
    let child = principal(expected.creation.worker.as_deref().ok_or_else(invalid)?)?;
    operation::controllers(agent, child, &target.to_text()).await?;
    let module = expected
        .upgrades
        .last()
        .and_then(|s| s.approval.as_ref())
        .map(|a| a.module_hash)
        .unwrap_or(
            expected
                .creation
                .approval
                .as_ref()
                .ok_or_else(invalid)?
                .module_hash,
        );
    if operation::module(agent, child).await? != crate::hex(&module) {
        return Err(Error::Blocked(
            "child code changed during Root maintenance".into(),
        ));
    }
    Ok(())
}

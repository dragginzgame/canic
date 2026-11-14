//! Root maintenance checks around the existing direct installation owner.
use crate::{Error, Plan, Result, network::principal, operation, signed, store, supplier};
use canic_management_protocol::{
    LIMITS, Phase, STATUS, Status,
    custody::{self, Custody},
    upgrade,
};
use ic_platform::agent::Agent;
use std::path::Path;

#[cfg(test)]
mod tests;

// The retained pre-0.204 Root has exactly one non-resettable upgrade slot and
// controller-only replicated unit observations. See the pinned qualification fixture.
const SINGLE_UPGRADE_ROOT: &str =
    "e4a4127abd525f59378503dcdb98fcb13f8eb9895673d4d8a4dea41d85e24296";

fn single_upgrade_root(plan: &Plan) -> bool {
    plan.previous_module.as_deref() == Some(SINGLE_UPGRADE_ROOT)
}

fn observations(plan: &Plan, after: bool) -> &'static [(&'static str, &'static str)] {
    if after {
        &[("custody-after", "observe_custody")]
    } else if single_upgrade_root(plan) {
        &[
            ("custody-before-creation", "observe"),
            ("custody-before-upgrade", "observe_upgrade"),
        ]
    } else {
        &[("custody-before", "observe_custody")]
    }
}

/// Exact read authority derived from the approved predecessor, never endpoint probing.
pub(crate) fn observation_args(plan: &Plan, method: &str) -> Result<Vec<u8>> {
    validate(plan)?;
    let (id, limits) = match method {
        "observe_custody" => (custody::OBSERVE, custody::LIMITS),
        "observe" if single_upgrade_root(plan) => (STATUS, LIMITS),
        "observe_upgrade" if single_upgrade_root(plan) => (upgrade::STATUS, LIMITS),
        _ => return Err(invalid()),
    };
    ic_wire::native::encode_request(id, &(), limits).map_err(supplier)
}

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
        || (single_upgrade_root(plan) && expected.upgrades.len() > 1)
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
    if plan.maintenance.is_none() {
        return Ok(true);
    }
    Ok(retained_at(dir, plan, false)? && retained_at(dir, plan, true)?)
}

fn retained_at(dir: &Path, plan: &Plan, after: bool) -> Result<bool> {
    let expected = plan.maintenance.as_ref().ok_or_else(invalid)?;
    for (step, method) in observations(plan, after) {
        let path = dir.join(step).join("reply.bin");
        if !path.try_exists().map_err(supplier)? {
            return Ok(false);
        }
        if !matches(
            expected,
            method,
            &store::read(&path, custody::LIMITS.bytes)?,
        )? {
            return Err(invalid());
        }
    }
    Ok(true)
}

fn matches(expected: &Custody, method: &str, bytes: &[u8]) -> Result<bool> {
    match method {
        "observe" => {
            let mut observed: Status =
                ic_wire::native::decode_response(STATUS, bytes, LIMITS).map_err(supplier)?;
            observed.stable_pages = 0;
            observed.maintenance_candidate = false;
            Ok(observed == expected.creation)
        }
        "observe_upgrade" => {
            let observed: upgrade::Status =
                ic_wire::native::decode_response(upgrade::STATUS, bytes, LIMITS)
                    .map_err(supplier)?;
            Ok(observed == expected.upgrades.first().cloned().unwrap_or_default())
        }
        "observe_custody" => {
            let observed: Custody =
                ic_wire::native::decode_response(custody::OBSERVE, bytes, custody::LIMITS)
                    .map_err(supplier)?;
            Ok(&observed == expected)
        }
        _ => Err(invalid()),
    }
}

pub(crate) fn require_before(dir: &Path, plan: &Plan) -> Result<()> {
    if plan.maintenance.is_some() && !retained_at(dir, plan, false)? {
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
    for (step, method) in observations(plan, after) {
        let call = signed::Call {
            canister: target,
            effective: target,
            method,
            args: observation_args(plan, method)?,
        };
        let bytes = signed::observe(&dir.join(step), plan, &call, identity, icp, agent).await?;
        if !matches(expected, method, &bytes)? {
            return Err(Error::Blocked(
                "Root journal differs from approved maintenance custody; retain this operation"
                    .into(),
            ));
        }
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

//! Root maintenance checks around the existing direct installation owner.

#[cfg(test)]
pub(crate) mod tests;

mod stored;

use crate::{Error, Plan, Result, network::principal, operation, signed, store, supplier};
use canic_management_protocol::{Phase, application, custody, upgrade};
use ic_platform::native::ReadClient;
use std::path::Path;

/// Stop/drain or restart the approved upgrade target through the existing signer.
/// Successful stop plus retained management status precedes installation. Lost
/// responses retain the original signed attempt; expiry never creates a new one.
pub(crate) async fn runtime(
    dir: &Path,
    plan: &Plan,
    start: bool,
    identity: &str,
    icp: &Path,
    client: &ReadClient,
) -> Result<()> {
    use ic_platform_protocol::{candid::Limits, management as mgmt, management_snapshots::Status};
    let target = principal(plan.target().map(String::as_str).ok_or_else(invalid)?)?;
    let args = mgmt::CanisterArgs {
        canister_id: ic_platform_types::Principal::from_slice(target.as_slice()),
    }
    .encode(1024)
    .map_err(Error::Encode)?;
    let (step, method) = if start {
        ("start", "start_canister")
    } else {
        ("stop", "stop_canister")
    };
    let call = signed::Call {
        canister: ic_platform_types::Principal::management_canister(),
        effective: target,
        method,
        args,
    };
    let reply = signed::execute(&dir.join(step), plan, &call, identity, icp, client).await?;
    let limits = Limits {
        bytes: 65_536,
        header: 4096,
        types: 256,
        work: 100_000,
        allocation: 262_144,
        vector: 65_536,
        skipping: 100_000,
    };
    mgmt::decode_empty_response(&reply, limits).map_err(Error::Decode)?;
    if !start {
        let status_call = signed::Call {
            method: "canister_status",
            ..call
        };
        let reply = signed::execute(
            &dir.join("stopped"),
            plan,
            &status_call,
            identity,
            icp,
            client,
        )
        .await?;
        let status = mgmt::decode_canister_status(&reply, limits).map_err(Error::Decode)?;
        if status.status != Status::Stopped
            || status.settings.controllers != [plan.controller.parse().map_err(supplier)?]
        {
            return Err(Error::Blocked(
                "upgrade target is not stopped under the approved controller".into(),
            ));
        }
    }
    Ok(())
}

// The retained pre-0.204 Root has exactly one non-resettable upgrade slot and
// controller-only replicated unit observations. See the pinned qualification fixture.
pub(crate) const SINGLE_UPGRADE_ROOT: &str =
    "e4a4127abd525f59378503dcdb98fcb13f8eb9895673d4d8a4dea41d85e24296";

/// Frozen one-child Roots that predate the explicit slot protocol.
pub(crate) fn implicit_child_root(module: &str) -> bool {
    matches!(
        module,
        SINGLE_UPGRADE_ROOT | "3de14eb8422bb99c57f9c63870523460fb9d7b3d4c5f0ef5f7a75cc0c1333c3a"
    )
}

pub(crate) const BEFORE: &str = "custody-before";
pub(crate) const AFTER: &str = "custody-after";
const OBSERVE: &str = "observe_root_custody";

pub(crate) fn observation_args(plan: &Plan, method: &str) -> Result<Vec<u8>> {
    validate(plan)?;
    if method != "observe_root_custody" {
        return Err(invalid());
    }
    ic_wire::encode(
        &(),
        ic_wire::Encoding::Frame {
            method: application::CUSTODY,
            direction: ic_wire::Direction::Request,
        },
        application::LIMITS,
    )
    .map_err(supplier)
}

pub(crate) fn validate(plan: &Plan) -> Result<()> {
    let crate::PlanAction::MaintainRoot {
        target,
        children: expected,
        ..
    } = &plan.action
    else {
        return Err(invalid());
    };
    if expected.is_empty()
        || plan
            .previous_module()
            .is_some_and(|module| implicit_child_root(module))
        || expected
            .iter()
            .map(|c| &c.creation.worker)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != expected.len()
    {
        return Err(invalid());
    }
    for expected in expected {
        let creation = expected.creation.approval.as_ref().ok_or_else(invalid)?;
        if target.canister != creation.manager
            || target.module == plan.artifact.sha256
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
        ic_wire::encode(expected, ic_wire::Encoding::Payload, custody::LIMITS).map_err(supplier)?;
    }
    ic_wire::encode(
        expected,
        ic_wire::Encoding::Payload,
        canic_management_protocol::children::LIMITS,
    )
    .map_err(supplier)?;
    stored::validate(plan)?;
    Ok(())
}

fn invalid() -> Error {
    Error::Invalid(
        "Root maintenance requires a bounded completed child journal and exact Root target".into(),
    )
}

pub(crate) fn retained(dir: &Path, plan: &Plan) -> Result<bool> {
    if plan.maintenance().is_none() {
        return Ok(true);
    }
    Ok(retained_at(dir, plan, false)? && retained_at(dir, plan, true)?)
}

fn retained_at(dir: &Path, plan: &Plan, after: bool) -> Result<bool> {
    let step = if after { AFTER } else { BEFORE };
    let path = dir.join(step).join("reply.bin");
    if !path.try_exists().map_err(supplier)? {
        return Ok(false);
    }
    if !matches_plan(
        plan,
        OBSERVE,
        &store::read(&path, application::LIMITS.bytes)?,
    )? {
        return Err(invalid());
    }
    Ok(true)
}

fn matches_plan(plan: &Plan, method: &str, bytes: &[u8]) -> Result<bool> {
    if method != "observe_root_custody" {
        return Err(invalid());
    }
    let actual = crate::root_custody::decode(application::CUSTODY, bytes)?;
    Ok(actual == stored::expected(plan)?)
}

pub(crate) fn require_before(dir: &Path, plan: &Plan) -> Result<()> {
    if plan.maintenance().is_some() && !retained_at(dir, plan, false)? {
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
    client: &ReadClient,
) -> Result<()> {
    let Some(expected) = plan.maintenance() else {
        return Ok(());
    };
    let target = principal(plan.target().map(String::as_str).ok_or_else(invalid)?)?;
    operation::controllers(client, target, &plan.controller).await?;
    let hash = if after {
        &plan.artifact.sha256
    } else {
        plan.previous_module().ok_or_else(invalid)?
    };
    if operation::module(client, target).await? != *hash {
        return Err(Error::Blocked(
            "Root maintenance code binding changed".into(),
        ));
    }
    let store = plan.maintenance_store().ok_or_else(invalid)?;
    crate::managed::stored::verify_store(client, &target.to_text(), &store.store).await?;
    let step = if after { AFTER } else { BEFORE };
    let call = signed::Call {
        canister: target,
        effective: target,
        method: OBSERVE,
        args: observation_args(plan, OBSERVE)?,
    };
    let bytes = signed::observe(&dir.join(step), plan, &call, identity, icp, client).await?;
    if !matches_plan(plan, OBSERVE, &bytes)? {
        return Err(Error::Blocked(
            "Root journal differs from approved maintenance custody; retain this operation".into(),
        ));
    }
    for expected in expected {
        // The child's live identity, code and controller also remain fixed by this window.
        let child = principal(expected.creation.worker.as_deref().ok_or_else(invalid)?)?;
        operation::controllers(client, child, &target.to_text()).await?;
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
        if operation::module(client, child).await? != crate::hex(&module) {
            return Err(Error::Blocked(
                "child code changed during Root maintenance".into(),
            ));
        }
    }
    Ok(())
}

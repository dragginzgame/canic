//! Root maintenance checks around the existing direct installation owner.
use crate::{Error, Plan, Result, network::principal, operation, signed, store, supplier};
use canic_management_protocol::{
    LIMITS, Phase, STATUS, Status,
    children::ChildId,
    custody::{self, Custody},
    upgrade,
};
use ic_platform::agent::Agent;
use std::path::Path;

#[cfg(test)]
mod installed_tests;
mod stored;
#[cfg(test)]
mod tests;

// The retained pre-0.204 Root has exactly one non-resettable upgrade slot and
// controller-only replicated unit observations. See the pinned qualification fixture.
const SINGLE_UPGRADE_ROOT: &str =
    "e4a4127abd525f59378503dcdb98fcb13f8eb9895673d4d8a4dea41d85e24296";

fn single_upgrade_root(plan: &Plan) -> bool {
    plan.previous_module().map(String::as_str) == Some(SINGLE_UPGRADE_ROOT)
}

// Execution and status share the exact retained-read locations for this predecessor.
pub(crate) fn observations(plan: &Plan, after: bool) -> &'static [(&'static str, &'static str)] {
    if plan
        .maintenance_store()
        .is_some_and(|s| !s.history.is_empty())
    {
        return if after {
            &[("custody-after", "observe_root_history")]
        } else {
            &[("custody-before", "observe_root_history")]
        };
    }
    if plan
        .maintenance_store()
        .is_some_and(|s| s.operation.is_some())
    {
        return if after {
            &[("custody-after", "observe_root_custody")]
        } else {
            &[("custody-before", "observe_root_custody")]
        };
    }
    if plan.maintenance_store().is_some() {
        return if after {
            &[("custody-after", "observe_stored_children")]
        } else {
            &[("custody-before", "observe_stored_children")]
        };
    }
    if plan
        .maintenance()
        .is_some_and(|children| children.len() > 1)
    {
        return if after {
            &[("custody-after", "observe_children")]
        } else {
            &[("custody-before", "observe_children")]
        };
    }
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
    if plan.maintenance_store().is_some() {
        if method != observations(plan, false)[0].1 {
            return Err(invalid());
        }
        if matches!(method, "observe_root_custody" | "observe_root_history") {
            return ic_wire::encode(
                &(),
                ic_wire::Encoding::Frame {
                    method: if method == "observe_root_history" {
                        custody::ROOT_HISTORY
                    } else {
                        custody::ROOT
                    },
                    direction: ic_wire::Direction::Request,
                },
                custody::ROOT_HISTORY_LIMITS,
            )
            .map_err(supplier);
        }
        return ic_wire::encode(
            &(),
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::stored::CUSTODY,
                direction: ic_wire::Direction::Request,
            },
            canic_management_protocol::stored::LIMITS,
        )
        .map_err(supplier);
    }
    let (id, limits) = match method {
        "observe_children"
            if plan
                .maintenance()
                .is_some_and(|children| children.len() > 1) =>
        {
            (canic_management_protocol::children::CUSTODY, LIMITS)
        }
        "observe_custody"
            if plan
                .maintenance()
                .is_some_and(|children| children.len() == 1) =>
        {
            (custody::OBSERVE, custody::LIMITS)
        }
        "observe" if single_upgrade_root(plan) => (STATUS, LIMITS),
        "observe_upgrade" if single_upgrade_root(plan) => (upgrade::STATUS, LIMITS),
        _ => return Err(invalid()),
    };
    ic_wire::encode(
        &(),
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Request,
        },
        limits,
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
        || expected.len() > ChildId::ALL.len()
        || (single_upgrade_root(plan) && expected.len() != 1)
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
        if plan.version != 2
            || target.canister != creation.manager
            || target.module == plan.artifact.sha256
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
    for (step, method) in observations(plan, after) {
        let path = dir.join(step).join("reply.bin");
        if !path.try_exists().map_err(supplier)? {
            return Ok(false);
        }
        if !matches_plan(
            plan,
            method,
            &store::read(
                &path,
                if plan
                    .maintenance_store()
                    .is_some_and(|s| s.operation.is_some())
                {
                    custody::ROOT_HISTORY_LIMITS.bytes
                } else if plan.maintenance_store().is_some() {
                    canic_management_protocol::stored::LIMITS.bytes
                } else {
                    canic_management_protocol::children::LIMITS.bytes
                },
            )?,
        )? {
            return Err(invalid());
        }
    }
    Ok(true)
}

fn matches_plan(plan: &Plan, method: &str, bytes: &[u8]) -> Result<bool> {
    let expected = plan.maintenance().ok_or_else(invalid)?;
    if plan.maintenance_store().is_some() {
        return stored::matches(plan, method, bytes);
    }
    if method == "observe_children" {
        let observed: canic_management_protocol::children::Custody = ic_wire::decode(
            bytes,
            ic_wire::Encoding::Frame {
                method: canic_management_protocol::children::CUSTODY,
                direction: ic_wire::Direction::Response,
            },
            canic_management_protocol::children::LIMITS,
        )
        .map_err(supplier)?;
        Ok(observed.as_slice() == expected)
    } else {
        let [child] = expected else {
            return Err(invalid());
        };
        matches(child, method, bytes)
    }
}

fn matches(expected: &Custody, method: &str, bytes: &[u8]) -> Result<bool> {
    match method {
        "observe" => {
            let mut observed: Status = ic_wire::decode(
                bytes,
                ic_wire::Encoding::Frame {
                    method: STATUS,
                    direction: ic_wire::Direction::Response,
                },
                LIMITS,
            )
            .map_err(supplier)?;
            observed.stable_pages = 0;
            observed.maintenance_candidate = false;
            Ok(observed == expected.creation)
        }
        "observe_upgrade" => {
            let observed: upgrade::Status = ic_wire::decode(
                bytes,
                ic_wire::Encoding::Frame {
                    method: upgrade::STATUS,
                    direction: ic_wire::Direction::Response,
                },
                LIMITS,
            )
            .map_err(supplier)?;
            Ok(observed == expected.upgrades.first().cloned().unwrap_or_default())
        }
        "observe_custody" => {
            let observed: Custody = ic_wire::decode(
                bytes,
                ic_wire::Encoding::Frame {
                    method: custody::OBSERVE,
                    direction: ic_wire::Direction::Response,
                },
                custody::LIMITS,
            )
            .map_err(supplier)?;
            Ok(&observed == expected)
        }
        _ => Err(invalid()),
    }
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
    agent: &Agent,
) -> Result<()> {
    let Some(expected) = plan.maintenance() else {
        return Ok(());
    };
    let target = principal(plan.target().map(String::as_str).ok_or_else(invalid)?)?;
    operation::controllers(agent, target, &plan.controller).await?;
    let hash = if after {
        &plan.artifact.sha256
    } else {
        plan.previous_module().ok_or_else(invalid)?
    };
    if operation::module(agent, target).await? != *hash {
        return Err(Error::Blocked(
            "Root maintenance code binding changed".into(),
        ));
    }
    if let Some(store) = plan.maintenance_store() {
        crate::managed::stored::verify_store(agent, &target.to_text(), &store.store).await?;
    }
    for (step, method) in observations(plan, after) {
        let call = signed::Call {
            canister: target,
            effective: target,
            method,
            args: observation_args(plan, method)?,
        };
        let bytes = signed::observe(&dir.join(step), plan, &call, identity, icp, agent).await?;
        if !matches_plan(plan, method, &bytes)? {
            return Err(Error::Blocked(
                "Root journal differs from approved maintenance custody; retain this operation"
                    .into(),
            ));
        }
    }
    for expected in expected {
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
    }
    Ok(())
}

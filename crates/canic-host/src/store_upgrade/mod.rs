//! Host drives explicit Store endpoints; Root alone owns management effects and recovery.

#[cfg(test)]
mod tests;

mod continuation;

use crate::{
    ApplyOutcome, Error, Plan, Receipt, Result, network::principal, operation, signed, store,
    supplier,
};
use canic_management_protocol::store_maintenance::{self as wire, Phase, Status};
pub use continuation::{StoreContinuation, store_continuation};
use ic_platform::native::ReadClient;
use std::path::{Path, PathBuf};

const OBSERVATIONS: usize = 4096;
const STEPS_PER_APPLY: usize = 32;

pub(crate) fn validate(plan: &Plan) -> Result<()> {
    let root = plan.store_upgrade_root().ok_or_else(invalid)?;
    let target = plan.target().ok_or_else(invalid)?;
    let root_id = principal(&root.canister)?;
    let target_id = principal(target)?;
    if plan.version != 2
        || !(1..=wire::MAX_UPGRADES as u64).contains(&plan.store_upgrade_sequence().unwrap_or(1))
        || plan.controller == "2vxsx-fae"
        || root_id.to_text() != root.canister
        || target_id.to_text() != *target
        || root.canister == *target
        || ["aaaaa-aa", "2vxsx-fae"].contains(&target.as_str())
        || ["aaaaa-aa", "2vxsx-fae"].contains(&root.canister.as_str())
        || crate::unhex(&root.module)?.len() != 32
        || root.module != root.module.to_ascii_lowercase()
        || plan.previous_module() == Some(&plan.artifact.sha256)
        || plan
            .previous_module()
            .is_none_or(|m| m != &m.to_ascii_lowercase())
    {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> Error {
    Error::Invalid(
        "Store upgrade requires exact Root, Store, code and retained operation evidence".into(),
    )
}

fn hash(value: &str) -> Result<[u8; 32]> {
    crate::unhex(value)?.try_into().map_err(|_| invalid())
}

pub(crate) fn approval(plan: &Plan) -> Result<wire::Approval> {
    validate(plan)?;
    Ok(wire::Approval {
        store: plan.target().ok_or_else(invalid)?.clone(),
        previous_module: hash(plan.previous_module().ok_or_else(invalid)?)?,
        module_hash: hash(&plan.artifact.sha256)?,
        attempt: ic_wasm_store_protocol::MaintenanceAttempt {
            sequence: plan.store_upgrade_sequence().unwrap_or(1),
            operation: hash(&plan.digest()?)?,
        },
        expires_at_ns: plan.expires_at_ns,
    })
}

fn call<T: serde::Serialize>(
    plan: &Plan,
    method: &'static str,
    id: u64,
    value: &T,
    limits: ic_wire::Limits,
) -> Result<signed::Call<'static>> {
    let root = principal(&plan.store_upgrade_root().ok_or_else(invalid)?.canister)?;
    Ok(signed::Call {
        canister: root,
        effective: root,
        method,
        args: ic_wire::encode(
            value,
            ic_wire::Encoding::Frame {
                method: id,
                direction: ic_wire::Direction::Request,
            },
            limits,
        )
        .map_err(supplier)?,
    })
}

pub(crate) fn observation(plan: &Plan) -> Result<signed::Call<'static>> {
    validate(plan)?;
    if plan.store_upgrade_sequence().is_some() {
        return call(
            plan,
            "observe_store_attempt",
            wire::OBSERVE_ATTEMPT,
            &approval(plan)?,
            wire::LIMITS,
        );
    }
    call(
        plan,
        "observe_store_maintenance",
        wire::OBSERVE,
        &(),
        wire::LIMITS,
    )
}

pub(crate) fn recovery(plan: &Plan) -> Result<signed::Call<'static>> {
    call(
        plan,
        "recover_store_maintenance",
        wire::RECOVER,
        &approval(plan)?,
        wire::LIMITS,
    )
}

fn admit(plan: &Plan, value: &Status) -> Result<()> {
    if value.approval != approval(plan)? || value.operator != plan.controller {
        return Err(invalid());
    }
    Ok(())
}

fn decode(plan: &Plan, bytes: &[u8]) -> Result<Option<Status>> {
    let value: Option<Status> = ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: if plan.store_upgrade_sequence().is_some() {
                wire::OBSERVE_ATTEMPT
            } else {
                wire::OBSERVE
            },
            direction: ic_wire::Direction::Response,
        },
        wire::LIMITS,
    )
    .map_err(supplier)?;
    if let Some(value) = &value {
        admit(plan, value)?;
    }
    Ok(value)
}

fn path(dir: &Path, index: usize) -> PathBuf {
    dir.join("store-upgrade").join(index.to_string())
}

fn next(dir: &Path) -> Result<usize> {
    for index in 0..OBSERVATIONS {
        if !path(dir, index)
            .join("reply.bin")
            .try_exists()
            .map_err(supplier)?
        {
            return Ok(index);
        }
    }
    Ok(OBSERVATIONS)
}

async fn observe(
    dir: &Path,
    plan: &Plan,
    identity: &str,
    icp: &Path,
    client: &ReadClient,
) -> Result<(usize, Option<Status>)> {
    let index = next(dir)?;
    if index == OBSERVATIONS {
        return Err(Error::Blocked(
            "Store observation budget exhausted; retain the original operation".into(),
        ));
    }
    let reply = signed::observe_operation(&path(dir, index), plan, identity, icp, client).await?;
    Ok((index, decode(plan, &reply)?))
}

fn retained(dir: &Path, plan: &Plan, index: usize) -> Result<Option<Status>> {
    if index >= OBSERVATIONS {
        return Err(invalid());
    }
    decode(plan, &signed::retained_operation(&path(dir, index), plan)?)
}

pub(crate) fn ready(value: &Status) -> bool {
    value.phase == Phase::Ready
        && value.failure.is_none()
        && value.preservation.as_ref().is_some_and(|p| p.complete)
        && value.installation.as_ref().is_some_and(|i| {
            i.request.is_some()
                && i.start_request.is_some()
                && i.activation_request.is_some()
                && i.verification == value.preservation
        })
}

pub(crate) fn receipt(dir: &Path, plan: &Plan) -> Result<Option<(Receipt, Status)>> {
    let completion = dir.join("store-completion.json");
    if !completion.try_exists().map_err(supplier)? {
        return Ok(None);
    }
    let index: usize = store::json(&completion)?;
    let status = retained(dir, plan, index)?.ok_or_else(invalid)?;
    if !ready(&status) {
        return Err(invalid());
    }
    signed::validate_retained(
        &dir.join("stage"),
        plan,
        &call(
            plan,
            "stage_store_maintenance",
            wire::STAGE,
            &approval(plan)?,
            wire::LIMITS,
        )?,
    )?;
    Ok(Some((
        Receipt {
            operation: plan.digest()?,
            canister: status.approval.store.clone(),
            module_hash: plan.artifact.sha256.clone(),
            controller: plan.application_controller().into(),
        },
        status,
    )))
}

async fn verify(plan: &Plan, client: &ReadClient) -> Result<()> {
    let root = plan.store_upgrade_root().ok_or_else(invalid)?;
    let root_id = principal(&root.canister)?;
    operation::controllers(client, root_id, &plan.controller).await?;
    if operation::module(client, root_id).await? != root.module {
        return Err(Error::Blocked(
            "Store upgrade Root code changed; retain the original approval".into(),
        ));
    }
    let target = plan.target().ok_or_else(invalid)?;
    operation::controllers(client, principal(target)?, &root.canister).await?;
    if crate::network::subnet(client, target).await?
        != crate::network::subnet(client, &root.canister).await?
    {
        return Err(invalid());
    }
    let module = operation::module(client, principal(target)?).await?;
    if &module != plan.previous_module().ok_or_else(invalid)? && module != plan.artifact.sha256 {
        return Err(Error::Blocked(
            "Store code differs from both approved executables".into(),
        ));
    }
    Ok(())
}

pub(crate) async fn apply(
    dir: &Path,
    plan: &Plan,
    wasm: &[u8],
    identity: &str,
    icp: &Path,
    read_only: bool,
) -> Result<ApplyOutcome> {
    let client = plan.network.client()?;
    verify(plan, &client).await?;
    if let Some((receipt, _)) = receipt(dir, plan)? {
        operation::verify_receipt(&client, plan, &plan.digest()?, &receipt).await?;
        return Ok(ApplyOutcome::Complete(receipt));
    }
    let approval = approval(plan)?;
    let stage = call(
        plan,
        "stage_store_maintenance",
        wire::STAGE,
        &approval,
        wire::LIMITS,
    )?;
    if read_only
        && !dir
            .join("stage/signed.json")
            .try_exists()
            .map_err(supplier)?
    {
        return Err(Error::Blocked("Store reconcile requires its retained admission attempt; apply the approved operation first".into()));
    }
    let (mut index, mut current) = observe(dir, plan, identity, icp, &client).await?;
    if current.is_none() {
        if read_only {
            return Err(Error::Blocked(
                "Root has not retained this Store operation; reconcile cannot admit it".into(),
            ));
        }
        if operation::module(&client, principal(&approval.store)?).await?
            != crate::hex(&approval.previous_module)
        {
            return Err(invalid());
        }
        let reply =
            signed::execute(&dir.join("stage"), plan, &stage, identity, icp, &client).await?;
        let staged: std::result::Result<Status, canic_management_protocol::Refusal> =
            ic_wire::decode(
                &reply,
                ic_wire::Encoding::Frame {
                    method: wire::STAGE,
                    direction: ic_wire::Direction::Response,
                },
                wire::LIMITS,
            )
            .map_err(supplier)?;
        admit(
            plan,
            &staged
                .map_err(|r| Error::Blocked(format!("Root refused Store maintenance: {r:?}")))?,
        )?;
        (index, current) = observe(dir, plan, identity, icp, &client).await?;
    }
    // A matching remote record cannot invent the local operator's signed admission custody.
    signed::validate_retained(&dir.join("stage"), plan, &stage)?;
    let mut current = current.ok_or_else(invalid)?;
    for _ in 0..STEPS_PER_APPLY {
        if ready(&current) {
            break;
        }
        let next = continuation::step(current.phase);
        if read_only
            && !matches!(
                next,
                StoreContinuation::RecoverIssued { .. } | StoreContinuation::VerifyCatalog { .. }
            )
        {
            break;
        }
        verify(plan, &client).await?;
        let action = path(dir, index).join("continue");
        if read_only || matches!(next, StoreContinuation::RecoverIssued { .. }) {
            let reply = signed::reconcile(&action, plan, identity, icp, &client).await?;
            let result: Status = ic_wire::decode(
                &reply,
                ic_wire::Encoding::Frame {
                    method: wire::RECOVER,
                    direction: ic_wire::Direction::Response,
                },
                wire::LIMITS,
            )
            .map_err(supplier)?;
            admit(plan, &result)?;
        } else {
            let (request, id, result) = match next {
                StoreContinuation::Install => (
                    call(
                        plan,
                        "install_store_maintenance",
                        wire::INSTALL,
                        &wire::Install {
                            approval: approval.clone(),
                            wasm: wasm.to_vec(),
                        },
                        wire::INSTALL_LIMITS,
                    )?,
                    wire::INSTALL,
                    true,
                ),
                StoreContinuation::Activate => (
                    call(
                        plan,
                        "activate_store_maintenance",
                        wire::ACTIVATE,
                        &approval,
                        wire::LIMITS,
                    )?,
                    wire::ACTIVATE,
                    false,
                ),
                _ => (
                    call(
                        plan,
                        "advance_store_maintenance",
                        wire::ADVANCE,
                        &approval,
                        wire::LIMITS,
                    )?,
                    wire::ADVANCE,
                    false,
                ),
            };
            let reply = signed::execute(&action, plan, &request, identity, icp, &client).await?;
            let value: Status = if result {
                ic_wire::decode::<std::result::Result<Status, canic_management_protocol::Refusal>>(
                    &reply,
                    ic_wire::Encoding::Frame {
                        method: id,
                        direction: ic_wire::Direction::Response,
                    },
                    wire::LIMITS,
                )
                .map_err(supplier)?
                .map_err(|r| Error::Blocked(format!("Root refused Store installation: {r:?}")))?
            } else {
                ic_wire::decode(
                    &reply,
                    ic_wire::Encoding::Frame {
                        method: id,
                        direction: ic_wire::Direction::Response,
                    },
                    wire::LIMITS,
                )
                .map_err(supplier)?
            };
            admit(plan, &value)?;
        }
        let (observed_index, observed) = observe(dir, plan, identity, icp, &client).await?;
        let observed = observed.ok_or_else(invalid)?;
        let stalled = observed == current || observed.failure.is_some();
        index = observed_index;
        current = observed;
        if stalled {
            break;
        }
    }
    if ready(&current) {
        if operation::module(&client, principal(&approval.store)?).await? != plan.artifact.sha256 {
            return Err(invalid());
        }
        store::put_json(&dir.join("store-completion.json"), &index)?;
        return Ok(ApplyOutcome::Complete(
            receipt(dir, plan)?.ok_or_else(invalid)?.0,
        ));
    }
    Ok(ApplyOutcome::StorePending {
        operation: plan.digest()?,
        root: plan.application_controller().into(),
        store_maintenance: Box::new(current),
    })
}

pub(crate) fn status(dir: &Path, plan: &Plan) -> Result<crate::CanisterStatus> {
    let end = next(dir)?;
    let mut effects = std::collections::BTreeMap::<String, String>::new();
    effects.insert(
        "stage".into(),
        operation::attempt_state(&dir.join("stage"))?.into(),
    );
    for index in 0..=end.min(OBSERVATIONS - 1) {
        let path = path(dir, index);
        if !path.try_exists().map_err(supplier)? {
            break;
        }
        effects.insert(
            format!("store/{index}/observe"),
            operation::attempt_state(&path)?.into(),
        );
        effects.insert(
            format!("store/{index}/continue"),
            operation::attempt_state(&path.join("continue"))?.into(),
        );
    }
    let current = if end > 0 {
        retained(dir, plan, end - 1)?
    } else {
        None
    };
    Ok(crate::CanisterStatus {
        operation: plan.digest()?,
        plan: plan.clone(),
        effects,
        receipt: receipt(dir, plan)?.map(|(r, _)| r),
        management: None,
        detail: crate::StatusDetail::StoreMaintenance {
            store_maintenance: current.map(Box::new),
        },
    })
}

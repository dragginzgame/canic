//! App-owned command codecs; Canic owns direct ingress custody, never application permission.

#[cfg(test)]
mod tests;

pub mod codec;

use crate::{
    Artifact, Error, Plan, PlanAction, Result, SelectedFleet, artifact, plan, signed, store,
    supplier,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

/// Transport request ceiling, independent of the application's codec budget.
pub const MAX_REQUEST_BYTES: usize = 65_536;

/// A codec's exact update request. Caller identity cannot be supplied here.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Actual IC update method.
    pub method: String,
    /// Complete encoded application payload.
    pub arguments: Vec<u8>,
}

/// Exact direct call authority, with request identity in the plan's artifact field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    /// Selected application principal; never a Root proxy.
    pub canister: String,
    /// Exact update method.
    pub method: String,
    /// App-owned command and arguments, for review only; never re-encoded on resume.
    pub command: Vec<String>,
}

/// Original replicated reply; app-level errors remain replies, not retry permission.
#[derive(Debug, Serialize)]
pub struct Reply {
    /// Approval digest used by status/apply/reconcile.
    pub operation: String,
    /// Uninterpreted application response, retained before invoking its decoder.
    pub bytes: Vec<u8>,
}

pub(crate) fn validate(plan: &Plan, value: &Approval) -> Result<()> {
    let id = crate::network::principal(&value.canister)?;
    if crate::network::principal(&plan.controller)?.to_text() != plan.controller
        || id.to_text() != value.canister
        || id.as_slice().is_empty()
        || id == ic_platform_types::Principal::anonymous()
        || value.method.is_empty()
        || value.method.len() > 128
        || !value
            .method
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        || value.command.is_empty()
        || value.command.len() > 32
        || value
            .command
            .iter()
            .any(|s| s.len() > 1024 || s.contains('\0'))
        || plan.artifact.bytes > MAX_REQUEST_BYTES
        || crate::unhex(&plan.artifact.sha256)?.len() != 32
        || plan.artifact.sha256 != plan.artifact.sha256.to_ascii_lowercase()
    {
        return Err(Error::Invalid(
            "invalid or unbounded application call".into(),
        ));
    }
    Ok(())
}

/// Resolve a declared slot or completed deployment, without inferring caller authority.
/// # Errors
/// Refuses missing, conflicting or malformed deployment evidence.
pub fn target(root: &Path, fleet: &SelectedFleet, slot: &str) -> Result<String> {
    let definition = fleet
        .config
        .canisters
        .get(slot)
        .ok_or_else(|| Error::Invalid(format!("unknown canister {slot}")))?;
    let inventory = crate::fleet::inventory(root, true, crate::now()?)?;
    if !inventory.issues.is_empty() {
        return Err(Error::Invalid(
            "application target inventory contains invalid evidence".into(),
        ));
    }
    let mut ids: BTreeSet<_> = definition.id.iter().cloned().collect();
    for (record, network) in inventory.records {
        if network == fleet.config.network
            && record.receipt_retained
            && record
                .deployment
                .as_ref()
                .is_some_and(|d| d.app == fleet.app && d.fleet == fleet.fleet && d.canister == slot)
            && let Some(id) = record.canister
        {
            ids.insert(id);
        }
    }
    if ids.len() != 1 {
        return Err(Error::Invalid(
            "application call needs one bound or completed canister identity".into(),
        ));
    }
    ids.pop_first()
        .ok_or_else(|| Error::Invalid("missing application identity".into()))
}

/// Retain one exact request under the existing operation journal.
/// No network effect occurs here. An unresolved call by this caller to this target blocks a new one.
/// # Errors
/// Refuses ambiguous targets, oversized requests or retained uncertainty, including after expiry.
pub fn prepare(
    root: &Path,
    fleet: &SelectedFleet,
    slot: &str,
    caller: String,
    command: Vec<String>,
    request: Request,
) -> Result<(String, Plan)> {
    let canister = target(root, fleet, slot)?;
    let now = crate::now()?;
    let plan = Plan {
        version: 7,
        network: fleet.config.network.clone(),
        controller: caller,
        artifact: Artifact {
            sha256: artifact::digest(&request.arguments),
            bytes: request.arguments.len(),
        },
        action: PlanAction::Call(Approval {
            canister,
            method: request.method,
            command,
        }),
        created_at_ns: now,
        expires_at_ns: now + 600_000_000_000,
        deployment: Some(crate::DeploymentName {
            app: fleet.app.clone(),
            fleet: fleet.fleet.clone(),
            canister: slot.into(),
        }),
    };
    plan.validate()?;
    let held = store::Store::open(root)?;
    let (ids, issues) = crate::fleet::operation_ids(root, false)?;
    if !issues.is_empty() {
        return Err(Error::Invalid("invalid call journal inventory".into()));
    }
    for id in ids {
        let dir = held.operation(&id)?;
        if crate::fleet_plan::exists(&dir)? {
            continue;
        }
        let (old, bytes) = plan::load(&dir, &id)?;
        if let Some(call) = old.application_call()
            && old.network == plan.network
            && old.controller == plan.controller
            && plan
                .application_call()
                .is_some_and(|proposed| call.canister == proposed.canister)
            && receipt(&dir, &old, &bytes)?.is_none()
        {
            return Err(Error::Blocked(format!(
                "application call {id} has no retained reply; observe that operation, never issue a replacement"
            )));
        }
    }
    plan::publish_locked(&held, plan, &request.arguments)
}

fn call<'a>(plan: &'a Plan, bytes: &[u8]) -> Result<signed::Call<'a>> {
    let approval = plan
        .application_call()
        .ok_or_else(|| Error::Invalid("application call required".into()))?;
    let id = crate::network::principal(&approval.canister)?;
    Ok(signed::Call {
        canister: id,
        effective: id,
        method: &approval.method,
        args: bytes.to_vec(),
    })
}

fn receipt(dir: &Path, plan: &Plan, bytes: &[u8]) -> Result<Option<Reply>> {
    let path = dir.join("call");
    if !path.join("reply.bin").try_exists().map_err(supplier)? {
        return Ok(None);
    }
    signed::validate_retained(&path, plan, &call(plan, bytes)?)?;
    Ok(Some(Reply {
        operation: plan.digest()?,
        bytes: store::read(&path.join("reply.bin"), 2 * 1024 * 1024)?,
    }))
}

pub(crate) async fn apply(
    dir: &Path,
    plan: &Plan,
    bytes: &[u8],
    identity: &str,
    icp: &Path,
    reconcile: bool,
) -> Result<crate::ApplyOutcome> {
    if let Some(call) = receipt(dir, plan, bytes)? {
        return Ok(crate::ApplyOutcome::Call { call });
    }
    let path = dir.join("call");
    let prepared = path.join("signed.json").try_exists().map_err(supplier)?;
    if reconcile && !prepared {
        return Err(Error::Blocked(
            "call has not been signed; apply its original approval first".into(),
        ));
    }
    signed::execute(
        &path,
        plan,
        &call(plan, bytes)?,
        identity,
        icp,
        &plan.network.client()?,
    )
    .await?;
    let call = receipt(dir, plan, bytes)?
        .ok_or_else(|| Error::Blocked("application has no retained reply".into()))?;
    Ok(crate::ApplyOutcome::Call { call })
}

pub(crate) fn status(dir: &Path, plan: &Plan, bytes: &[u8]) -> Result<crate::CanisterStatus> {
    Ok(crate::CanisterStatus {
        operation: plan.digest()?,
        plan: plan.clone(),
        receipt: None,
        management: None,
        effects: std::collections::BTreeMap::from([(
            "call".into(),
            crate::operation::attempt_state(&dir.join("call"))?.into(),
        )]),
        detail: crate::StatusDetail::Call {
            call: receipt(dir, plan, bytes)?,
        },
    })
}

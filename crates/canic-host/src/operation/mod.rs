use crate::{
    Error, Network, Plan, Result,
    network::{bounded, principal},
    plan,
    signed::{self, Call},
    store::{self, Store},
    supplier,
};
use ic_platform::{
    agent::Agent,
    candid_codec::{self, decoder},
    cycles_ledger_types as ledger, management_canister_types as mgmt,
    principal::Principal,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[cfg(test)]
mod tests;

const LEDGER: &str = "um5iw-rqaaa-aaaaq-qaaba-cai";
const LEDGER_FEE: u64 = 100_000_000;
const DECODE: decoder::Limits = decoder::Limits {
    bytes: 2 * 1024 * 1024,
    header: 4096,
    types: 256,
    work: 100_000,
    allocation: 4 * 1024 * 1024,
};

/// Retained outcome verified against the approved artifact and controller.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    /// Original approved deployment digest.
    pub operation: String,
    /// Exact application canister principal.
    pub canister: String,
    /// Certified installed module hash at completion.
    pub module_hash: String,
    /// Sole certified controller at completion.
    pub controller: String,
}

/// One named child outcome within a fleet operation.
#[derive(Debug, Serialize)]
pub struct ChildOutcome {
    /// Retained application canister name.
    pub name: String,
    /// Admission or completion evidence for this child.
    pub outcome: ApplyOutcome,
}

/// Applying may admit autonomous work before the child is ready.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ApplyOutcome {
    /// Store publication progress; it does not report an installed canister.
    Publication {
        /// Exact release evidence from the approved Store through Root.
        publication: crate::PublicationProgress,
    },
    /// One fleet approval, composed from the existing Root and child effect owners.
    Fleet {
        /// Original whole-fleet approval digest.
        operation: String,
        /// Verified Root installation.
        root: Receipt,
        /// Named outcomes in permanent child order.
        children: Vec<ChildOutcome>,
    },
    /// Verified completed installation; direct deployment JSON remains unchanged.
    Complete(Receipt),
    /// Root retains the exact work; apply again to retain a replicated observation.
    Pending {
        /// Original deployment digest.
        operation: String,
        /// Enduring effect owner.
        root: String,
        /// Authenticated historical progress, not a completion receipt.
        status: Box<canic_management_protocol::Status>,
    },
    /// Root retains a child upgrade; completion requires its authenticated evidence.
    UpgradePending {
        /// Retained operation digest.
        operation: String,
        /// Enduring management owner.
        root: String,
        /// Committed upgrade progress.
        upgrade: Box<canic_management_protocol::upgrade::Status>,
    },
}

fn create_call(plan: &Plan) -> Result<Call<'static>> {
    let controller = principal(&plan.controller)?;
    if plan.network == Network::Mainnet {
        let args = ledger::CreateCanisterArgs {
            from_subaccount: None,
            created_at_time: Some(plan.created_at_ns),
            amount: (plan.creation_cycles() - LEDGER_FEE).into(),
            creation_args: Some(ledger::CmcCreateCanisterArgs {
                subnet_selection: None,
                settings: Some(ledger::CanisterSettings {
                    controllers: Some(vec![controller]),
                    compute_allocation: Some(0_u64.into()),
                    memory_allocation: Some(0_u64.into()),
                    freezing_threshold: None,
                    reserved_cycles_limit: None,
                    wasm_memory_limit: None,
                    wasm_memory_threshold: None,
                    log_visibility: None,
                    environment_variables: None,
                }),
            }),
        };
        let canister = principal(LEDGER)?;
        Ok(Call {
            canister,
            effective: canister,
            method: "create_canister",
            args: candid_codec::encode_one(&args, 4096).map_err(Error::Encode)?,
        })
    } else {
        let args = mgmt::ProvisionalCreateCanisterWithCyclesArgs {
            amount: Some(plan.creation_cycles().into()),
            settings: Some(mgmt::CanisterSettings {
                controllers: Some(vec![controller]),
                ..Default::default()
            }),
            ..Default::default()
        };
        let Network::Local {
            routing_canister, ..
        } = &plan.network
        else {
            unreachable!()
        };
        Ok(Call {
            canister: Principal::management_canister(),
            effective: principal(routing_canister)?,
            method: "provisional_create_canister_with_cycles",
            args: candid_codec::encode_one(&args, 4096).map_err(Error::Encode)?,
        })
    }
}

pub(crate) fn created(plan: &Plan, bytes: &[u8]) -> Result<Principal> {
    if plan.network == Network::Mainnet {
        let result: ledger::CreateCanisterResult =
            decoder::decode_one(bytes, DECODE).map_err(Error::Decode)?;
        match result {
            Ok(value) => Ok(value.canister_id),
            Err(ledger::CreateCanisterError::Duplicate {
                canister_id: Some(id),
                ..
            }) => Ok(id),
            Err(error) => Err(Error::Blocked(format!(
                "cycles ledger returned {error:?}; creation result retained, no automatic retry"
            ))),
        }
    } else {
        let result: mgmt::CanisterIdRecord =
            decoder::decode_one(bytes, DECODE).map_err(Error::Decode)?;
        Ok(result.canister_id)
    }
}

pub(crate) async fn controllers(
    agent: &Agent,
    canister: Principal,
    controller: &str,
) -> Result<()> {
    let bytes = bounded(
        agent
            .read_state_canister_info(canister.to_text().parse().map_err(supplier)?, "controllers"),
    )
    .await?;
    let values = ic_platform::ic_protocol::decode_controllers(&bytes, 4096).map_err(supplier)?;
    if values != [principal(controller)?] {
        return Err(Error::Blocked(
            "certified controllers differ from the approved sole controller".into(),
        ));
    }
    Ok(())
}

pub(crate) async fn module(agent: &Agent, canister: Principal) -> Result<String> {
    Ok(crate::hex(
        &bounded(agent.read_state_canister_info(
            canister.to_text().parse().map_err(supplier)?,
            "module_hash",
        ))
        .await?,
    ))
}

/// Apply or resume precisely the named plan using ICP CLI only for offline signing.
///
/// # Errors
/// Refuses altered plans, authority/artifact mismatch, expired unresolved attempts,
/// concurrent operators and provider failures. Retained signed requests must never be deleted to retry.
pub async fn apply(root: &Path, digest: &str, identity: &str, icp: &Path) -> Result<ApplyOutcome> {
    let store = Store::open(root)?;
    let dir = store.operation(digest)?;
    if crate::fleet_plan::exists(&dir)? {
        return crate::fleet_plan::apply(&dir, digest, identity, icp).await;
    }
    apply_at(&dir, digest, identity, icp).await
}

/// Request recovery of the named Root approval, retaining the exact signed request.
/// This does not renew its effect budgets or deadlines and cannot bootstrap a fleet.
///
/// # Errors
/// Refuses unmanaged or unadmitted work, changed Root authority and unresolved expired attempts.
pub async fn reconcile(
    root: &Path,
    digest: &str,
    identity: &str,
    icp: &Path,
) -> Result<ApplyOutcome> {
    let store = Store::open(root)?;
    let dir = store.operation(digest)?;
    if crate::fleet_plan::exists(&dir)? {
        return crate::fleet_plan::reconcile(&dir, digest, identity, icp).await;
    }
    reconcile_at(&dir, digest, identity, icp).await
}

pub(crate) async fn reconcile_at(
    dir: &Path,
    digest: &str,
    identity: &str,
    icp: &Path,
) -> Result<ApplyOutcome> {
    let (plan, bytes) = plan::load(dir, digest)?;
    if plan.publication().is_some() {
        return crate::publication::apply(dir, digest, &plan, &bytes, identity, icp, true).await;
    }
    let agent = plan.network.agent()?;
    crate::managed::recovery::reconcile(dir, digest, &plan, &bytes, identity, icp, &agent).await
}

// Caller holds the journal lock; fleet composition uses this same effect owner.
pub(crate) async fn apply_at(
    dir: &Path,
    digest: &str,
    identity: &str,
    icp: &Path,
) -> Result<ApplyOutcome> {
    let (plan, wasm) = plan::load(dir, digest)?;
    if plan.publication().is_some() {
        return crate::publication::apply(dir, digest, &plan, &wasm, identity, icp, false).await;
    }
    let agent = plan.network.agent()?;
    if matches!(
        plan.action,
        crate::PlanAction::CreateChild(_) | crate::PlanAction::UpgradeChild { .. }
    ) {
        return crate::managed::apply(dir, digest, &plan, &wasm, identity, icp, &agent).await;
    }
    if dir.join("receipt.json").try_exists().map_err(supplier)? {
        let receipt: Receipt = store::json(&dir.join("receipt.json"))?;
        let expected = if let Some(target) = plan.target() {
            principal(target)?
        } else {
            created(
                &plan,
                &store::read(&dir.join("create/reply.bin"), 2 * 1024 * 1024)?,
            )?
        };
        if principal(&receipt.canister)? != expected {
            return Err(Error::Invalid(
                "receipt substituted the approved target or retained creation output".into(),
            ));
        }
        verify_receipt(&agent, &plan, digest, &receipt).await?;
        if !crate::maintenance::retained(dir, &plan)? {
            return Err(Error::Invalid(
                "maintenance receipt lacks retained custody".into(),
            ));
        }
        return Ok(ApplyOutcome::Complete(receipt));
    }
    let canister = if let Some(target) = plan.target() {
        principal(target)?
    } else {
        if plan.network == Network::Mainnet
            && !dir
                .join("create/signed.json")
                .try_exists()
                .map_err(supplier)?
        {
            let fee_args = candid_codec::arguments::encode_args(&(), 128).map_err(Error::Encode)?;
            let fee = bounded(
                agent
                    .query(&LEDGER.parse().map_err(supplier)?, "icrc1_fee")
                    .with_arg(fee_args)
                    .call(),
            )
            .await?;
            let fee: candid_codec::Nat =
                decoder::decode_one(&fee, DECODE).map_err(Error::Decode)?;
            if fee != LEDGER_FEE {
                return Err(Error::Blocked(
                    "cycles-ledger fee changed; review a new supported funding contract".into(),
                ));
            }
        }
        let call = create_call(&plan)?;
        let reply =
            signed::execute(&dir.join("create"), &plan, &call, identity, icp, &agent).await?;
        created(&plan, &reply)?
    };
    controllers(&agent, canister, &plan.controller).await?;
    // Before the first install signature, enforce the exact old module for an
    // upgrade. On resume, the retained attempt owns settlement; do not replace it.
    if !dir
        .join("install/signed.json")
        .try_exists()
        .map_err(supplier)?
        && let Some(expected) = plan.previous_module()
        && module(&agent, canister).await? != *expected
    {
        return Err(Error::Blocked(
            "upgrade precondition: previous module hash changed".into(),
        ));
    }
    if !dir
        .join("install/signed.json")
        .try_exists()
        .map_err(supplier)?
    {
        crate::maintenance::observe(dir, &plan, false, identity, icp, &agent).await?;
    }
    crate::maintenance::require_before(dir, &plan)?;
    let args = mgmt::InstallCodeArgs {
        mode: if plan.target().is_some() {
            mgmt::CanisterInstallMode::Upgrade(None)
        } else {
            mgmt::CanisterInstallMode::Install
        },
        canister_id: canister,
        wasm_module: wasm,
        arg: vec![],
        sender_canister_version: None,
    };
    let call = Call {
        canister: Principal::management_canister(),
        effective: canister,
        method: "install_code",
        args: candid_codec::encode_one(&args, 2 * 1024 * 1024).map_err(Error::Encode)?,
    };
    let reply = signed::execute(&dir.join("install"), &plan, &call, identity, icp, &agent).await?;
    decoder::decode_args::<()>(&reply, DECODE, decoder::Arity::Exact).map_err(Error::Decode)?;
    crate::maintenance::observe(dir, &plan, true, identity, icp, &agent).await?;
    let receipt = Receipt {
        operation: digest.into(),
        canister: canister.to_text(),
        module_hash: plan.artifact.sha256.clone(),
        controller: plan.controller.clone(),
    };
    verify_receipt(&agent, &plan, digest, &receipt).await?;
    store::put_json(&dir.join("receipt.json"), &receipt)?;
    Ok(ApplyOutcome::Complete(receipt))
}

pub(crate) async fn verify_receipt(
    agent: &Agent,
    plan: &Plan,
    digest: &str,
    receipt: &Receipt,
) -> Result<()> {
    if receipt.operation != digest
        || receipt.module_hash != plan.artifact.sha256
        || receipt.controller != plan.application_controller()
    {
        return Err(Error::Invalid(
            "receipt does not match approved plan".into(),
        ));
    }
    let canister = principal(&receipt.canister)?;
    controllers(agent, canister, plan.application_controller()).await?;
    if module(agent, canister).await? != plan.artifact.sha256 {
        return Err(Error::Blocked(
            "certified module differs from approved artifact".into(),
        ));
    }
    Ok(())
}

/// Inspect retained progress without signing, sending or spending mainnet cycles.
///
/// # Errors
/// Refuses malformed or altered retained inputs. Reports local evidence, not current network state.
pub fn status(root: &Path, digest: &str) -> Result<serde_json::Value> {
    let store = Store::open(root)?;
    let dir = store.operation(digest)?;
    if crate::fleet_plan::exists(&dir)? {
        return crate::fleet_plan::status(&dir, digest);
    }
    status_at(&dir, digest)
}

pub(crate) fn status_at(dir: &Path, digest: &str) -> Result<serde_json::Value> {
    let (plan, bytes) = plan::load(dir, digest)?;
    if plan.publication().is_some() {
        return crate::publication::status(dir, digest, &plan, &bytes);
    }
    let mut effects = serde_json::Map::new();
    let effect_names: Vec<_> = match plan.action {
        crate::PlanAction::Publish(_) => unreachable!("publication status selected above"),
        crate::PlanAction::MaintainRoot { .. } => crate::maintenance::observations(&plan, false)
            .iter()
            .chain(crate::maintenance::observations(&plan, true))
            .map(|(step, _)| *step)
            .chain(std::iter::once("install"))
            .collect(),
        crate::PlanAction::CreateChild(_) | crate::PlanAction::UpgradeChild { .. } => vec!["stage"],
        crate::PlanAction::Create { .. } | crate::PlanAction::Upgrade(_) => {
            vec!["create", "install"]
        }
    };
    for effect in effect_names {
        let path = dir.join(effect);
        effects.insert(effect.into(), attempt_state(&path)?.into());
    }
    if plan.management().is_some() {
        for index in 0..crate::managed::OBSERVATIONS {
            let path = dir.join(format!("observe/{index}/reconcile"));
            if path.join("authority.json").try_exists().map_err(supplier)? {
                effects.insert(format!("reconcile/{index}"), attempt_state(&path)?.into());
            }
        }
    }
    let receipt: Option<Receipt> = if dir.join("receipt.json").try_exists().map_err(supplier)? {
        Some(store::json(&dir.join("receipt.json"))?)
    } else {
        None
    };
    let managed = match &plan.action {
        crate::PlanAction::UpgradeChild { .. } => {
            serde_json::to_value(crate::managed::upgrade::evidence(dir, &plan)?)
                .map_err(supplier)?
        }
        crate::PlanAction::CreateChild(_) => {
            serde_json::to_value(crate::managed::evidence(dir, &plan)?).map_err(supplier)?
        }
        _ => serde_json::Value::Null,
    };
    Ok(
        serde_json::json!({"operation": digest, "plan": plan, "effects": effects, "receipt": receipt, "management": managed}),
    )
}

pub(crate) fn attempt_state(path: &Path) -> Result<&'static str> {
    for (file, state) in [
        ("reply.bin", "reply_retained"),
        ("rejected.json", "rejection_retained"),
        ("signed.json", "signed_attempt_retained_outcome_unknown"),
        ("authority.json", "authority_retained_not_signed"),
    ] {
        if path.join(file).try_exists().map_err(supplier)? {
            return Ok(state);
        }
    }
    Ok("not_submitted")
}

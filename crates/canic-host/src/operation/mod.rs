#[cfg(test)]
mod tests;

mod status;

use crate::{
    Error, Network, Plan, Result,
    network::{bounded, principal},
    plan,
    signed::{self, Call},
    store::{self, Store},
    supplier,
};
use ic_platform::native::ReadClient;
use ic_platform_protocol::{candid::Limits, ledger, management as mgmt};
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};
pub use status::{
    CanisterStatus, FleetChildStatus, FleetStatus, FleetStepStatus, ManagementStatus,
    OperationStatus, StatusDetail,
};
use std::path::Path;

const LEDGER: &str = "um5iw-rqaaa-aaaaq-qaaba-cai";
const LEDGER_FEE: u64 = 100_000_000;
const DECODE: Limits = Limits {
    bytes: 2 * 1024 * 1024,
    header: 4096,
    types: 256,
    work: 100_000,
    allocation: 4 * 1024 * 1024,
    vector: 2 * 1024 * 1024,
    skipping: 100_000,
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
    /// Original application reply bytes; interpretation belongs to the app.
    Call {
        /// Exact request's retained result, never a deployment receipt.
        call: crate::application::Reply,
    },
    /// Exact successful Auth configuration or revocation response.
    Auth {
        /// Historical response, never receiver readiness.
        auth: crate::AuthReceipt,
    },
    /// Retained Store upgrade progress, without a completion claim.
    StorePending {
        /// Original approval digest.
        operation: String,
        /// Enduring Store effect owner.
        root: String,
        /// Authenticated retained operation, including uncertainty.
        store_maintenance: Box<canic_management_protocol::store_maintenance::Status>,
    },
    /// Successful exact removal reply; restoration still requires separate authority.
    Removal {
        /// Validated original response and dispatch deadline.
        removal: crate::RemovalReceipt,
    },
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
        /// Optional infrastructure completion, separate from application slots.
        #[serde(skip_serializing_if = "Option::is_none")]
        store: Option<Receipt>,
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
        /// Optional Blob lifecycle evidence from the same complete Root observations.
        #[serde(skip_serializing_if = "Option::is_none")]
        maintenance: Option<Box<canic_management_protocol::application::Maintenance>>,
    },
}

fn create_call(plan: &Plan, subnet: Option<Principal>) -> Result<Call<'static>> {
    if plan.store_root().is_some() != subnet.is_some() {
        return Err(Error::Invalid(
            "Store creation requires retained Root subnet placement".into(),
        ));
    }
    let controller = principal(&plan.controller)?;
    if plan.network == Network::Mainnet {
        let args = ledger::CreateCanisterArgs {
            from_subaccount: None,
            created_at_time: Some(plan.created_at_ns),
            amount: (plan.creation_cycles() - LEDGER_FEE).into(),
            creation_args: Some(ledger::CmcCreationArgs {
                subnet_selection: subnet.map(|subnet| {
                    ledger::SubnetSelection::Subnet(ledger::ExplicitSubnet { subnet })
                }),
                settings: Some(mgmt::CreationSettings {
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
            args: args.encode(4096).map_err(Error::Encode)?,
        })
    } else {
        // Retained signed attempts require the existing complete settings schema.
        let args = mgmt::ProvisionalCreateArgs {
            amount: Some(plan.creation_cycles().into()),
            settings: Some(mgmt::CanisterSettings {
                controllers: Some(vec![controller]),
                ..Default::default()
            }),
            specified_id: None,
            sender_canister_version: None,
        };
        let Network::Local {
            routing_canister, ..
        } = &plan.network
        else {
            unreachable!()
        };
        Ok(Call {
            canister: Principal::management_canister(),
            effective: principal(
                plan.store_root()
                    .map_or(routing_canister.as_str(), |root| root.canister.as_str()),
            )?,
            method: "provisional_create_canister_with_cycles",
            args: args.encode(4096).map_err(Error::Encode)?,
        })
    }
}

pub(crate) fn created(plan: &Plan, bytes: &[u8]) -> Result<Principal> {
    if plan.network == Network::Mainnet {
        let result = ledger::decode_created(bytes, DECODE).map_err(Error::Decode)?;
        match result {
            ledger::CreateResult::Ok(value) => Ok(value.canister_id),
            ledger::CreateResult::Err(ledger::CreateError::Duplicate(ledger::Duplicate {
                canister_id: Some(id),
                ..
            })) => Ok(id),
            ledger::CreateResult::Err(error) => Err(Error::Blocked(format!(
                "cycles ledger returned {error:?}; creation result retained, no automatic retry"
            ))),
        }
    } else {
        let result = mgmt::decode_created(bytes, DECODE).map_err(Error::Decode)?;
        Ok(result.canister_id)
    }
}

pub(crate) async fn controllers(
    client: &ReadClient,
    canister: Principal,
    controller: &str,
) -> Result<()> {
    let (expiry, time) = crate::network::context()?;
    let observation = bounded(client.read_canister(canister, expiry, time, 10)).await?;
    if observation.controllers() != [principal(controller)?] {
        return Err(Error::Blocked(
            "certified controllers differ from the approved sole controller".into(),
        ));
    }
    Ok(())
}

pub(crate) async fn module(client: &ReadClient, canister: Principal) -> Result<String> {
    let (expiry, time) = crate::network::context()?;
    Ok(crate::hex(
        &bounded(client.canister_module_hash(canister, expiry, time)).await?,
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
        return crate::fleet_plan::execute(&dir, digest, identity, icp, false).await;
    }
    let plan: Plan = store::json(&dir.join("plan.json"))?;
    if matches!(plan.action, crate::PlanAction::Upgrade(_))
        || (matches!(plan.action, crate::PlanAction::Create { .. }) && plan.version != 10)
    {
        return Err(Error::Blocked("direct deployment approvals from before mandatory fleet Stores require their original Host".into()));
    }
    apply_at(root, &dir, digest, identity, icp).await
}

/// Observe an original application call or request recovery of a named Root approval.
/// Keeps exact signed-request custody; never renews effect budgets or deadlines or bootstraps a fleet.
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
        return crate::fleet_plan::execute(&dir, digest, identity, icp, true).await;
    }
    reconcile_at(root, &dir, digest, identity, icp).await
}

pub(crate) async fn reconcile_at(
    journal: &Path,
    dir: &Path,
    digest: &str,
    identity: &str,
    icp: &Path,
) -> Result<ApplyOutcome> {
    let (plan, bytes) = plan::load(dir, digest)?;
    if plan.application_call().is_some() {
        return crate::application::apply(dir, &plan, &bytes, identity, icp, true).await;
    }
    if plan.auth().is_some() {
        return crate::auth::apply(dir, &plan, identity, icp, true).await;
    }
    if plan.removal().is_some() {
        return crate::publication::removal::apply(dir, &plan, &bytes, identity, icp, true).await;
    }
    if plan.store_upgrade_root().is_some() {
        return crate::store_upgrade::apply(dir, &plan, &bytes, identity, icp, true).await;
    }
    if plan.publication().is_some() {
        return crate::publication::apply(journal, dir, &plan, &bytes, identity, icp, true).await;
    }
    if plan.store_root().is_some() {
        return apply_operation(journal, dir, digest, identity, icp, true).await;
    }
    crate::auth::permit_upgrade(journal, &plan)?;
    let client = plan.network.client()?;
    crate::managed::recovery::reconcile(dir, digest, &plan, identity, icp, &client).await
}

// Caller holds the journal lock; fleet composition uses this same effect owner.
pub(crate) async fn apply_at(
    journal: &Path,
    dir: &Path,
    digest: &str,
    identity: &str,
    icp: &Path,
) -> Result<ApplyOutcome> {
    apply_operation(journal, dir, digest, identity, icp, false).await
}

async fn apply_operation(
    journal: &Path,
    dir: &Path,
    digest: &str,
    identity: &str,
    icp: &Path,
    read_only: bool,
) -> Result<ApplyOutcome> {
    let (plan, wasm) = plan::load(dir, digest)?;
    if plan.application_call().is_some() {
        return crate::application::apply(dir, &plan, &wasm, identity, icp, false).await;
    }
    if plan.auth().is_some() {
        return crate::auth::apply(dir, &plan, identity, icp, read_only).await;
    }
    crate::auth::permit_upgrade(journal, &plan)?;
    if plan.removal().is_some() {
        return crate::publication::removal::apply(dir, &plan, &wasm, identity, icp, read_only)
            .await;
    }
    if plan.store_upgrade_root().is_some() {
        return crate::store_upgrade::apply(dir, &plan, &wasm, identity, icp, false).await;
    }
    if plan.publication().is_some() {
        return crate::publication::apply(journal, dir, &plan, &wasm, identity, icp, false).await;
    }
    let client = plan.network.client()?;
    crate::store_bootstrap::verify_root(&client, &plan).await?;
    crate::store_bootstrap::prepare_placement(dir, &plan, &client).await?;
    if matches!(
        plan.action,
        crate::PlanAction::CreateChild(_) | crate::PlanAction::UpgradeChild { .. }
    ) {
        return crate::managed::apply(dir, digest, &plan, identity, icp, &client).await;
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
        if plan.store_root().is_some() {
            crate::store_bootstrap::verify_placement(dir, &plan, expected, &client).await?;
            store_install_evidence(dir, &plan, &wasm, expected)?;
            signed::validate_retained(
                &dir.join("handover"),
                &plan,
                &crate::store_bootstrap::handover(&plan, expected)?,
            )?;
            crate::store_bootstrap::retained(dir, &plan, expected)?;
        }
        verify_receipt(&client, &plan, digest, &receipt).await?;
        if !crate::maintenance::retained(dir, &plan)? {
            return Err(Error::Invalid(
                "maintenance receipt lacks retained custody".into(),
            ));
        }
        return Ok(ApplyOutcome::Complete(receipt));
    }
    let handover_prepared = plan.store_root().is_some()
        && dir
            .join("handover/signed.json")
            .try_exists()
            .map_err(supplier)?;
    if read_only && !handover_prepared {
        return Err(Error::Blocked(
            "Store reconcile requires a retained handover attempt; use apply for approved effects"
                .into(),
        ));
    }
    let canister = if handover_prepared {
        created(
            &plan,
            &store::read(&dir.join("create/reply.bin"), 2 * 1024 * 1024)?,
        )?
    } else if let Some(target) = plan.target() {
        principal(target)?
    } else {
        if plan.network == Network::Mainnet
            && !dir
                .join("create/signed.json")
                .try_exists()
                .map_err(supplier)?
        {
            let fee_args = ledger::encode_fee_args(128).map_err(Error::Encode)?;
            let fee =
                crate::network::query(&client, principal(LEDGER)?, "icrc1_fee", &fee_args).await?;
            let fee = ledger::decode_amount(&fee, DECODE).map_err(Error::Decode)?;
            if fee != ledger::Nat::from(LEDGER_FEE) {
                return Err(Error::Blocked(
                    "cycles-ledger fee changed; review a new supported funding contract".into(),
                ));
            }
        }
        let call = create_call(&plan, crate::store_bootstrap::placement(dir, &plan)?)?;
        let reply =
            signed::execute(&dir.join("create"), &plan, &call, identity, icp, &client).await?;
        created(&plan, &reply)?
    };
    crate::store_bootstrap::verify_placement(dir, &plan, canister, &client).await?;
    if handover_prepared {
        store_install_evidence(dir, &plan, &wasm, canister)?;
    } else {
        controllers(&client, canister, &plan.controller).await?;
        // Before the first install signature, enforce the exact old module for an
        // upgrade. On resume, the retained attempt owns settlement; do not replace it.
        if !dir
            .join("install/signed.json")
            .try_exists()
            .map_err(supplier)?
            && let Some(expected) = plan.previous_module()
            && module(&client, canister).await? != *expected
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
            crate::maintenance::observe(dir, &plan, false, identity, icp, &client).await?;
        }
        crate::maintenance::require_before(dir, &plan)?;
        if plan.previous_module().is_some() {
            if dir
                .join("install/signed.json")
                .try_exists()
                .map_err(supplier)?
                && !dir.join("stop/reply.bin").try_exists().map_err(supplier)?
            {
                return Err(Error::Blocked("legacy upgrade attempt has no retained stop evidence; retain it for reconciliation, do not replace it".into()));
            }
            crate::maintenance::runtime(dir, &plan, false, identity, icp, &client).await?;
        }
        let call = install_call(&plan, &wasm, canister)?;
        let reply =
            signed::execute(&dir.join("install"), &plan, &call, identity, icp, &client).await?;
        mgmt::decode_empty_response(&reply, DECODE).map_err(Error::Decode)?;
        if plan.previous_module().is_some() {
            crate::maintenance::runtime(dir, &plan, true, identity, icp, &client).await?;
        }
        crate::maintenance::observe(dir, &plan, true, identity, icp, &client).await?;
    }
    if plan.store_root().is_some() {
        crate::store_bootstrap::finish(dir, &plan, canister, identity, icp, &client, read_only)
            .await?;
    }
    let receipt = Receipt {
        operation: digest.into(),
        canister: canister.to_text(),
        module_hash: plan.artifact.sha256.clone(),
        controller: plan.application_controller().into(),
    };
    verify_receipt(&client, &plan, digest, &receipt).await?;
    store::put_json(&dir.join("receipt.json"), &receipt)?;
    Ok(ApplyOutcome::Complete(receipt))
}

fn install_call(plan: &Plan, wasm: &[u8], canister: Principal) -> Result<Call<'static>> {
    let args = mgmt::InstallCodeArgs {
        mode: if plan.target().is_some() {
            mgmt::InstallMode::Upgrade(None)
        } else {
            mgmt::InstallMode::Install
        },
        canister_id: canister,
        wasm_module: wasm.to_vec(),
        arg: match plan.action {
            crate::PlanAction::Create {
                bootstrap: Some(policy),
                ..
            } => ic_wire::encode(
                &policy,
                ic_wire::Encoding::Payload,
                canic_management_protocol::LIMITS,
            )
            .map_err(crate::supplier)?,
            _ => Vec::new(),
        },
        sender_canister_version: None,
    };
    Ok(Call {
        canister: Principal::management_canister(),
        effective: canister,
        method: "install_code",
        args: args.encode(2 * 1024 * 1024).map_err(Error::Encode)?,
    })
}

fn store_install_evidence(dir: &Path, plan: &Plan, wasm: &[u8], canister: Principal) -> Result<()> {
    signed::validate_retained(
        &dir.join("create"),
        plan,
        &create_call(plan, crate::store_bootstrap::placement(dir, plan)?)?,
    )?;
    signed::validate_retained(
        &dir.join("install"),
        plan,
        &install_call(plan, wasm, canister)?,
    )?;
    mgmt::decode_empty_response(
        &store::read(&dir.join("install/reply.bin"), 2 * 1024 * 1024)?,
        DECODE,
    )
    .map_err(Error::Decode)?;
    Ok(())
}

pub(crate) async fn verify_receipt(
    client: &ReadClient,
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
    controllers(client, canister, plan.application_controller()).await?;
    if module(client, canister).await? != plan.artifact.sha256 {
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
pub fn status(root: &Path, digest: &str) -> Result<OperationStatus> {
    let store = Store::open(root)?;
    let dir = store.operation(digest)?;
    if crate::fleet_plan::exists(&dir)? {
        return crate::fleet_plan::status(&dir, digest)
            .map(|s| OperationStatus::Fleet(Box::new(s)));
    }
    status_at(&dir, digest).map(|s| OperationStatus::Canister(Box::new(s)))
}

pub(crate) fn status_at(dir: &Path, digest: &str) -> Result<CanisterStatus> {
    let (plan, bytes) = plan::load(dir, digest)?;
    status_loaded(dir, digest, plan, &bytes)
}

pub(crate) fn status_loaded(
    dir: &Path,
    digest: &str,
    plan: Plan,
    bytes: &[u8],
) -> Result<CanisterStatus> {
    if plan.application_call().is_some() {
        return crate::application::status(dir, &plan, bytes);
    }
    if plan.auth().is_some() {
        return crate::auth::status(dir, &plan);
    }
    if plan.removal().is_some() {
        return crate::publication::removal::status(dir, &plan, bytes);
    }
    if plan.store_upgrade_root().is_some() {
        return crate::store_upgrade::status(dir, &plan);
    }
    if plan.publication().is_some() {
        return crate::publication::status(dir, digest, &plan, bytes);
    }
    let mut effects = std::collections::BTreeMap::<String, String>::new();
    let effect_names: Vec<_> = match plan.action {
        crate::PlanAction::Auth(_)
        | crate::PlanAction::Call(_)
        | crate::PlanAction::Remove(_)
        | crate::PlanAction::Publish(_) => {
            unreachable!("non-deployment status selected above")
        }
        crate::PlanAction::UpgradeStore { .. } => unreachable!("Store status selected above"),
        crate::PlanAction::MaintainRoot { .. } => crate::maintenance::observations(&plan, false)
            .iter()
            .chain(crate::maintenance::observations(&plan, true))
            .map(|(step, _)| *step)
            .chain(["stop", "stopped", "install", "start"])
            .collect(),
        crate::PlanAction::CreateChild(_) | crate::PlanAction::UpgradeChild { .. } => vec!["stage"],
        crate::PlanAction::Create { .. } => {
            vec!["create", "install"]
        }
        crate::PlanAction::Upgrade(_) => vec!["stop", "stopped", "install", "start"],
        crate::PlanAction::CreateStore { .. } => vec!["create", "install", "handover"],
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
    let receipt = retained_receipt(dir, &plan, bytes)?;
    if plan.store_root().is_some() {
        for index in 0..crate::store_bootstrap::OBSERVATIONS {
            let path = dir.join(format!("store-observe/{index}"));
            if path.try_exists().map_err(supplier)? {
                effects.insert(
                    format!("store-observe/{index}"),
                    attempt_state(&path)?.into(),
                );
            }
        }
    }
    let evidence = match &plan.action {
        crate::PlanAction::UpgradeChild { .. } => {
            crate::managed::upgrade::evidence_with_source(dir, &plan)?
                .map(|(status, delivery)| (ManagementStatus::Upgrade(status), delivery))
        }
        crate::PlanAction::CreateChild(_) => crate::managed::evidence_with_source(dir, &plan)?
            .map(|(status, delivery)| (ManagementStatus::Creation(status), delivery)),
        _ => None,
    };
    let (management, delivery) =
        evidence.map_or((None, None), |(status, delivery)| (Some(status), delivery));
    let detail = if plan
        .management()
        .is_some_and(|owner| owner.source.is_some())
    {
        StatusDetail::StoredChild {
            delivery: delivery.map(Box::new),
            maintenance: crate::managed::stored::maintenance_evidence(dir, &plan)?.map(Box::new),
        }
    } else {
        StatusDetail::Deployment {}
    };
    Ok(CanisterStatus {
        operation: digest.into(),
        plan,
        effects,
        receipt,
        management,
        detail,
    })
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

/// Read completion through the effect owner, including Store handover/readiness evidence.
pub(crate) fn retained_receipt(dir: &Path, plan: &Plan, bytes: &[u8]) -> Result<Option<Receipt>> {
    if plan.store_upgrade_root().is_some() {
        return crate::store_upgrade::receipt(dir, plan)
            .map(|value| value.map(|(receipt, _)| receipt));
    }
    let digest = plan.digest()?;
    let receipt: Option<Receipt> = if dir.join("receipt.json").try_exists().map_err(supplier)? {
        Some(store::json(&dir.join("receipt.json"))?)
    } else {
        None
    };
    if plan.store_root().is_some()
        && let Some(receipt) = &receipt
    {
        let canister = created(
            plan,
            &store::read(&dir.join("create/reply.bin"), 2 * 1024 * 1024)?,
        )?;
        if receipt.canister != canister.to_text()
            || receipt.operation != digest
            || receipt.module_hash != plan.artifact.sha256
            || receipt.controller != plan.application_controller()
        {
            return Err(Error::Invalid(
                "Store receipt differs from approved creation".into(),
            ));
        }
        store_install_evidence(dir, plan, bytes, canister)?;
        signed::validate_retained(
            &dir.join("handover"),
            plan,
            &crate::store_bootstrap::handover(plan, canister)?,
        )?;
        crate::store_bootstrap::retained(dir, plan, canister)?;
    }
    Ok(receipt)
}

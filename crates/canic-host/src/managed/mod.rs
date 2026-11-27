//! Root admission and authenticated child evidence. No host-owned child effects.

#[cfg(test)]
mod tests;

mod application;

pub(crate) mod recovery;
pub(crate) mod stored;
pub(crate) mod upgrade;
mod wire;

use crate::{
    ApplyOutcome, DeploymentName, Error, ManagedOwner, Plan, PlanInput, Receipt, Result,
    SelectedFleet,
    network::principal,
    operation, plan,
    signed::{self, Call},
    store, supplier,
};
pub use application::activate;
use canic_management_protocol::children::ChildId;
use canic_management_protocol::{Approval, LIMITS, Phase, Refusal, Status};
use ic_platform::native::ReadClient;
use std::path::Path;
use wire::Method;
pub(crate) use wire::observation;

pub(crate) const OBSERVATIONS: usize = 32;

/// Retain Root-owned creation or an upgrade for a declared parent/child pair.
/// The parent must be bound or have a completed named deployment in this journal.
/// `execute_at_ns` selects a new plan's schedule (default: ready once approved);
/// reopening preserves the original schedule.
/// `release` selects a completed publication operation in this journal. Its exact
/// bytes, app/fleet and owners must match.
/// Fleet-owned upgrades resolve original and later child admissions through the
/// same retained history as whole-fleet planning, including completed maintenance.
///
/// # Errors
/// Refuses ambiguous/occupied slots, missing parent custody, invalid budgets or artifacts.
#[expect(
    clippy::too_many_arguments,
    reason = "named planning adds an explicit publication selector without introducing a second planning API"
)]
pub fn create_managed_plan(
    journal: &Path,
    wasm: &Path,
    input: PlanInput,
    selection: &SelectedFleet,
    canister: &str,
    root_module: &str,
    execute_at_ns: Option<u64>,
    release: &str,
    arguments: Option<&Path>,
    blob_attempt: Option<u64>,
) -> Result<(String, Plan)> {
    let arguments = arguments
        .map(|path| {
            crate::store::read(
                path,
                canic_management_protocol::application::MAX_ARGUMENT_BYTES,
            )
        })
        .transpose()?;
    selection.validate()?;
    let role_service = input.target.is_none()
        && selection
            .config
            .policy
            .role_receivers
            .iter()
            .any(|n| n == canister);
    if role_service && !selection.config.receiver_selected(canister) {
        return Err(Error::Invalid(
            "plan the whole fleet to approve a receiver installation with its Root signing key"
                .into(),
        ));
    }
    let child = selection
        .config
        .canisters
        .get(canister)
        .ok_or_else(|| Error::Invalid("unknown child slot".into()))?;
    let parent = child
        .parent
        .as_ref()
        .ok_or_else(|| Error::Invalid("Root-owned work requires a declared parent".into()))?;
    let owner = &selection.config.canisters[parent];
    if child.role.as_deref() == Some("wasm-store") {
        return Err(Error::Invalid(
            "Store bootstrap is not qualified through the application-child contract".into(),
        ));
    }
    let children: Vec<_> = selection
        .config
        .canisters
        .iter()
        .filter(|(_, slot)| {
            slot.parent.as_ref() == Some(parent) && slot.role.as_deref() != Some("wasm-store")
        })
        .map(|(name, _)| name.as_str())
        .collect();
    if input.network != selection.config.network
        || (input.target.is_none() && (child.id.is_some() || input.previous_module.is_some()))
        || owner.role.as_deref() != Some("root")
        || owner.parent.is_some()
    {
        return Err(Error::Invalid(
            "select one child of a top-level Root on this fleet's network; creation requires an unbound slot".into(),
        ));
    }
    let inventory = crate::fleet::inventory(journal, true, crate::now()?)?;
    if !inventory.issues.is_empty() {
        return Err(Error::Invalid(
            "repair unreadable journal evidence before resolving Root".into(),
        ));
    }
    let mut roots = std::collections::BTreeSet::new();
    let mut creation = None;
    if let Some(id) = &owner.id {
        roots.insert(id.clone());
    }
    for (record, network) in &inventory.records {
        if input.target.is_some()
            && record.management.is_some()
            && record.deployment.as_ref().is_some_and(|name| {
                name.app == selection.app
                    && name.fleet == selection.fleet
                    && name.canister == canister
            })
        {
            if network != &input.network
                || !record.receipt_retained
                || record.canister != input.target
                || child
                    .id
                    .as_ref()
                    .is_some_and(|id| Some(id) != input.target.as_ref())
            {
                return Err(Error::Invalid(
                    "upgrade needs this child's completed creation receipt and exact identity"
                        .into(),
                ));
            }
            if creation
                .replace((record.approved_controller.clone(), record.slot))
                .is_some()
            {
                return Err(Error::Invalid("ambiguous child creation evidence".into()));
            }
        }
        if record.deployment.as_ref().is_some_and(|name| {
            name.app == selection.app && name.fleet == selection.fleet && name.canister == *parent
        }) {
            if network != &input.network || !record.receipt_retained {
                return Err(Error::Blocked(
                    "parent deployment is incomplete or on another network".into(),
                ));
            }
            roots.insert(
                record
                    .canister
                    .clone()
                    .ok_or_else(|| Error::Invalid("parent receipt has no canister".into()))?,
            );
        }
    }
    if roots.len() != 1 {
        return Err(Error::Invalid(
            "parent needs one unambiguous configured or retained deployment identity".into(),
        ));
    }
    if input.target.is_some() && creation.as_ref().map(|(root, _)| root) != roots.first() {
        return Err(Error::Invalid(
            "child upgrade requires retained creation custody under this Root".into(),
        ));
    }
    let root = roots
        .pop_first()
        .ok_or_else(|| Error::Invalid("missing Root identity".into()))?;
    let slot = if input.target.is_none() {
        let reservations = inventory
            .records
            .iter()
            .filter(|(record, network)| {
                record.management.is_some()
                    && network == &input.network
                    && record.approved_controller == root
            })
            .map(|(record, _)| (record.slot, record.deployment.as_ref()));
        let deployment = DeploymentName {
            app: selection.app.clone(),
            fleet: selection.fleet.clone(),
            canister: canister.into(),
        };
        permanent_slot(&children, &deployment, reservations)?
    } else {
        creation
            .and_then(|(_, slot)| slot)
            .ok_or_else(|| Error::Invalid("missing child slot".into()))?
    };
    let arguments = if input.target.is_none() && selection.config.receiver_selected(canister) {
        if selection
            .config
            .policy
            .receiver_policy
            .as_ref()
            .is_none_or(|p| &p.root != parent)
        {
            return Err(Error::Invalid(
                "managed receiver must select the planned Root".into(),
            ));
        }
        if role_service {
            let pin = crate::application::issuer_pin(
                journal,
                selection,
                parent,
                &root,
                &input.controller,
            )?;
            Some(selection.config.db_service_install_args(
                canister,
                &root,
                &arguments.unwrap_or_default(),
                pin,
                &input.controller,
            )?)
        } else {
            let application = selection.config.bind_hub_operator(
                canister,
                &input.controller,
                arguments.unwrap_or_default(),
            )?;
            Some(
                selection
                    .config
                    .receiver_install_args(canister, &root, application)?,
            )
        }
    } else if input.target.is_none()
        && (selection.config.route_operator_role(canister).is_some()
            || selection.config.funding_operator_role(canister).is_some())
    {
        Some(selection.config.bind_hub_operator(
            canister,
            &input.controller,
            arguments.unwrap_or_default(),
        )?)
    } else {
        arguments
    };
    if release.is_empty() {
        return Err(Error::Invalid(
            "--release requires a completed Wasm Store publication".into(),
        ));
    }
    let source = stored::select(
        journal,
        release,
        &input,
        selection,
        canister,
        &root,
        root_module,
    )?;
    let now = crate::now()?;
    let execute_at_ns = execute_at_ns.unwrap_or(now);
    let fleet = &selection.config;
    let pool_child_evidence = !fleet.policy.shard_pools.is_empty()
        || fleet.policy.registry.is_some()
        || !fleet.policy.worker_pools.is_empty()
        || fleet.standby_pool.is_some();
    let (plan, bytes) = plan::prepare(
        wasm,
        input,
        Some(DeploymentName {
            app: selection.app.clone(),
            fleet: selection.fleet.clone(),
            canister: canister.into(),
        }),
        Some(ManagedOwner {
            child_evidence_only: pool_child_evidence,
            arguments,
            blob_attempt,
            slot,
            root,
            source,
            root_module: root_module.into(),
            execute_at_ns,
        }),
        now,
    )?;
    plan::retain_locked(&store::Store::open(journal)?, plan, &bytes)
}

// A newly declared service may sort before an already approved one. Only unclaimed
// slots and names participate in config ordering, including before receipts arrive.
fn permanent_slot<'a>(
    children: &[&str],
    deployment: &DeploymentName,
    reservations: impl Iterator<Item = (Option<ChildId>, Option<&'a DeploymentName>)>,
) -> Result<ChildId> {
    let mut occupied = std::collections::BTreeSet::new();
    let mut retained = std::collections::BTreeSet::new();
    let mut selected = None;
    for (slot, name) in reservations {
        let name = name
            .filter(|name| name.app == deployment.app && name.fleet == deployment.fleet)
            .ok_or_else(|| {
                Error::Blocked(
                    "Root child custody belongs to another fleet or an unnamed approval".into(),
                )
            })?;
        let slot =
            slot.ok_or_else(|| Error::Invalid("missing child slot in reservation".into()))?;
        if name == deployment {
            selected = Some(slot);
        }
        if !occupied.insert(slot) {
            return Err(Error::Blocked(
                "multiple creation approvals claim one Root slot".into(),
            ));
        }
        if !retained.insert(name.canister.as_str()) {
            return Err(Error::Blocked(
                "multiple Root slots claim the same service".into(),
            ));
        }
    }
    let mut available = (0..)
        .map_while(ChildId::service)
        .filter(|s| !occupied.contains(s));
    for &name in children.iter().filter(|name| !retained.contains(**name)) {
        let slot = available.next().ok_or_else(|| {
            Error::Invalid(format!("permanent service identity exhausted for {name}"))
        })?;
        if name == deployment.canister {
            selected = Some(slot);
        }
    }
    selected.ok_or_else(|| Error::Invalid("child missing from permanent slots".into()))
}

pub(crate) fn approval(plan: &Plan) -> Result<Approval> {
    let crate::PlanAction::CreateChild(owner) = &plan.action else {
        return Err(Error::Invalid(
            "expected Root-owned creation approval".into(),
        ));
    };
    Ok(Approval {
        manager: owner.root.clone(),
        module_hash: crate::unhex(&plan.artifact.sha256)?
            .try_into()
            .map_err(|_| Error::Invalid("invalid child module hash".into()))?,
        creation_cycles: plan.creation_cycles(),
        execute_at_ns: owner.execute_at_ns,
        expires_at_ns: plan.expires_at_ns,
    })
}

fn validate(plan: &Plan, previous: Option<&Status>, status: &Status) -> Result<()> {
    if status.approval.as_ref() != Some(&approval(plan)?)
        || status.phase == Phase::Empty
        || status.attempts.iter().any(|count| *count > 1)
        || (status.worker.is_some()
            && status.attempts[0] == 0
            && (status.requests[0].is_some()
                || status.creation_refund.is_some()
                || matches!(status.phase, Phase::Scheduled | Phase::CreateIssued)))
        || status
            .observed_module
            .as_ref()
            .is_some_and(|hash| crate::hex(hash) != plan.artifact.sha256)
        || (status.phase == Phase::Ready
            && (status.worker.is_none()
                || status.observed_module.is_none()
                || status.attempts[1..] != [1; 2]
                || status.failure.is_some()))
    {
        return Err(Error::Invalid(
            "Root evidence does not match the approved child operation".into(),
        ));
    }
    if let Some(child) = &status.worker {
        let child = principal(child)?;
        if child == principal(plan.application_controller())?
            || child == principal("aaaaa-aa")?
            || child == principal("2vxsx-fae")?
        {
            return Err(Error::Invalid("invalid returned child principal".into()));
        }
    }
    if let Some(old) = previous
        && ((old.worker.is_some() && old.worker != status.worker)
            || (old.phase as u8 > status.phase as u8)
            || (matches!(old.phase, Phase::Expired | Phase::ReserveBlocked)
                && old.phase != status.phase)
            || old
                .attempts
                .iter()
                .zip(status.attempts)
                .any(|(old, new)| *old > new)
            || old
                .requests
                .iter()
                .zip(status.requests)
                .any(|(old, new)| old.is_some() && *old != new)
            || (old.worker.is_some() && old.attempts[0] != status.attempts[0])
            || (old.creation_refund.is_some() && old.creation_refund != status.creation_refund)
            || (old.observed_module.is_some() && old.observed_module != status.observed_module)
            || (old.phase == Phase::Ready && status.phase != Phase::Ready))
    {
        return Err(Error::Invalid(
            "Root evidence regressed or replaced retained outcome custody".into(),
        ));
    }
    Ok(())
}

fn stage_reply(plan: &Plan, bytes: &[u8]) -> Result<Status> {
    let reply: std::result::Result<Status, Refusal> = ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: Method::Stage.id(plan),
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .map_err(supplier)?;
    reply.map_err(|refusal| {
        Error::Blocked(format!(
            "Root refused admission: {refusal:?}; original attempt retained"
        ))
    })
}

fn stage_evidence<T>(dir: &Path, decode: impl FnOnce(&[u8]) -> Result<T>) -> Result<Option<T>> {
    let path = dir.join("stage/reply.bin");
    if !path.try_exists().map_err(supplier)? {
        return Ok(None);
    }
    match decode(&store::read(&path, LIMITS.bytes)?) {
        Ok(value) => Ok(Some(value)),
        Err(Error::Blocked(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

fn visit_observations(
    dir: &Path,
    [reconcile, observe]: [Method; 2],
    mut visit: impl FnMut(Method, &[u8]) -> Result<()>,
) -> Result<()> {
    for index in 0..OBSERVATIONS {
        let base = dir.join(format!("observe/{index}"));
        for (name, method) in [("reconcile/reply.bin", reconcile), ("reply.bin", observe)] {
            let path = base.join(name);
            if path.try_exists().map_err(supplier)? {
                visit(
                    method,
                    &store::read(&path, canic_management_protocol::application::LIMITS.bytes)?,
                )?;
            }
        }
    }
    Ok(())
}

/// Read only authenticated retained replies; public queries never establish output custody.
pub(crate) fn evidence(dir: &Path, plan: &Plan) -> Result<Option<Status>> {
    evidence_with_source(dir, plan).map(|value| value.map(|(status, _)| status))
}

type Evidence<S> = (S, Option<canic_management_protocol::stored::Delivery>);

fn fold_evidence<S>(
    dir: &Path,
    methods: [Method; 2],
    staged: Option<S>,
    validate: impl Fn(Option<&S>, &S) -> Result<()>,
    empty: impl Fn(&S) -> bool,
    decode: impl Fn(Method, &[u8]) -> Result<Evidence<S>>,
) -> Result<Option<Evidence<S>>> {
    let mut latest = if let Some(status) = staged {
        validate(None, &status)?;
        Some((status, None))
    } else {
        None
    };
    visit_observations(dir, methods, |method, bytes| {
        let (status, source) = decode(method, bytes)?;
        if latest.is_none() && empty(&status) {
            return Ok(());
        }
        validate(latest.as_ref().map(|(previous, _)| previous), &status)?;
        let source =
            stored::retain_delivery(latest.as_ref().and_then(|(_, old)| old.as_ref()), source)?;
        latest = Some((status, source));
        Ok(())
    })?;
    Ok(latest)
}

pub(crate) fn evidence_with_source(
    dir: &Path,
    plan: &Plan,
) -> Result<Option<(Status, Option<canic_management_protocol::stored::Delivery>)>> {
    fold_evidence(
        dir,
        [Method::Reconcile, Method::Observe],
        stage_evidence(dir, |bytes| stage_reply(plan, bytes))?,
        |old, new| validate(plan, old, new),
        |status| status.phase == Phase::Empty && status.approval.is_none(),
        |method, bytes| {
            let method = method.id(plan);
            if matches!(
                method,
                canic_management_protocol::application::CHILD_EVIDENCE
                    | canic_management_protocol::application::CUSTODY
            ) {
                stored::creation_evidence(plan, bytes)
            } else {
                Ok((decode_observation(method, bytes)?, None))
            }
        },
    )
}

struct Admission<'a> {
    dir: &'a Path,
    plan: &'a Plan,
    identity: &'a str,
    icp: &'a Path,
    client: &'a ReadClient,
}

impl Admission<'_> {
    async fn stage<S>(
        &self,
        retained: bool,
        before_signing: impl Future<Output = Result<()>>,
        call: impl FnOnce() -> Result<Call<'static>>,
        staged: impl FnOnce(&[u8]) -> Result<S>,
        observed: impl FnOnce(&[u8]) -> Result<S>,
        empty: impl FnOnce(&S) -> bool,
    ) -> Result<S> {
        let Self {
            dir,
            plan,
            identity,
            icp,
            client,
        } = *self;
        stored::verify(plan, client).await?;
        let stage = dir.join("stage");
        if stage.join("signed.json").try_exists().map_err(supplier)? {
            let path = recovery::next_observation(dir)?;
            let read = observation(plan)?;
            recovery::require_settled(&path)?;
            let bytes = signed::observe(&path, plan, &read, identity, icp, client).await?;
            let status = observed(&bytes)?;
            if retained || !empty(&status) {
                return Ok(status);
            }
            // An empty observation cannot replace the original signed attempt.
        } else {
            let owner = plan.management().expect("managed child");
            if crate::now()?
                >= plan::admission_deadline(
                    plan.created_at_ns,
                    owner.execute_at_ns,
                    plan.expires_at_ns,
                )
            {
                let reason = if plan.target().is_some() {
                    "Root upgrade admission time passed; no new attempt signed"
                } else {
                    "Root admission time passed; no new stage attempt was signed"
                };
                return Err(Error::Blocked(reason.into()));
            }
            before_signing.await?;
        }
        staged(&signed::execute(&stage, plan, &call()?, identity, icp, client).await?)
    }
}

pub(crate) async fn apply(
    dir: &Path,
    id: &str,
    plan: &Plan,
    identity: &str,
    icp: &Path,
    client: &ReadClient,
) -> Result<ApplyOutcome> {
    let owner = plan
        .management()
        .ok_or_else(|| Error::Invalid("expected Root-owned child operation".into()))?;
    let root = principal(&owner.root)?;
    operation::controllers(client, root, &plan.controller).await?;
    if operation::module(client, root).await? != owner.root_module {
        let reason = if matches!(plan.action, crate::PlanAction::UpgradeChild { .. }) {
            "Root code differs from reviewed upgrade owner"
        } else {
            "Root code differs from the reviewed management owner"
        };
        return Err(Error::Blocked(reason.into()));
    }
    let admission = Admission {
        dir,
        plan,
        identity,
        icp,
        client,
    };
    if matches!(plan.action, crate::PlanAction::UpgradeChild { .. }) {
        return upgrade::apply(&admission, id).await;
    }
    let retained = evidence(dir, plan)?;
    if let Some(status) = &retained
        && status.phase == Phase::Ready
    {
        return finish(dir, id, plan, status, client).await;
    }
    let status = admission
        .stage(
            retained.is_some(),
            async { Ok(()) },
            || Method::Stage.call(plan),
            |bytes| stage_reply(plan, bytes),
            |bytes| stored::creation(plan, bytes),
            |status| status.phase == Phase::Empty && status.approval.is_none(),
        )
        .await?;
    validate(plan, retained.as_ref(), &status)?;
    if status.phase == Phase::Ready {
        finish(dir, id, plan, &status, client).await
    } else {
        Ok(ApplyOutcome::Pending {
            operation: id.into(),
            root: owner.root.clone(),
            status: Box::new(status),
        })
    }
}

async fn finish(
    dir: &Path,
    id: &str,
    plan: &Plan,
    status: &Status,
    client: &ReadClient,
) -> Result<ApplyOutcome> {
    let child = status
        .worker
        .as_ref()
        .ok_or_else(|| Error::Invalid("ready evidence has no child".into()))?;
    complete(dir, id, plan, child, client).await
}

async fn complete(
    dir: &Path,
    id: &str,
    plan: &Plan,
    child: &str,
    client: &ReadClient,
) -> Result<ApplyOutcome> {
    let receipt = Receipt {
        operation: id.into(),
        canister: child.into(),
        module_hash: plan.artifact.sha256.clone(),
        controller: plan.application_controller().into(),
    };
    operation::verify_receipt(client, plan, id, &receipt).await?;
    if dir.join("receipt.json").try_exists().map_err(supplier)? {
        if store::json::<Receipt>(&dir.join("receipt.json"))? != receipt {
            return Err(Error::Invalid(
                "receipt substituted retained child evidence".into(),
            ));
        }
    } else {
        store::put_json(&dir.join("receipt.json"), &receipt)?;
    }
    Ok(ApplyOutcome::Complete(receipt))
}

fn decode_observation(method: u64, bytes: &[u8]) -> Result<Status> {
    ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method,
            direction: ic_wire::Direction::Response,
        },
        LIMITS,
    )
    .map_err(supplier)
}

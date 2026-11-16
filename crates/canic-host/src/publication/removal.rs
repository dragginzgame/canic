//! Removal reuses the plan lock and exact signed-attempt owner. No Store catalog copy.
use super::{Publication, manifest};
use crate::{
    Error, Plan, PlanAction, Result, SelectedFleet, operation, plan, signed, store, supplier,
};
use canic_management_protocol::publication as wire;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Exact release authority inherited from one independently retained publication.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationRemoval {
    /// Source publication approval; never a deployment receipt or a live catalog alias.
    pub publication_operation: String,
    /// Reviewed Root, Store and immutable release label.
    pub publication: Publication,
    /// Delete published content or cancel an incomplete upload.
    pub action: wire::Removal,
}

/// Successful response from the original removal call, not an absence observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemovalReceipt {
    /// Exact removal approval digest.
    pub operation: String,
    /// Original dispatch deadline; completion does not renew or erase it.
    pub dispatch_deadline_ns: u64,
    /// Exact immutable content returned by Store through Root.
    pub manifest: ic_wasm_store_protocol::Manifest,
}

pub(crate) fn validate(plan: &Plan, value: &PublicationRemoval) -> Result<()> {
    super::validate(plan, &value.publication)?;
    if value.publication_operation.len() != 64
        || crate::unhex(&value.publication_operation)?.len() != 32
        || value.publication_operation != value.publication_operation.to_ascii_lowercase()
    {
        return Err(Error::Invalid(
            "removal requires an exact publication operation".into(),
        ));
    }
    Ok(())
}

fn binding(plan: &Plan) -> Result<&PublicationRemoval> {
    plan.removal()
        .ok_or_else(|| Error::Invalid("removal plan required".into()))
}
fn expected(plan: &Plan, bytes: &[u8]) -> Result<ic_wasm_store_protocol::Manifest> {
    Ok(super::release_manifest(
        binding(plan)?.publication.release.clone(),
        bytes,
    ))
}
fn same_release(left: &Plan, right: &Plan, publication: &Publication) -> bool {
    left.network == right.network
        && left.removal().is_some_and(|r| {
            r.publication.store.canister == publication.store.canister
                && r.publication.release == publication.release
        })
}

// Only top-level operation directories are removal owners. Nested fleet publications
// are still fenced by network/Store/release, not by application names or directory shape.
fn plans(journal: &Path) -> Result<Vec<(String, Plan)>> {
    let (ids, issues) = crate::fleet::operation_ids(journal, true)?;
    if !issues.is_empty() {
        return Err(Error::Blocked(
            "repair unreadable journal evidence before changing releases".into(),
        ));
    }
    ids.into_iter()
        .filter_map(|id| {
            let dir = journal.join(&id);
            match crate::fleet_plan::exists(&dir) {
                Ok(true) => None,
                Ok(false) => Some((|| {
                    let plan: Plan = store::json(&dir.join("plan.json"))?;
                    if plan.digest()? != id {
                        return Err(Error::Invalid("retained plan digest mismatch".into()));
                    }
                    Ok((id, plan))
                })()),
                Err(error) => Some(Err(error)),
            }
        })
        .collect()
}

/// Every removal of this release must be an explicitly settled predecessor.
/// Returns publication ancestors so retained planning cannot reuse their old receipts.
pub(crate) fn publication_predecessors(journal: &Path, plan: &Plan) -> Result<Vec<String>> {
    let publication = super::binding(plan)?;
    let removals: std::collections::BTreeMap<_, _> = plans(journal)?
        .into_iter()
        .filter(|(_, old)| same_release(old, plan, publication))
        .collect();
    let mut next = publication.after_removal.as_ref();
    let mut seen = std::collections::BTreeSet::new();
    let mut predecessors = Vec::new();
    while let Some(id) = next {
        if !seen.insert(id.clone()) {
            return Err(Error::Invalid("cyclic restoration authority".into()));
        }
        let removal = removals.get(id).ok_or_else(|| {
            Error::Invalid("restoration removal is absent or belongs to another release".into())
        })?;
        let (loaded, bytes) = plan::load(&journal.join(id), id)?;
        let retained = receipt(&journal.join(id), &loaded, &bytes)?.ok_or_else(|| {
            Error::Blocked(
                "removal has no successful retained reply; restoration remains blocked".into(),
            )
        })?;
        if crate::now()? <= retained.dispatch_deadline_ns {
            return Err(Error::Blocked(
                "original removal dispatch deadline has not expired; restoration remains blocked"
                    .into(),
            ));
        }
        if loaded.controller != plan.controller
            || loaded.artifact != plan.artifact
            || loaded.deployment != plan.deployment
            || binding(&loaded)?.publication.root != publication.root
            || binding(&loaded)?.publication.store != publication.store
        {
            return Err(Error::Invalid(
                "restoration differs from retained content or authority".into(),
            ));
        }
        let value = binding(removal)?;
        let (source, _) = plan::load(
            &journal.join(&value.publication_operation),
            &value.publication_operation,
        )?;
        if source.publication() != Some(&value.publication)
            || source.network != plan.network
            || source.controller != plan.controller
            || source.artifact != plan.artifact
            || source.deployment != plan.deployment
        {
            return Err(Error::Invalid(
                "removal differs from its source publication".into(),
            ));
        }
        predecessors.push(value.publication_operation.clone());
        next = value.publication.after_removal.as_ref();
    }
    if seen.len() != removals.len() {
        return Err(Error::Blocked("release has a removal outside this approval's settled history; use an explicit restoration plan".into()));
    }
    Ok(predecessors)
}

pub(crate) fn permit_publication(journal: &Path, plan: &Plan) -> Result<()> {
    publication_predecessors(journal, plan).map(|_| ())
}

/// Plan restoration of the same immutable bytes after settled, expired removal.
/// # Errors
/// Refuses missing/uncertain replies, unexpired dispatch, altered authority or scope.
pub fn create_restoration_plan(
    journal: &Path,
    removal_operation: &str,
    selection: &SelectedFleet,
) -> Result<(String, Plan)> {
    selection.validate()?;
    let held = store::Store::open(journal)?;
    let (old, bytes) = plan::load(&held.operation(removal_operation)?, removal_operation)?;
    if !super::list::selected(&old, selection) {
        return Err(Error::Invalid(
            "removal does not belong to the selected fleet".into(),
        ));
    }
    let mut publication = binding(&old)?.publication.clone();
    publication.after_removal = Some(removal_operation.into());
    let now = crate::now()?;
    let proposed = Plan {
        version: 3,
        action: PlanAction::Publish(publication),
        created_at_ns: now,
        expires_at_ns: now + 3_600_000_000_000,
        ..old
    };
    plan::retain_locked(&held, proposed, &bytes)
}

/// Retain a removal plan without signing or sending a mutation.
///
/// The selected source must be an independent publication in this journal. All
/// known matching publication calls must have settled and their dispatch windows
/// expired. Replanning returns the original removal and never renews its authority.
/// # Errors
/// Refuses changed scope, unsettled publication, competing removal or unreadable evidence.
pub fn create_removal_plan(
    journal: &Path,
    publication_operation: &str,
    selection: &SelectedFleet,
    action: wire::Removal,
) -> Result<(String, Plan)> {
    selection.validate()?;
    let held = store::Store::open(journal)?;
    let source = held.operation(publication_operation)?;
    let (publication, bytes) = plan::load(&source, publication_operation)?;
    let binding = super::binding(&publication)?.clone();
    if !super::list::selected(&publication, selection) {
        return Err(Error::Invalid(
            "publication does not belong to the selected fleet".into(),
        ));
    }
    let value = PublicationRemoval {
        publication_operation: publication_operation.into(),
        publication: binding,
        action,
    };
    for (id, old) in plans(journal)? {
        if same_release(&old, &publication, &value.publication)
            && old
                .removal()
                .is_some_and(|r| r.publication_operation == publication_operation)
        {
            if old.removal() == Some(&value)
                && old.artifact == publication.artifact
                && old.controller == publication.controller
                && old.deployment == publication.deployment
            {
                return Ok((id, old));
            }
            return Err(Error::Blocked(format!(
                "release already has removal approval {id}; no replacement authority"
            )));
        }
    }
    permit_publication(journal, &publication)?;
    // Include materialized publications inside whole-fleet operations. No observer
    // receipt can settle an earlier ambiguous Prepare/Put/Publish invocation.
    let (ids, _) = crate::fleet::operation_ids(journal, true)?;
    for id in ids {
        let dir = journal.join(&id);
        if crate::fleet_plan::exists(&dir)? {
            crate::fleet_plan::quiesce_publications(&dir, &id, &publication)?;
        } else {
            let (other, bytes) = plan::load(&dir, &id)?;
            quiesce_matching(&dir, &other, &bytes, &publication)?;
        }
    }
    let now = crate::now()?;
    let removal = Plan {
        version: 4,
        action: PlanAction::Remove(value),
        created_at_ns: now,
        expires_at_ns: now + 3_600_000_000_000,
        ..publication
    };
    plan::publish_locked(&held, removal, &bytes)
}

pub(crate) fn quiesce_matching(
    dir: &Path,
    plan: &Plan,
    bytes: &[u8],
    selected: &Plan,
) -> Result<()> {
    let Some(publication) = plan.publication() else {
        return Ok(());
    };
    let expected_binding = super::binding(selected)?;
    if plan.network != selected.network
        || publication.store.canister != expected_binding.store.canister
        || publication.release != expected_binding.release
    {
        return Ok(());
    }
    let expected = manifest(plan, bytes)?;
    for (index, step) in super::steps(&expected).iter().enumerate() {
        let path = dir.join("publication").join(step);
        if !path.try_exists().map_err(supplier)? {
            continue;
        }
        let deadline: u64 = store::json(&path.join("dispatch-deadline.json"))?;
        if deadline > plan.expires_at_ns || crate::now()? <= deadline {
            return Err(Error::Blocked(
                "publication dispatch window has not expired; removal remains blocked".into(),
            ));
        }
        if !path.join("signed.json").try_exists().map_err(supplier)? {
            continue;
        }
        signed::validate_retained(
            &path,
            plan,
            &super::call(plan, super::command(&expected, bytes, index), deadline)?,
        )?;
        let reply = store::read(&path.join("reply.bin"), 2 * 1024 * 1024)?;
        if super::decode(&reply, &expected)?.is_none() {
            return Err(Error::Blocked(
                "publication call has no settled matching result; removal remains blocked".into(),
            ));
        }
    }
    Ok(())
}

fn call(plan: &Plan, bytes: &[u8], deadline: u64) -> Result<signed::Call<'static>> {
    let value = binding(plan)?;
    let root = crate::network::principal(&value.publication.root.canister)?;
    Ok(signed::Call {
        canister: root,
        effective: root,
        method: "store_removal",
        args: ic_wire::encode(
            &wire::RemovalRequest {
                store: value.publication.store.canister.clone(),
                expires_at_ns: deadline,
                action: value.action,
                manifest: expected(plan, bytes)?,
            },
            ic_wire::Encoding::Frame {
                method: wire::REMOVE,
                direction: ic_wire::Direction::Request,
            },
            wire::REMOVAL_LIMITS,
        )
        .map_err(supplier)?,
    })
}
fn decode(reply: &[u8], expected: &ic_wasm_store_protocol::Manifest) -> Result<()> {
    let reply: wire::RemovalReply = ic_wire::decode(
        reply,
        ic_wire::Encoding::Frame {
            method: wire::REMOVE,
            direction: ic_wire::Direction::Response,
        },
        wire::RESPONSE_LIMITS,
    )
    .map_err(supplier)?;
    match reply {
        Ok(value) if value == *expected => Ok(()),
        Ok(_) => Err(Error::Invalid(
            "removal result differs from approved manifest".into(),
        )),
        Err(error) => Err(Error::Blocked(format!(
            "removal returned {error:?}; original evidence retained, no replacement attempt"
        ))),
    }
}

pub(crate) fn receipt(dir: &Path, plan: &Plan, bytes: &[u8]) -> Result<Option<RemovalReceipt>> {
    let path = dir.join("removal");
    if !path.join("reply.bin").try_exists().map_err(supplier)? {
        return Ok(None);
    }
    let deadline: u64 = store::json(&path.join("dispatch-deadline.json"))?;
    if deadline > plan.expires_at_ns {
        return Err(Error::Invalid("removal deadline exceeds approval".into()));
    }
    signed::validate_retained(&path, plan, &call(plan, bytes, deadline)?)?;
    let manifest = expected(plan, bytes)?;
    decode(
        &store::read(&path.join("reply.bin"), 2 * 1024 * 1024)?,
        &manifest,
    )?;
    Ok(Some(RemovalReceipt {
        operation: plan.digest()?,
        dispatch_deadline_ns: deadline,
        manifest,
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
    if let Some(removal) = receipt(dir, plan, bytes)? {
        return Ok(crate::ApplyOutcome::Removal { removal });
    }
    let path = dir.join("removal");
    if reconcile && !path.join("signed.json").try_exists().map_err(supplier)? {
        return Err(Error::Blocked(
            "removal reconciliation requires its original signed attempt; use apply first".into(),
        ));
    }
    let agent = plan.network.agent()?;
    super::verify_binding(&agent, plan, &binding(plan)?.publication).await?;
    let deadline = super::step_deadline(&path, plan)?;
    if !path.join("signed.json").try_exists().map_err(supplier)? && crate::now()? >= deadline {
        return Err(Error::Blocked(
            "original removal dispatch window expired; no replacement attempt".into(),
        ));
    }
    signed::execute(
        &path,
        plan,
        &call(plan, bytes, deadline)?,
        identity,
        icp,
        &agent,
    )
    .await?;
    let removal = receipt(dir, plan, bytes)?
        .ok_or_else(|| Error::Blocked("removal has no retained result".into()))?;
    Ok(crate::ApplyOutcome::Removal { removal })
}

pub(crate) fn status(dir: &Path, plan: &Plan, bytes: &[u8]) -> Result<crate::CanisterStatus> {
    Ok(crate::CanisterStatus {
        operation: plan.digest()?,
        plan: plan.clone(),
        effects: std::collections::BTreeMap::from([(
            "removal".into(),
            operation::attempt_state(&dir.join("removal"))?.into(),
        )]),
        receipt: None,
        management: None,
        detail: crate::StatusDetail::Removal {
            removal: receipt(dir, plan, bytes)?,
        },
    })
}

#[cfg(test)]
mod tests;

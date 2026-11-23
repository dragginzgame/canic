//! Publication uses the existing plan journal and signed-attempt owner, not deployment receipts.

#[cfg(test)]
mod tests;

pub(crate) mod list;
pub(crate) mod removal;

use crate::{
    ApplyOutcome, DeploymentName, Error, Plan, PlanAction, Result, SelectedFleet, UpgradeTarget,
    artifact, network::principal, operation, plan, signed, store, supplier,
};
use canic_management_protocol::publication::{self as wire, Command};
use ic_platform::native::ReadClient;
use ic_wasm_store_protocol as protocol;
use serde::{Deserialize, Serialize};
use std::path::Path;

const OBSERVATIONS: usize = 32;

/// Exact canister/code identities and immutable release label; no deployment authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Publication {
    /// Operator-controlled Root and its reviewed executable.
    pub root: UpgradeTarget,
    /// Root-controlled Store and its reviewed executable.
    pub store: UpgradeTarget,
    /// Application canister name and explicit version, never a moving latest alias.
    pub release: protocol::Release,
    /// Settled predecessor removal; omission preserves historical approval bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_removal: Option<String>,
}
/// Named publication inputs, resolved against one selected fleet before retaining a plan.
pub struct PublicationInput {
    /// App-defined artifact canister slot.
    pub canister: String,
    /// Bound sibling slot with role `wasm-store`.
    pub store: String,
    /// Immutable release version.
    pub version: String,
    /// Exact Root module hash.
    pub root_module: String,
    /// Exact Store module hash.
    pub store_module: String,
}
/// Historical publication evidence; never an installed-canister receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationProgress {
    /// Original plan digest.
    pub operation: String,
    /// Admitted Root identity.
    pub root: String,
    /// Admitted Store identity.
    pub store: String,
    /// Retained signed observation establishing this exact progress.
    pub observation: usize,
    /// Retained replicated Store progress, or no prepared release observed yet.
    pub status: Option<protocol::Status>,
}

pub(crate) fn validate(plan: &Plan, publication: &Publication) -> Result<()> {
    if let Some(id) = &publication.after_removal
        && (id.len() != 64 || crate::unhex(id)?.len() != 32 || id != &id.to_ascii_lowercase())
    {
        return Err(Error::Invalid(
            "restoration requires an exact removal operation".into(),
        ));
    }
    for target in [&publication.root, &publication.store] {
        let id = principal(&target.canister)?;
        if id.to_text() != target.canister
            || id == principal("aaaaa-aa")?
            || id == principal("2vxsx-fae")?
            || crate::unhex(&target.module)?.len() != 32
            || target.module != target.module.to_ascii_lowercase()
        {
            return Err(Error::Invalid(
                "publication requires canonical canister IDs and exact module hashes".into(),
            ));
        }
    }
    if publication.root.canister == publication.store.canister
        || !publication.release.valid()
        || plan
            .deployment
            .as_ref()
            .is_none_or(|name| name.canister != publication.release.name)
    {
        return Err(Error::Invalid("invalid publication binding".into()));
    }
    Ok(())
}

/// Retain publication only; Root/Store must already have explicit fleet identity bindings.
/// # Errors
/// Refuses mismatched topology, ambiguous authority, invalid bytes or changed retained releases.
pub fn create_publication_plan(
    journal: &Path,
    wasm: &Path,
    selection: &SelectedFleet,
    controller: &str,
    input: PublicationInput,
) -> Result<(String, Plan)> {
    selection.validate()?;
    let slots = &selection.config.canisters;
    let child = slots
        .get(&input.canister)
        .ok_or_else(|| Error::Invalid("unknown artifact canister".into()))?;
    // Fleet validation has already checked every parent reference and rejected cycles.
    let mut root_name = &input.canister;
    while let Some(parent) = &slots[root_name].parent {
        root_name = parent;
    }
    let store_slot = slots
        .get(&input.store)
        .ok_or_else(|| Error::Invalid("unknown Store slot".into()))?;
    let root = &slots[root_name];
    if child.parent.is_none()
        || root.role.as_deref() != Some("root")
        || input.canister == input.store
        || store_slot.role.as_deref() != Some("wasm-store")
        || store_slot.parent.as_ref() != Some(root_name)
    {
        return Err(Error::Invalid(
            "artifact and wasm-store must share the declared top-level Root".into(),
        ));
    }
    let binding = Publication {
        after_removal: None,
        root: UpgradeTarget {
            canister: root.id.clone().ok_or_else(|| {
                Error::Invalid("bind the existing Root ID before publication".into())
            })?,
            module: input.root_module,
        },
        store: UpgradeTarget {
            canister: store_slot.id.clone().ok_or_else(|| {
                Error::Invalid("bind the existing Store ID before publication".into())
            })?,
            module: input.store_module,
        },
        release: protocol::Release {
            name: input.canister.clone(),
            version: input.version,
        },
    };
    let bytes = store::read(wasm, artifact::MAX_ARTIFACT_BYTES)?;
    let now = crate::now()?;
    let plan = Plan {
        version: 3,
        network: selection.config.network.clone(),
        controller: controller.into(),
        artifact: artifact::admit_bounded(&bytes, artifact::MAX_ARTIFACT_BYTES)?,
        action: PlanAction::Publish(binding),
        created_at_ns: now,
        expires_at_ns: now + 3_600_000_000_000,
        deployment: Some(DeploymentName {
            app: selection.app.clone(),
            fleet: selection.fleet.clone(),
            canister: input.canister,
        }),
    };
    let held = store::Store::open(journal)?;
    plan::retain_locked(&held, plan, &bytes)
}

fn binding(plan: &Plan) -> Result<&Publication> {
    plan.publication()
        .ok_or_else(|| Error::Invalid("publication plan required".into()))
}

pub(crate) fn manifest(plan: &Plan, bytes: &[u8]) -> Result<protocol::Manifest> {
    Ok(release_manifest(binding(plan)?.release.clone(), bytes))
}

pub(crate) fn release_manifest(release: protocol::Release, bytes: &[u8]) -> protocol::Manifest {
    protocol::Manifest {
        release,
        hash: protocol::digest(bytes),
        bytes: bytes.len() as u64,
        chunks: bytes
            .chunks(protocol::CHUNK_BYTES)
            .map(protocol::digest)
            .collect(),
    }
}

pub(crate) fn call(plan: &Plan, command: Command, deadline: u64) -> Result<signed::Call<'static>> {
    let publication = binding(plan)?;
    let root = principal(&publication.root.canister)?;
    Ok(signed::Call {
        canister: root,
        effective: root,
        method: "store_publication",
        args: ic_wire::encode(
            &wire::Request {
                store: publication.store.canister.clone(),
                expires_at_ns: deadline,
                command,
            },
            ic_wire::Encoding::Frame {
                method: wire::EXECUTE,
                direction: ic_wire::Direction::Request,
            },
            wire::REQUEST_LIMITS,
        )
        .map_err(supplier)?,
    })
}

pub(crate) fn observation(plan: &Plan, deadline: u64) -> Result<signed::Call<'static>> {
    call(
        plan,
        Command::Observe(binding(plan)?.release.clone()),
        deadline,
    )
}

fn decode(bytes: &[u8], expected: &protocol::Manifest) -> Result<Option<protocol::Status>> {
    let reply: wire::Reply = ic_wire::decode(
        bytes,
        ic_wire::Encoding::Frame {
            method: wire::EXECUTE,
            direction: ic_wire::Direction::Response,
        },
        wire::RESPONSE_LIMITS,
    )
    .map_err(supplier)?;
    match reply {
        Err(wire::Error::Store(protocol::Error::NotFound)) => Ok(None),
        Err(error) => Err(Error::Blocked(format!(
            "Store publication returned {error:?}; retain the original attempt and reconcile"
        ))),
        Ok(status)
            if status.manifest == *expected
                && status
                    .missing
                    .iter()
                    .all(|i| (*i as usize) < expected.chunks.len())
                && status.missing.windows(2).all(|w| w[0] < w[1])
                && (!status.published || status.missing.is_empty()) =>
        {
            Ok(Some(status))
        }
        Ok(_) => Err(Error::Invalid(
            "Store reply differs from the approved artifact".into(),
        )),
    }
}

fn progress(
    plan: &Plan,
    digest: &str,
    observation: usize,
    status: Option<protocol::Status>,
) -> Result<PublicationProgress> {
    let publication = binding(plan)?;
    Ok(PublicationProgress {
        operation: digest.into(),
        root: publication.root.canister.clone(),
        store: publication.store.canister.clone(),
        observation,
        status,
    })
}

fn steps(manifest: &protocol::Manifest) -> Vec<String> {
    std::iter::once("prepare".into())
        .chain((0..manifest.chunks.len()).map(|i| format!("chunk-{i}")))
        .chain(std::iter::once("publish".into()))
        .collect()
}

fn command(expected: &protocol::Manifest, bytes: &[u8], index: usize) -> Command {
    if index == 0 {
        Command::Prepare(expected.clone())
    } else if index <= expected.chunks.len() {
        Command::Put(protocol::Put {
            release: expected.release.clone(),
            index: (index - 1) as u32,
            bytes: bytes
                .chunks(protocol::CHUNK_BYTES)
                .nth(index - 1)
                .expect("manifest chunk")
                .to_vec(),
        })
    } else {
        Command::Publish(expected.release.clone())
    }
}

fn observed(status: Option<&protocol::Status>, index: usize, chunks: usize) -> bool {
    status.is_some_and(|s| {
        if index == 0 {
            true
        } else if index <= chunks {
            !s.missing.contains(&((index - 1) as u32))
        } else {
            s.published
        }
    })
}

fn step_deadline(dir: &Path, plan: &Plan) -> Result<u64> {
    let directory = crate::output_publication::prepare(dir, 0o700)?;
    let path = dir.join("dispatch-deadline.json");
    let deadline = if path.try_exists().map_err(supplier)? {
        store::json(&path)?
    } else {
        let deadline = (crate::now()? + wire::MAX_WINDOW_NS).min(plan.expires_at_ns);
        store::put_json_in(
            &directory,
            std::ffi::OsStr::new("dispatch-deadline.json"),
            &deadline,
        )?;
        deadline
    };
    if deadline > plan.expires_at_ns {
        return Err(Error::Invalid(
            "publication deadline exceeds approval".into(),
        ));
    }
    Ok(deadline)
}

async fn verify_binding(client: &ReadClient, plan: &Plan, publication: &Publication) -> Result<()> {
    for (target, controller) in [
        (&publication.root, &plan.controller),
        (&publication.store, &publication.root.canister),
    ] {
        let id = principal(&target.canister)?;
        operation::controllers(client, id, controller).await?;
        if operation::module(client, id).await? != target.module {
            return Err(Error::Blocked(
                "publication Root/Store code differs from approval".into(),
            ));
        }
    }
    Ok(())
}

async fn observe(
    dir: &Path,
    plan: &Plan,
    expected: &protocol::Manifest,
    identity: &str,
    icp: &Path,
    client: &ReadClient,
) -> Result<(usize, Option<protocol::Status>)> {
    for index in 0..OBSERVATIONS {
        let path = dir.join(format!("publication/observe/{index}"));
        if path.join("reply.bin").try_exists().map_err(supplier)? {
            continue;
        }
        let reply = signed::observe_operation(&path, plan, identity, icp, client).await?;
        return Ok((index, decode(&reply, expected)?));
    }
    Err(Error::Blocked(
        "publication observation budget exhausted; retained evidence is unchanged".into(),
    ))
}

pub(crate) async fn apply(
    journal: &Path,
    dir: &Path,
    plan: &Plan,
    bytes: &[u8],
    identity: &str,
    icp: &Path,
    read_only: bool,
) -> Result<ApplyOutcome> {
    let digest = plan.digest()?;
    let digest = digest.as_str();
    let client = plan.network.client()?;
    removal::permit_publication(journal, plan)?;
    verify_binding(&client, plan, binding(plan)?).await?;
    let expected = manifest(plan, bytes)?;
    if let Some(receipt) = receipt(dir, plan, digest, &expected)? {
        return Ok(ApplyOutcome::Publication {
            publication: receipt,
        });
    }
    let (mut observation, mut status) =
        observe(dir, plan, &expected, identity, icp, &client).await?;
    let mut changed = false;
    if !read_only {
        for (index, step) in steps(&expected).iter().enumerate() {
            if observed(status.as_ref(), index, expected.chunks.len()) {
                continue;
            }
            let command = command(&expected, bytes, index);
            let path = dir.join("publication").join(step);
            let deadline = step_deadline(&path, plan)?;
            let call = call(plan, command, deadline)?;
            let reply = signed::execute(&path, plan, &call, identity, icp, &client).await?;
            status = decode(&reply, &expected)?;
            changed = true;
            if !observed(status.as_ref(), index, expected.chunks.len()) {
                return Err(Error::Blocked(
                    "publication step lacks its expected retained outcome".into(),
                ));
            }
        }
    }
    if changed {
        (observation, status) = observe(dir, plan, &expected, identity, icp, &client).await?;
    }
    let value = progress(plan, digest, observation, status)?;
    if value.status.as_ref().is_some_and(|s| s.published) {
        store::put_json(&dir.join("publication.json"), &value)?;
    }
    Ok(ApplyOutcome::Publication { publication: value })
}

pub(crate) fn receipt(
    dir: &Path,
    plan: &Plan,
    digest: &str,
    expected: &protocol::Manifest,
) -> Result<Option<PublicationProgress>> {
    if !dir
        .join("publication.json")
        .try_exists()
        .map_err(supplier)?
    {
        return Ok(None);
    }
    let value: PublicationProgress = store::json(&dir.join("publication.json"))?;
    if value.observation >= OBSERVATIONS
        || value != progress(plan, digest, value.observation, value.status.clone())?
        || value
            .status
            .as_ref()
            .is_none_or(|s| s.manifest != *expected || !s.published || !s.missing.is_empty())
    {
        return Err(Error::Invalid(
            "publication receipt differs from approved release".into(),
        ));
    }
    let reply = signed::retained_operation(
        &dir.join(format!("publication/observe/{}", value.observation)),
        plan,
    )?;
    if decode(&reply, expected)? != value.status {
        return Err(Error::Invalid(
            "publication receipt differs from retained observation".into(),
        ));
    }
    Ok(Some(value))
}

/// Reuse historical completion under a newly reviewed Root executable; never renew publication authority.
pub(crate) fn completed_release<'a>(
    dir: &Path,
    old: &Plan,
    proposed: &Plan,
    expected: &protocol::Manifest,
    preserved_store: impl IntoIterator<Item = &'a canic_management_protocol::store_maintenance::Status>,
) -> Result<bool> {
    let previous = binding(old)?;
    let next = binding(proposed)?;
    let mut preserved_module = previous.store.module.clone();
    for status in preserved_store {
        if crate::store_upgrade::ready(status)
            && status.approval.store == previous.store.canister
            && crate::hex(&status.approval.previous_module) == preserved_module
        {
            preserved_module = crate::hex(&status.approval.module_hash);
        }
    }
    if old.network != proposed.network
        || old.controller != proposed.controller
        || old.deployment != proposed.deployment
        || old.artifact != proposed.artifact
        || previous.root.canister != next.root.canister
        || previous.store.canister != next.store.canister
        || preserved_module != next.store.module
        || previous.release != next.release
    {
        return Ok(false);
    }
    Ok(receipt(dir, old, &old.digest()?, expected)?.is_some())
}

pub(crate) fn status(
    dir: &Path,
    digest: &str,
    plan: &Plan,
    bytes: &[u8],
) -> Result<crate::CanisterStatus> {
    let expected = manifest(plan, bytes)?;
    let mut effects = std::collections::BTreeMap::<String, String>::new();
    for step in steps(&expected)
        .into_iter()
        .chain((0..OBSERVATIONS).map(|i| format!("observe/{i}")))
    {
        let path = dir.join("publication").join(&step);
        if step.starts_with("observe/") && !path.try_exists().map_err(supplier)? {
            continue;
        }
        effects.insert(step, operation::attempt_state(&path)?.into());
    }
    let publication = receipt(dir, plan, digest, &expected)?;
    Ok(crate::CanisterStatus {
        operation: digest.into(),
        plan: plan.clone(),
        effects,
        receipt: None,
        management: None,
        detail: crate::StatusDetail::Publication {
            publication: publication.map(Box::new),
        },
    })
}

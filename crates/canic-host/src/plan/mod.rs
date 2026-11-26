mod action;
mod record;
#[cfg(test)]
mod tests;

use crate::{
    Artifact, Error, Network, Result, artifact,
    network::principal,
    store::{self, Store},
    supplier,
};
pub use action::{PlanAction, StoreMaintenance, UpgradeTarget};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One explicit effect approval, including application calls. No implicit funding conversions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(try_from = "record::PlanRecord")]
pub struct Plan {
    /// Retained format version.
    pub version: u32,
    /// Exact network trust binding.
    pub network: Network,
    /// Required signer. The historical field name grants no controller authority to application calls.
    pub controller: String,
    /// Exact retained payload: Wasm for management, request bytes for application calls.
    pub artifact: Artifact,
    /// Explicit effect and its complete authority.
    pub action: PlanAction,
    /// Stable creation deduplication timestamp and plan identity component.
    pub created_at_ns: u64,
    /// Latest time at which a new effect may be signed under this approval.
    pub expires_at_ns: u64,
    /// Optional named slot; absent in legacy v1 plans, required in v2 plans.
    pub deployment: Option<crate::DeploymentName>,
}

/// Exact enduring effect owner for a single approved child.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedOwner {
    /// Exact application initialization bytes; absent preserves historical empty arguments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<u8>>,
    /// Exact Blob maintenance attempt for a configured upgrade; absent for creation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob_attempt: Option<u64>,
    /// Root principal, resolved from the selected parent slot.
    pub root: String,
    /// Exact reviewed Root executable hash, checked before admission and observation.
    pub root_module: String,
    /// Earliest autonomous dispatch, as Unix nanoseconds.
    pub execute_at_ns: u64,
    /// Authenticated runtime slot; absent preserves the original one-child contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<canic_management_protocol::children::ChildId>,
    /// Exact published Store content; absent selects the existing inline contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<crate::StoredArtifact>,
}

/// Operator-selected inputs for a one-canister plan.
pub struct PlanInput {
    /// Exact network and trust root.
    pub network: Network,
    /// Sole controller and signer principal.
    pub controller: String,
    /// Upgrade target, or no target for creation.
    pub target: Option<String>,
    /// Previous module hash, required with an upgrade target.
    pub previous_module: Option<String>,
    /// Creation budget including the expected ledger fee; zero for an upgrade.
    pub creation_cycles: u64,
}

/// Large Root deployment approvals commit to ordered uploads and a chunked install.
pub(crate) const CHUNKED_ROOT_VERSION: u32 = 12;
pub(crate) const CHILD_ACTIVATION_VERSION: u32 = 15;
pub(crate) const PLACEMENT_CHILD_ACTIVATION_VERSION: u32 = 16;
pub(crate) const STORE_MAINTENANCE_VERSION: u32 = 14;
pub(crate) const AUTH_ACCESS_VERSION: u32 = 13;

pub(crate) const fn root_install_version(bytes: usize, direct_version: u32) -> u32 {
    if bytes > artifact::MAX_WASM {
        CHUNKED_ROOT_VERSION
    } else {
        direct_version
    }
}

impl Plan {
    pub(crate) fn artifact_limit(&self) -> usize {
        if self.version == CHUNKED_ROOT_VERSION {
            artifact::MAX_ARTIFACT_BYTES
        } else if self.store_upgrade_root().is_some() {
            canic_management_protocol::store_maintenance::MAX_MODULE_BYTES
        } else if self.publication().is_some()
            || self.removal().is_some()
            || self.auth().is_some()
            || self.management().is_some_and(|m| m.source.is_some())
        {
            artifact::MAX_ARTIFACT_BYTES
        } else {
            artifact::MAX_WASM
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        self.network.validate()?;
        let controller = principal(&self.controller)?;
        if !matches!(
            (self.version, self.deployment.is_some()),
            (1, false)
                | (2, true)
                | (3, true)
                | (4, true)
                | (5, true)
                | (6, true)
                | (7, true)
                | (8, true)
                | (9, true)
                | (10, true)
                | (11, true)
                | (CHUNKED_ROOT_VERSION, true)
                | (AUTH_ACCESS_VERSION, true)
                | (STORE_MAINTENANCE_VERSION, true)
                | (
                    CHILD_ACTIVATION_VERSION | PLACEMENT_CHILD_ACTIVATION_VERSION,
                    true
                )
        ) || (self.version == 10
            && !matches!(
                self.action,
                PlanAction::Create {
                    bootstrap: None,
                    ..
                }
            ))
            || (self.version == STORE_MAINTENANCE_VERSION && self.store_upgrade_root().is_none())
            || (self.version == 9
                && self.store_root().is_none()
                && self.store_upgrade_root().is_none())
            || (self.version == 3) != self.publication().is_some()
            || matches!(self.version, 4 | 11) != self.removal().is_some()
            || matches!(self.version, 5 | AUTH_ACCESS_VERSION) != self.auth().is_some()
            || (matches!(
                self.version,
                CHILD_ACTIVATION_VERSION | PLACEMENT_CHILD_ACTIVATION_VERSION
            ) && !self
                .management()
                .is_some_and(|owner| owner.blob_attempt.is_some()))
            || (!matches!(self.version, 8 | PLACEMENT_CHILD_ACTIVATION_VERSION)
                && matches!(self.version, 6 | CHILD_ACTIVATION_VERSION)
                    != self.management().is_some_and(|m| m.arguments.is_some()))
            || (matches!(self.version, 8 | PLACEMENT_CHILD_ACTIVATION_VERSION)
                && !self.management().is_some_and(|m| m.source.is_some()))
            || (self.version == 7) != self.application_call().is_some()
            || (self.version == CHUNKED_ROOT_VERSION
                && (self.artifact.bytes <= artifact::MAX_WASM
                    || !matches!(
                        self.action,
                        PlanAction::Create { .. } | PlanAction::MaintainRoot { .. }
                    )))
            || self.artifact.bytes > self.artifact_limit()
            || self.artifact.sha256.len() != 64
            || self.expires_at_ns <= self.created_at_ns
            || self.expires_at_ns - self.created_at_ns > 3_600_000_000_000
        {
            return Err(Error::Invalid(
                "unsupported or unbounded deployment plan".into(),
            ));
        }
        if let Some(deployment) = &self.deployment {
            deployment.validate()?;
        }
        if self.maintenance().is_some() {
            crate::maintenance::validate(self)?;
        }
        if self.store_upgrade_root().is_some() {
            crate::store_upgrade::validate(self)?;
        }
        if let Some(root) = self.store_root() {
            let id = principal(&root.canister)?;
            if !matches!(self.version, 2 | 9)
                || id.to_text() != root.canister
                || id == principal("aaaaa-aa")?
                || id == principal("2vxsx-fae")?
                || crate::unhex(&root.module)?.len() != 32
                || root.module != root.module.to_ascii_lowercase()
            {
                return Err(Error::Invalid(
                    "Store bootstrap requires canonical Root identity and exact code".into(),
                ));
            }
        }
        if self.network == Network::Mainnet && controller == principal("2vxsx-fae")? {
            return Err(Error::Invalid(
                "mainnet requires an authenticated signer".into(),
            ));
        }
        if let Some(managed) = self.management() {
            let root = principal(&managed.root)?;
            if let Some(source) = &managed.source {
                crate::managed::stored::validate(self, source)?;
            }
            if let PlanAction::UpgradeChild { target, .. } = &self.action
                && (target.module == self.artifact.sha256 || principal(&target.canister)? == root)
            {
                return Err(Error::Invalid("invalid child upgrade target".into()));
            }
            if !matches!(
                self.version,
                2 | 6 | 8 | CHILD_ACTIVATION_VERSION | PLACEMENT_CHILD_ACTIVATION_VERSION
            ) || (managed.source.is_none()
                && self.artifact.bytes > canic_management_protocol::MAX_WASM)
                || crate::unhex(&managed.root_module)?.len() != 32
                || root == principal("aaaaa-aa")?
                || root == principal("2vxsx-fae")?
                || managed.execute_at_ns < self.created_at_ns
                || self.expires_at_ns.checked_sub(managed.execute_at_ns) != Some(600_000_000_000)
            {
                return Err(Error::Invalid("invalid Root-owned child approval".into()));
            }
            if managed.arguments.as_ref().is_some_and(|args| {
                args.is_empty()
                    || args.len() > canic_management_protocol::application::MAX_ARGUMENT_BYTES
                    || managed.source.is_none()
            }) {
                return Err(Error::Invalid(
                    "application arguments require Store-backed management".into(),
                ));
            }
            if managed.blob_attempt.is_some()
                != (managed.arguments.is_some() && self.target().is_some())
                || managed.blob_attempt == Some(0)
            {
                return Err(Error::Invalid(
                    "configured upgrades require an exact nonzero Blob attempt".into(),
                ));
            }
        }
        match &self.action {
            PlanAction::Call(value) => crate::application::validate(self, value)?,
            PlanAction::Auth(value) => crate::auth::validate(self, value)?,
            PlanAction::Publish(value) => crate::publication::validate(self, value)?,
            PlanAction::Remove(value) => crate::publication::removal::validate(self, value)?,
            PlanAction::Create {
                cycles,
                bootstrap: Some(policy),
            } => {
                let maximum = (u64::from(policy.services) + 2)
                    .saturating_mul(1_000_000_000_000)
                    .max(5_000_000_000_000);
                if !matches!(self.version, 2 | CHUNKED_ROOT_VERSION)
                    || !policy.valid()
                    || !(500_000_000_000..=maximum).contains(cycles)
                {
                    return Err(Error::Invalid(format!(
                        "Root creation budget {cycles} exceeds the approved range for {} services, or the subnet policy is invalid",
                        policy.services
                    )));
                }
            }
            PlanAction::Create {
                cycles,
                bootstrap: None,
            }
            | PlanAction::CreateStore { cycles, .. }
                if !(500_000_000_000..=5_000_000_000_000).contains(cycles) =>
            {
                return Err(Error::Invalid("creation needs 0.5T–5T cycles".into()));
            }
            PlanAction::Upgrade(target)
            | PlanAction::UpgradeStore { target, .. }
            | PlanAction::UpgradeChild { target, .. }
            | PlanAction::MaintainRoot { target, .. } => {
                principal(&target.canister)?;
                if crate::unhex(&target.module)?.len() != 32 {
                    return Err(Error::Invalid(
                        "upgrade needs an exact predecessor module hash".into(),
                    ));
                }
            }
            PlanAction::Create { .. }
            | PlanAction::CreateStore { .. }
            | PlanAction::CreateChild(_) => {}
        }
        Ok(())
    }

    /// Digest of the versioned typed plan, independent of its presentation whitespace.
    ///
    /// # Errors
    /// Refuses an invalid plan or serialization failure.
    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        let mut bytes = format!("canic:deployment-plan:v{}\0", self.version).into_bytes();
        bytes.extend(serde_json::to_vec(self).map_err(supplier)?);
        Ok(artifact::digest(&bytes))
    }

    /// Sole controller of the resulting application canister.
    #[must_use]
    pub fn application_controller(&self) -> &str {
        if let Some(root) = self.store_upgrade_root() {
            return &root.canister;
        }
        if let Some(root) = self.store_root() {
            return &root.canister;
        }
        self.management()
            .map_or(&self.controller, |owner| &owner.root)
    }
}

/// Retain a reviewable plan and its immutable artifact without network mutations.
///
/// # Errors
/// Refuses invalid budgets, occupied records, concurrent operators or failed durable publication.
#[cfg(test)]
pub(crate) fn create_plan(root: &Path, wasm: &Path, input: PlanInput) -> Result<(String, Plan)> {
    retain(root, wasm, input, None, None, crate::now()?)
}

// Equality is the ready-to-dispatch plan. Historical future schedules retain their
// original pre-dispatch admission cutoff; no serialized authority is rewritten.
pub(crate) fn admission_deadline(created: u64, dispatch: u64, expires: u64) -> u64 {
    if dispatch == created {
        expires
    } else {
        dispatch
    }
}

#[cfg(test)]
pub(crate) fn retain(
    root: &Path,
    wasm: &Path,
    input: PlanInput,
    deployment: Option<crate::DeploymentName>,
    management: Option<ManagedOwner>,
    now: u64,
) -> Result<(String, Plan)> {
    let (plan, bytes) = prepare(wasm, input, deployment, management, now)?;
    retain_locked(&Store::open(root)?, plan, &bytes)
}

pub(crate) fn prepare(
    wasm: &Path,
    input: PlanInput,
    deployment: Option<crate::DeploymentName>,
    management: Option<ManagedOwner>,
    now: u64,
) -> Result<(Plan, Vec<u8>)> {
    let limit = if management.as_ref().is_some_and(|m| m.source.is_some())
        || (deployment.is_some() && management.is_none() && input.target.is_none())
    {
        artifact::MAX_ARTIFACT_BYTES
    } else {
        artifact::MAX_WASM
    };
    let bytes = store::read(wasm, limit)?;
    let expires_at_ns = management
        .as_ref()
        .map_or(Ok(now + 3_600_000_000_000), |owner| {
            owner
                .execute_at_ns
                .checked_add(600_000_000_000)
                .ok_or_else(|| Error::Invalid("dispatch deadline overflow".into()))
        })?;
    let action = match (input.target, input.previous_module, management) {
        (None, None, None) => PlanAction::Create {
            bootstrap: None,
            cycles: input.creation_cycles,
        },
        (None, None, Some(owner))
            if input.creation_cycles == canic_management_protocol::CREATION_CYCLES =>
        {
            PlanAction::CreateChild(owner)
        }
        (Some(canister), Some(module), None) if input.creation_cycles == 0 => {
            PlanAction::Upgrade(UpgradeTarget { canister, module })
        }
        (Some(canister), Some(module), Some(owner)) if input.creation_cycles == 0 => {
            PlanAction::UpgradeChild {
                target: UpgradeTarget { canister, module },
                owner,
            }
        }
        _ => {
            return Err(Error::Invalid(
                "incomplete creation or upgrade authority".into(),
            ));
        }
    };
    let role_service = if let PlanAction::CreateChild(owner) = &action
        && let Some(bytes) = &owner.arguments
    {
        canic::topology::directory::DbService::from_receiver(bytes, principal(&owner.root)?)
            .map_err(|e| Error::Invalid(e.into()))?
            .is_some()
    } else {
        false
    };
    let plan = Plan {
        version: if role_service {
            8
        } else if matches!(&action, PlanAction::CreateChild(owner) | PlanAction::UpgradeChild { owner, .. } if owner.arguments.is_some())
        {
            6
        } else if deployment.is_some() {
            2
        } else {
            1
        },
        deployment,
        network: input.network,
        controller: input.controller,
        artifact: artifact::admit_bounded(&bytes, limit)?,
        action,
        created_at_ns: now,
        expires_at_ns,
    };
    Ok((plan, bytes))
}

/// Lookup and publication share the caller's journal lock.
pub(crate) fn retain_locked(store: &Store, mut plan: Plan, bytes: &[u8]) -> Result<(String, Plan)> {
    let root = &store.path;
    let publication_predecessors = if plan.publication().is_some() {
        crate::publication::removal::publication_predecessors(root, &plan)?
    } else {
        Vec::new()
    };
    let mut retained = None;
    let mut fleet_predecessors = None;
    if let Some(name) = plan.deployment.clone() {
        // The journal lock spans lookup and publication, including an interrupted earlier plan.
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(root).map_err(supplier)? {
            let entry = entry.map_err(supplier)?;
            let existing_id = entry.file_name().to_string_lossy().into_owned();
            if existing_id.len() != 64
                || !existing_id
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                continue;
            }
            entries.push(entry);
            if entries.len() > 128 {
                return Err(Error::Invalid(
                    "named planning inventory exceeds 128 operations".into(),
                ));
            }
        }
        // Resolve inherited maintenance custody before comparing a retained named approval.
        for entry in &entries {
            let existing_id = entry.file_name().to_string_lossy().into_owned();
            if crate::fleet_plan::exists(&entry.path())? {
                if plan.publication().is_some() {
                    continue;
                }
                let fleet = crate::fleet_plan::load(&entry.path(), &existing_id)?;
                if crate::fleet_plan::cancelled(&entry.path(), &existing_id)? {
                    continue;
                }
                if fleet.reserves(&name) {
                    let predecessors = crate::fleet_plan::permits_upgrade(store, &existing_id, &fleet, &mut plan)?
                        .ok_or_else(|| Error::Blocked(format!("fleet already belongs to operation {existing_id}; resume that whole-fleet approval")))?;
                    if fleet_predecessors.replace(predecessors).is_some() {
                        return Err(Error::Blocked(
                            "multiple approvals reserve this fleet; inspect the original journal"
                                .into(),
                        ));
                    }
                }
                continue;
            }
        }
        for entry in entries {
            let existing_id = entry.file_name().to_string_lossy().into_owned();
            if crate::fleet_plan::exists(&entry.path())? {
                continue;
            }
            let existing: Plan = store::json(&entry.path().join("plan.json"))?;
            if existing.digest()? != existing_id {
                return Err(Error::Invalid("retained plan digest mismatch".into()));
            }
            if existing.application_call().is_some()
                || existing.auth().is_some()
                || existing.removal().is_some()
                || publication_predecessors.contains(&existing_id)
            {
                continue;
            }
            // Only the shared history owner can prove that these earlier upgrades completed.
            if fleet_predecessors
                .as_ref()
                .is_some_and(|ids| ids.contains(&existing_id))
            {
                continue;
            }
            if existing.publication().is_some() || plan.publication().is_some() {
                if let (Some(old), Some(new)) = (existing.publication(), plan.publication())
                    && existing.network == plan.network
                    && old.store.canister == new.store.canister
                    && old.release == new.release
                {
                    let (existing, _) = load(&entry.path(), &existing_id)?;
                    if existing.publication() == plan.publication()
                        && existing.controller == plan.controller
                        && existing.artifact == plan.artifact
                        && existing.deployment == plan.deployment
                    {
                        if retained.replace((existing_id, existing)).is_some() {
                            return Err(Error::Blocked(
                                "multiple approvals reserve this release; inspect the original journal".into(),
                            ));
                        }
                        continue;
                    }
                    return Err(Error::Blocked(format!(
                        "release already belongs to operation {existing_id}; retained authority cannot be replaced"
                    )));
                }
                continue;
            }
            if existing.deployment.as_ref() != Some(&name)
                && existing.network == plan.network
                && existing
                    .management()
                    .zip(plan.management())
                    .is_some_and(|(old, new)| {
                        old.root == new.root && crate::managed::same_slot(old.slot, new.slot)
                    })
            {
                return Err(Error::Blocked(format!(
                    "Root's child slot already belongs to operation {existing_id}"
                )));
            }
            if existing.deployment.as_ref() == Some(&name) {
                let (existing, _) = load(&entry.path(), &existing_id)?;
                // Creation remains immutable alongside upgrades. Fleet history above
                // owns successor admission; standalone named children keep the first-upgrade bound.
                if existing.network == plan.network
                    && existing
                        .management()
                        .zip(plan.management())
                        .is_some_and(|(old, new)| old.root == new.root)
                    && existing.target().is_some() != plan.target().is_some()
                {
                    if fleet_predecessors.is_none()
                        && existing.target().is_none()
                        && plan.previous_module() != Some(&existing.artifact.sha256)
                    {
                        return Err(Error::Blocked(
                            "upgrade requires its retained predecessor module".into(),
                        ));
                    }
                    if existing
                        .management()
                        .and_then(|m| m.source.as_ref())
                        .is_some()
                        != plan.management().and_then(|m| m.source.as_ref()).is_some()
                    {
                        return Err(Error::Blocked("changing an admitted child between inline and Store custody requires qualified migration".into()));
                    }
                    continue;
                }
                if same_authority(&existing, &plan) {
                    if retained.replace((existing_id, existing)).is_some() {
                        return Err(Error::Blocked(
                            "multiple approvals reserve this named slot; inspect the original journal".into(),
                        ));
                    }
                    continue;
                }
                return Err(Error::Blocked(format!(
                    "named slot already belongs to operation {existing_id}; inspect its status instead of creating new authority"
                )));
            }
        }
    }
    // Every reservation is checked under the lock before reusing any matching authority.
    // Reopening returns the original authority before checking a new schedule.
    // A past requested dispatch cannot renew or replace that retained operation.
    if let Some(retained) = retained {
        return Ok(retained);
    }
    publish_locked(store, plan, bytes)
}

/// A retry may request a different schedule, but cannot change the retained authority.
pub(crate) fn same_authority(existing: &Plan, proposed: &Plan) -> bool {
    let mut proposed = proposed.clone();
    proposed.created_at_ns = existing.created_at_ns;
    proposed.expires_at_ns = existing.expires_at_ns;
    if let Some(old) = existing.management() {
        match &mut proposed.action {
            PlanAction::CreateChild(owner) | PlanAction::UpgradeChild { owner, .. } => {
                owner.execute_at_ns = old.execute_at_ns;
            }
            _ => {}
        }
    }
    *existing == proposed
}

/// Publish after the caller validates its reservation under this same lock.
pub(crate) fn publish_locked(store: &Store, plan: Plan, bytes: &[u8]) -> Result<(String, Plan)> {
    crate::auth::permit_upgrade(&store.path, &plan)?;
    let id = plan.digest()?;
    let dir = store.operation(&id)?;
    let held = crate::output_publication::prepare(&dir, 0o700)?;
    let file = if plan.application_call().is_some() {
        "request.bin"
    } else {
        "module.wasm"
    };
    store::put_in(&held, std::ffi::OsStr::new(file), bytes)?;
    let plan_bytes = serde_json::to_vec_pretty(&plan).map_err(supplier)?;
    store::put_in(&held, std::ffi::OsStr::new("plan.json"), &plan_bytes)?;
    Ok((id, plan))
}

pub(crate) fn load(dir: &Path, digest: &str) -> Result<(Plan, Vec<u8>)> {
    let plan: Plan = store::json(&dir.join("plan.json"))?;
    let bytes = if plan.application_call().is_some() {
        store::read(
            &dir.join("request.bin"),
            crate::application::MAX_REQUEST_BYTES,
        )?
    } else {
        store::read(&dir.join("module.wasm"), plan.artifact_limit())?
    };
    let actual = if plan.application_call().is_some() {
        Artifact {
            sha256: artifact::digest(&bytes),
            bytes: bytes.len(),
        }
    } else {
        artifact::admit_bounded(&bytes, plan.artifact_limit())?
    };
    if plan.digest()? != digest || actual != plan.artifact {
        return Err(Error::Invalid(
            "retained plan or artifact differs from the approved digest".into(),
        ));
    }
    Ok((plan, bytes))
}

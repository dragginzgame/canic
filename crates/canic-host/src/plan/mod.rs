use crate::{
    Artifact, Error, Network, Result, artifact,
    network::principal,
    now,
    store::{self, Store},
    supplier,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
mod action;
mod record;
#[cfg(test)]
mod tests;
pub use action::{PlanAction, StoreMaintenance, UpgradeTarget};

/// Complete one-canister deployment approval. No implicit funding conversions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(try_from = "record::PlanRecord")]
pub struct Plan {
    /// Retained format version.
    pub version: u32,
    /// Exact network trust binding.
    pub network: Network,
    /// Required signer; sole direct controller, or sole operator controller of Root.
    pub controller: String,
    /// Exact installation bytes.
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
    /// Root principal, resolved from the selected parent slot.
    pub root: String,
    /// Exact reviewed Root executable hash, checked before admission and observation.
    pub root_module: String,
    /// Earliest autonomous dispatch, as Unix nanoseconds.
    pub execute_at_ns: u64,
    /// Authenticated runtime slot; absent preserves the original one-child contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<canic_management_protocol::children::Slot>,
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

impl Plan {
    pub(crate) fn artifact_limit(&self) -> usize {
        if self.store_upgrade_root().is_some() {
            canic_management_protocol::store_maintenance::MAX_MODULE_BYTES
        } else if self.publication().is_some()
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
            (1, false) | (2, true) | (3, true)
        ) || (self.version == 3) != self.publication().is_some()
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
            if self.version != 2
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
                "mainnet requires an authenticated controller".into(),
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
            if self.version != 2
                || (managed.source.is_none()
                    && self.artifact.bytes > canic_management_protocol::MAX_WASM)
                || crate::unhex(&managed.root_module)?.len() != 32
                || root == principal("aaaaa-aa")?
                || root == principal("2vxsx-fae")?
                || managed.execute_at_ns < self.created_at_ns
                || self.expires_at_ns.checked_sub(managed.execute_at_ns) != Some(600_000_000_000)
            {
                return Err(Error::Invalid("invalid Root-owned child approval".into()));
            }
        }
        match &self.action {
            PlanAction::Publish(value) => crate::publication::validate(self, value)?,
            PlanAction::Create { cycles } | PlanAction::CreateStore { cycles, .. }
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
pub fn create_plan(root: &Path, wasm: &Path, input: PlanInput) -> Result<(String, Plan)> {
    retain(root, wasm, input, None, None, now()?)
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

pub(crate) fn retain(
    root: &Path,
    wasm: &Path,
    input: PlanInput,
    deployment: Option<crate::DeploymentName>,
    management: Option<ManagedOwner>,
    now: u64,
) -> Result<(String, Plan)> {
    let limit = if management.as_ref().is_some_and(|m| m.source.is_some()) {
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
    let plan = Plan {
        version: if deployment.is_some() { 2 } else { 1 },
        deployment,
        network: input.network,
        controller: input.controller,
        artifact: artifact::admit_bounded(&bytes, limit)?,
        action,
        created_at_ns: now,
        expires_at_ns,
    };
    let store = Store::open(root)?;
    retain_locked(&store, plan, &bytes)
}

/// Lookup and publication share the caller's journal lock.
pub(crate) fn retain_locked(store: &Store, plan: Plan, bytes: &[u8]) -> Result<(String, Plan)> {
    let root = &store.path;
    if let Some(name) = &plan.deployment {
        // The journal lock spans lookup and publication, including an interrupted earlier plan.
        let mut count = 0;
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
            count += 1;
            if count > 128 {
                return Err(Error::Invalid(
                    "named planning inventory exceeds 128 operations".into(),
                ));
            }
            if crate::fleet_plan::exists(&entry.path())? {
                if plan.publication().is_some() {
                    continue;
                }
                let fleet = crate::fleet_plan::load(&entry.path(), &existing_id)?;
                if crate::fleet_plan::cancelled(&entry.path(), &existing_id)? {
                    continue;
                }
                if fleet.reserves(name)
                    && !crate::fleet_plan::permits_upgrade(&entry.path(), &fleet, &plan)?
                {
                    return Err(Error::Blocked(format!(
                        "fleet already belongs to operation {existing_id}; resume that whole-fleet approval"
                    )));
                }
                continue;
            }
            let existing: Plan = store::json(&entry.path().join("plan.json"))?;
            if existing.digest()? != existing_id {
                return Err(Error::Invalid("retained plan digest mismatch".into()));
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
                        return Ok((existing_id, existing));
                    }
                    return Err(Error::Blocked(format!(
                        "release already belongs to operation {existing_id}; retained authority cannot be replaced"
                    )));
                }
                continue;
            }
            if existing.deployment.as_ref() != Some(name)
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
            if existing.deployment.as_ref() == Some(name) {
                let (existing, _) = load(&entry.path(), &existing_id)?;
                // Creation and the first child upgrade retain separate approvals.
                // Never overwrite either; subsequent upgrades still hit the occupied slot.
                if existing.network == plan.network
                    && existing
                        .management()
                        .zip(plan.management())
                        .is_some_and(|(old, new)| old.root == new.root)
                    && existing.target().is_some() != plan.target().is_some()
                {
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
                if existing.network == plan.network
                    && existing.controller == plan.controller
                    && existing.artifact == plan.artifact
                    && existing.creation_cycles() == plan.creation_cycles()
                    && existing.target() == plan.target()
                    && existing.previous_module() == plan.previous_module()
                    && existing.store_root() == plan.store_root()
                    && existing
                        .management()
                        .map(|owner| (&owner.root, &owner.root_module, owner.slot, &owner.source))
                        == plan.management().map(|owner| {
                            (&owner.root, &owner.root_module, owner.slot, &owner.source)
                        })
                {
                    return Ok((existing_id, existing));
                }
                return Err(Error::Blocked(format!(
                    "named slot already belongs to operation {existing_id}; inspect its status instead of creating new authority"
                )));
            }
        }
    }
    // Reopening returns the original authority before checking a new schedule.
    // A past requested dispatch cannot renew or replace that retained operation.
    publish_locked(store, plan, bytes)
}

/// Publish after the caller validates its reservation under this same lock.
pub(crate) fn publish_locked(store: &Store, plan: Plan, bytes: &[u8]) -> Result<(String, Plan)> {
    let id = plan.digest()?;
    let dir = store.operation(&id)?;
    ic_host_fs::durable::prepare_private_parents(&dir).map_err(supplier)?;
    store::put(&dir.join("module.wasm"), bytes)?;
    store::put_json(&dir.join("plan.json"), &plan)?;
    Ok((id, plan))
}

pub(crate) fn load(dir: &Path, digest: &str) -> Result<(Plan, Vec<u8>)> {
    let plan: Plan = store::json(&dir.join("plan.json"))?;
    let bytes = store::read(&dir.join("module.wasm"), plan.artifact_limit())?;
    if plan.digest()? != digest
        || artifact::admit_bounded(&bytes, plan.artifact_limit())? != plan.artifact
    {
        return Err(Error::Invalid(
            "retained plan or artifact differs from the approved digest".into(),
        ));
    }
    Ok((plan, bytes))
}

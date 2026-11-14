use crate::{
    Artifact, Error, Network, Result, artifact,
    network::principal,
    now,
    store::{self, Store},
    supplier,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Complete one-canister deployment approval. No implicit funding conversions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    /// Retained format version.
    pub version: u32,
    /// Exact network trust binding.
    pub network: Network,
    /// Required signer; sole direct controller, or sole operator controller of Root.
    pub controller: String,
    /// Exact installation bytes.
    pub artifact: Artifact,
    /// Existing canister to upgrade; absent means the retained output of creation.
    pub target: Option<String>,
    /// Required previous module hash for upgrade; absent for fresh installation.
    pub previous_module: Option<String>,
    /// Direct ledger debit including fee, or the exact Root-owned child attachment.
    pub creation_cycles: u64,
    /// Stable creation deduplication timestamp and plan identity component.
    pub created_at_ns: u64,
    /// Latest time at which a new effect may be signed under this approval.
    pub expires_at_ns: u64,
    /// Optional named slot; absent in legacy v1 plans, required in v2 plans.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployment: Option<crate::DeploymentName>,
    /// Explicit Root-owned creation or upgrade; absent bindings retain their original encoding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub management: Option<ManagedOwner>,
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
    pub(crate) fn validate(&self) -> Result<()> {
        self.network.validate()?;
        let controller = principal(&self.controller)?;
        if !matches!(
            (self.version, self.deployment.is_some()),
            (1, false) | (2, true)
        ) || self.artifact.bytes > artifact::MAX_WASM
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
        if self.network == Network::Mainnet && controller == principal("2vxsx-fae")? {
            return Err(Error::Invalid(
                "mainnet requires an authenticated controller".into(),
            ));
        }
        if let Some(managed) = &self.management {
            let root = principal(&managed.root)?;
            let kind_valid = match (&self.target, &self.previous_module) {
                (None, None) => self.creation_cycles == canic_management_protocol::CREATION_CYCLES,
                (Some(child), Some(previous)) => {
                    self.creation_cycles == 0
                        && previous != &self.artifact.sha256
                        && principal(child)? != root
                }
                _ => false,
            };
            if self.version != 2
                || !kind_valid
                || self.artifact.bytes > canic_management_protocol::MAX_WASM
                || crate::unhex(&managed.root_module)?.len() != 32
                || root == principal("aaaaa-aa")?
                || root == principal("2vxsx-fae")?
                || managed.execute_at_ns <= self.created_at_ns
                || self.expires_at_ns.checked_sub(managed.execute_at_ns) != Some(600_000_000_000)
            {
                return Err(Error::Invalid("invalid Root-owned child approval".into()));
            }
        }
        match (&self.target, &self.previous_module) {
            (None, None) if (500_000_000_000..=5_000_000_000_000).contains(&self.creation_cycles) => {},
            (Some(target), Some(hash)) if self.creation_cycles == 0 && crate::unhex(hash)?.len() == 32 => { principal(target)?; },
            _ => return Err(Error::Invalid("creation needs 0.5T–5T cycles; upgrade needs target, previous hash and zero creation cycles".into())),
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
        self.management
            .as_ref()
            .map_or(&self.controller, |owner| &owner.root)
    }
}

/// Retain a reviewable plan and its immutable artifact without network mutations.
///
/// # Errors
/// Refuses invalid budgets, occupied records, concurrent operators or failed durable publication.
pub fn create_plan(root: &Path, wasm: &Path, input: PlanInput) -> Result<(String, Plan)> {
    retain(root, wasm, input, None, None)
}

pub(crate) fn retain(
    root: &Path,
    wasm: &Path,
    input: PlanInput,
    deployment: Option<crate::DeploymentName>,
    management: Option<ManagedOwner>,
) -> Result<(String, Plan)> {
    let bytes = store::read(wasm, artifact::MAX_WASM)?;
    let now = now()?;
    let plan = Plan {
        version: if deployment.is_some() { 2 } else { 1 },
        deployment,
        network: input.network,
        controller: input.controller,
        artifact: artifact::admit(&bytes)?,
        target: input.target,
        previous_module: input.previous_module,
        creation_cycles: input.creation_cycles,
        created_at_ns: now,
        expires_at_ns: management
            .as_ref()
            .map_or(Ok(now + 3_600_000_000_000), |owner| {
                owner
                    .execute_at_ns
                    .checked_add(600_000_000_000)
                    .ok_or_else(|| Error::Invalid("dispatch deadline overflow".into()))
            })?,
        management,
    };
    let store = Store::open(root)?;
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
            let existing: Plan = store::json(&entry.path().join("plan.json"))?;
            if existing.digest()? != existing_id {
                return Err(Error::Invalid("retained plan digest mismatch".into()));
            }
            if existing.deployment.as_ref() != Some(name)
                && existing.network == plan.network
                && existing
                    .management
                    .as_ref()
                    .zip(plan.management.as_ref())
                    .is_some_and(|(old, new)| old.root == new.root)
            {
                return Err(Error::Blocked(format!(
                    "Root's single child slot already belongs to operation {existing_id}"
                )));
            }
            if existing.deployment.as_ref() == Some(name) {
                let (existing, _) = load(&entry.path(), &existing_id)?;
                // Creation and the first child upgrade retain separate approvals.
                // Never overwrite either; subsequent upgrades still hit the occupied slot.
                if existing.network == plan.network
                    && existing
                        .management
                        .as_ref()
                        .zip(plan.management.as_ref())
                        .is_some_and(|(old, new)| old.root == new.root)
                    && existing.target.is_some() != plan.target.is_some()
                {
                    continue;
                }
                if existing.network == plan.network
                    && existing.controller == plan.controller
                    && existing.artifact == plan.artifact
                    && existing.creation_cycles == plan.creation_cycles
                    && existing.target == plan.target
                    && existing.previous_module == plan.previous_module
                    && existing
                        .management
                        .as_ref()
                        .map(|owner| (&owner.root, &owner.root_module))
                        == plan
                            .management
                            .as_ref()
                            .map(|owner| (&owner.root, &owner.root_module))
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
    let id = plan.digest()?;
    let dir = store.operation(&id)?;
    ic_host_fs::durable::prepare_private_parents(&dir).map_err(supplier)?;
    store::put(&dir.join("module.wasm"), &bytes)?;
    store::put_json(&dir.join("plan.json"), &plan)?;
    Ok((id, plan))
}

pub(crate) fn load(dir: &Path, digest: &str) -> Result<(Plan, Vec<u8>)> {
    let plan: Plan = store::json(&dir.join("plan.json"))?;
    let bytes = store::read(&dir.join("module.wasm"), artifact::MAX_WASM)?;
    if plan.digest()? != digest || artifact::admit(&bytes)? != plan.artifact {
        return Err(Error::Invalid(
            "retained plan or artifact differs from the approved digest".into(),
        ));
    }
    Ok((plan, bytes))
}

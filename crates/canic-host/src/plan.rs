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
    /// Sole controller and required signing principal.
    pub controller: String,
    /// Exact installation bytes.
    pub artifact: Artifact,
    /// Existing canister to upgrade; absent means the retained output of creation.
    pub target: Option<String>,
    /// Required previous module hash for upgrade; absent for fresh installation.
    pub previous_module: Option<String>,
    /// Approved creation budget including the expected ledger fee, checked before dispatch.
    pub creation_cycles: u64,
    /// Stable creation deduplication timestamp and plan identity component.
    pub created_at_ns: u64,
    /// Latest time at which a new effect may be signed under this approval.
    pub expires_at_ns: u64,
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
        if self.version != 1
            || self.artifact.bytes > artifact::MAX_WASM
            || self.artifact.sha256.len() != 64
            || self.expires_at_ns <= self.created_at_ns
            || self.expires_at_ns - self.created_at_ns > 3_600_000_000_000
        {
            return Err(Error::Invalid(
                "unsupported or unbounded deployment plan".into(),
            ));
        }
        if self.network == Network::Mainnet && controller == principal("2vxsx-fae")? {
            return Err(Error::Invalid(
                "mainnet requires an authenticated controller".into(),
            ));
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
        let mut bytes = b"canic:deployment-plan:v1\0".to_vec();
        bytes.extend(serde_json::to_vec(self).map_err(supplier)?);
        Ok(artifact::digest(&bytes))
    }
}

/// Retain a reviewable plan and its immutable artifact without network mutations.
///
/// # Errors
/// Refuses invalid budgets, occupied records, concurrent operators or failed durable publication.
pub fn create_plan(root: &Path, wasm: &Path, input: PlanInput) -> Result<(String, Plan)> {
    let bytes = store::read(wasm, artifact::MAX_WASM)?;
    let now = now()?;
    let plan = Plan {
        version: 1,
        network: input.network,
        controller: input.controller,
        artifact: artifact::admit(&bytes)?,
        target: input.target,
        previous_module: input.previous_module,
        creation_cycles: input.creation_cycles,
        created_at_ns: now,
        expires_at_ns: now + 3_600_000_000_000,
    };
    let id = plan.digest()?;
    let store = Store::open(root)?;
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

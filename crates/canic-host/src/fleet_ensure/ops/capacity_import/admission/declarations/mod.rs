//! Parse explicit operator disposition declarations without filling absent authority fields.

use crate::fleet_ensure::{
    model::capacity_import::{
        CapacityImportDisposition, CapacityImportPlanRecord, CapacityImportSourceBinding,
    },
    ops::capacity_import::{
        CapacityImportReviewError,
        admission::{MAXIMUM_DECLARATION_BYTES, canonical_controllers},
    },
};
use candid::Principal;
use canic_core::{cdk::utils::hash::decode_hex, ids::SubnetId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Operator-owned TOML document retained byte for byte in the import review.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportDeclarations {
    pub schema_version: u16,
    pub operator: String,
    pub network_root_key_sha256: String,
    pub canisters: Vec<CapacityImportDeclaration>,
}

/// Explicit destructive disposition for one original physical binding.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportDeclaration {
    pub canister: String,
    pub subnet: String,
    pub controllers: Vec<String>,
    /// Exact installed hash, or the explicit literal `empty` for no installed module.
    pub module_sha256: String,
    pub canister_version: u64,
    pub disposition: CapacityImportDispositionKind,
    pub no_external_obligations: bool,
    pub no_other_fleet_ownership: bool,
    pub evidence: String,
}

/// Operator assertion of absence or completed retirement of outside obligations.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityImportDispositionKind {
    Absence,
    Retired,
}

pub(super) fn parse(text: &str) -> Result<CapacityImportDeclarations, CapacityImportReviewError> {
    if text.len() > MAXIMUM_DECLARATION_BYTES {
        return Err(CapacityImportReviewError::AdmissionInvalid);
    }
    toml::from_str(text).map_err(|_| CapacityImportReviewError::AdmissionInvalid)
}

pub(super) fn validate(
    plan: &CapacityImportPlanRecord,
    document: &CapacityImportDeclarations,
    digest: [u8; 32],
) -> Result<(), CapacityImportReviewError> {
    let invalid = || CapacityImportReviewError::AdmissionInvalid;
    if document.schema_version != 1
        || principal(&document.operator)? != plan.authority.operator
        || hash(&document.network_root_key_sha256)? != plan.authority.network_root_key_sha256
        || document.canisters.len() != plan.sources.len()
    {
        return Err(invalid());
    }
    let mut seen = BTreeSet::new();
    for declaration in &document.canisters {
        let binding = declaration.binding()?;
        if !seen.insert(binding.canister_id)
            || !declaration.no_external_obligations
            || !declaration.no_other_fleet_ownership
            || declaration.evidence.trim().is_empty()
        {
            return Err(invalid());
        }
        let source = plan
            .sources
            .iter()
            .find(|source| source.binding.canister_id == binding.canister_id)
            .ok_or_else(invalid)?;
        // Running/empty state and snapshots are live admission facts, not operator assertions.
        let mut expected = source.binding.clone();
        expected.stopped = true;
        expected.snapshots_size_bytes = 0;
        if expected != binding || source.disposition != declaration.disposition(digest) {
            return Err(invalid());
        }
    }
    Ok(())
}

impl CapacityImportDeclaration {
    /// Construct the exact declared physical identity; actual stopped/snapshot state must be read live.
    pub fn binding(&self) -> Result<CapacityImportSourceBinding, CapacityImportReviewError> {
        let mut controllers = self
            .controllers
            .iter()
            .map(|value| principal(value))
            .collect::<Result<Vec<_>, _>>()?;
        controllers.sort_unstable();
        if !canonical_controllers(&controllers) {
            return Err(CapacityImportReviewError::AdmissionInvalid);
        }
        Ok(CapacityImportSourceBinding {
            canister_id: principal(&self.canister)?,
            subnet: SubnetId::from_principal(principal(&self.subnet)?),
            controllers,
            module_sha256: if self.module_sha256 == "empty" {
                None
            } else {
                Some(hash(&self.module_sha256)?)
            },
            canister_version: self.canister_version,
            stopped: true,
            snapshots_size_bytes: 0,
        })
    }

    /// Bind the disposition to the unchanged complete operator document.
    #[must_use]
    pub const fn disposition(&self, evidence_sha256: [u8; 32]) -> CapacityImportDisposition {
        match self.disposition {
            CapacityImportDispositionKind::Absence => {
                CapacityImportDisposition::AbsenceEvidence { evidence_sha256 }
            }
            CapacityImportDispositionKind::Retired => {
                CapacityImportDisposition::Retired { evidence_sha256 }
            }
        }
    }
}

fn principal(value: &str) -> Result<Principal, CapacityImportReviewError> {
    Principal::from_text(value).map_err(|_| CapacityImportReviewError::AdmissionInvalid)
}

pub(super) fn hash(value: &str) -> Result<[u8; 32], CapacityImportReviewError> {
    decode_hex(value)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(CapacityImportReviewError::AdmissionInvalid)
}

//! Bind explicit held children to certified custody before installing their current Root.
//!
//! A held declaration contains no claimed management version or cycle balance.

use crate::fleet_ensure::{
    model::{
        DesiredCanisterKind, DesiredFleet,
        capacity_import::CapacityImportDisposition,
        infrastructure_bootstrap::{
            InfrastructureBootstrapCustodyRecord, InfrastructureBootstrapHeldSourceRecord,
        },
    },
    ops::{
        capacity_import::admission::{CapacityImportDeclaration, CapacityImportDispositionKind},
        certified_custody,
        infrastructure_bootstrap::InfrastructureBootstrapError,
    },
};
use candid::Principal;
use canic_contracts::ids::SubnetId;
use canic_core::cdk::utils::hash::decode_hex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Operator declarations for observed infrastructure and children held by a supplied Root.

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct BootstrapDeclarations {
    pub schema_version: u16,
    pub operator: String,
    pub network_root_key_sha256: String,
    pub canisters: Vec<CapacityImportDeclaration>,
    #[serde(default)]
    pub root_owned: Vec<HeldDeclaration>,
}

/// Physical facts and destructive disposition; live balances are sampled after Root initialization.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct HeldDeclaration {
    pub canister: String,
    pub subnet: String,
    pub controllers: Vec<String>,
    pub module_sha256: String,
    pub disposition: CapacityImportDispositionKind,
    pub no_external_obligations: bool,
    pub no_other_fleet_ownership: bool,
    pub evidence: String,
}

impl HeldDeclaration {
    /// Convert operator fields into exact physical custody, without filling unobserved values.
    pub(in crate::fleet_ensure) fn binding(
        &self,
    ) -> Result<InfrastructureBootstrapCustodyRecord, InfrastructureBootstrapError> {
        if !self.no_external_obligations
            || !self.no_other_fleet_ownership
            || self.evidence.trim().is_empty()
        {
            return Err(InfrastructureBootstrapError::Integrity);
        }
        let mut controllers = self
            .controllers
            .iter()
            .map(|value| principal(value))
            .collect::<Result<Vec<_>, _>>()?;
        controllers.sort_unstable();
        if controllers.is_empty()
            || controllers.len() > 10
            || !controllers.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(InfrastructureBootstrapError::Integrity);
        }
        Ok(InfrastructureBootstrapCustodyRecord {
            canister: principal(&self.canister)?,
            subnet: SubnetId::from_principal(principal(&self.subnet)?),
            controllers,
            module_sha256: module_hash(&self.module_sha256)?,
        })
    }

    pub(in crate::fleet_ensure) const fn disposition(
        &self,
        digest: [u8; 32],
    ) -> CapacityImportDisposition {
        match self.disposition {
            CapacityImportDispositionKind::Absence => CapacityImportDisposition::AbsenceEvidence {
                evidence_sha256: digest,
            },
            CapacityImportDispositionKind::Retired => CapacityImportDisposition::Retired {
                evidence_sha256: digest,
            },
        }
    }
}

/// Resolve each declared child only to its explicit supplied parent Root.
pub(in crate::fleet_ensure) fn held_sources(
    desired: &DesiredFleet,
    declarations: &BootstrapDeclarations,
    digest: [u8; 32],
) -> Result<BTreeMap<String, InfrastructureBootstrapHeldSourceRecord>, InfrastructureBootstrapError>
{
    let mut result = BTreeMap::new();
    for declaration in &declarations.root_owned {
        let binding = declaration.binding()?;
        let configured = desired
            .canisters
            .iter()
            .find(|entry| entry.principal.as_deref() == Some(declaration.canister.as_str()))
            .ok_or(InfrastructureBootstrapError::Integrity)?;
        let parent = desired
            .canisters
            .iter()
            .find(|entry| Some(&entry.name) == configured.parent.as_ref())
            .filter(|entry| entry.kind == DesiredCanisterKind::Root)
            .ok_or(InfrastructureBootstrapError::Integrity)?;
        let root = principal(
            parent
                .principal
                .as_deref()
                .ok_or(InfrastructureBootstrapError::Integrity)?,
        )?;
        if configured.kind != DesiredCanisterKind::Pool
            || root == binding.canister
            || !binding.controllers.contains(&root)
            || principal(&configured.subnet)? != binding.subnet.into_principal()
            || principal(&parent.subnet)? != binding.subnet.into_principal()
        {
            return Err(InfrastructureBootstrapError::Integrity);
        }
        let record = InfrastructureBootstrapHeldSourceRecord {
            root,
            custody: binding,
            disposition: declaration.disposition(digest),
        };
        if result.insert(configured.name.clone(), record).is_some() {
            return Err(InfrastructureBootstrapError::Integrity);
        }
    }
    Ok(result)
}

/// Read certified fields only; no predecessor runtime or management update is called.
pub(in crate::fleet_ensure) async fn observe_held(
    agent: &ic_agent::Agent,
    expected: &InfrastructureBootstrapHeldSourceRecord,
) -> Result<InfrastructureBootstrapCustodyRecord, InfrastructureBootstrapError> {
    let observed = certified_custody::observe_one(agent, expected.custody.canister)
        .await
        .map_err(|error| InfrastructureBootstrapError::Observation(error.to_string()))?;
    let actual = InfrastructureBootstrapCustodyRecord {
        canister: observed.principal,
        subnet: observed.subnet,
        controllers: observed.controllers,
        module_sha256: observed
            .module_sha256
            .as_deref()
            .map(module_hash)
            .transpose()?
            .flatten(),
    };
    if actual != expected.custody || !actual.controllers.contains(&expected.root) {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    Ok(actual)
}

fn principal(value: &str) -> Result<Principal, InfrastructureBootstrapError> {
    Principal::from_text(value)
        .ok()
        .filter(|id| *id != Principal::anonymous() && *id != Principal::management_canister())
        .ok_or(InfrastructureBootstrapError::Integrity)
}

fn module_hash(value: &str) -> Result<Option<[u8; 32]>, InfrastructureBootstrapError> {
    if value == "empty" {
        return Ok(None);
    }
    decode_hex(value)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .map(Some)
        .ok_or(InfrastructureBootstrapError::Integrity)
}

/// Retain only the fields observable before current Root initialization.
pub(in crate::fleet_ensure) fn custody_from_binding(
    binding: &crate::fleet_ensure::model::capacity_import::CapacityImportSourceBinding,
) -> InfrastructureBootstrapCustodyRecord {
    InfrastructureBootstrapCustodyRecord {
        canister: binding.canister_id,
        subnet: binding.subnet,
        controllers: binding.controllers.clone(),
        module_sha256: binding.module_sha256,
    }
}

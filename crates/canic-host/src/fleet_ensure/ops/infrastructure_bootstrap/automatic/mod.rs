//! Compile disposable-estate declarations from current certified custody and durable samples.
//!
//! Completed journals are history. No installed predecessor interface is queried here.

use crate::{
    fleet_ensure::{
        model::{
            DesiredCanisterKind, DesiredFleet,
            capacity_import::survey::CapacityImportSampleRecord,
            infrastructure_bootstrap::{
                InfrastructureBootstrapCustodyRecord, InfrastructureBootstrapRecord,
                InfrastructureBootstrapSurveyRecord,
            },
        },
        ops::{
            EnsurePaths,
            capacity_import::admission::{
                CapacityImportDeclaration, CapacityImportDispositionKind,
            },
            certified_custody,
            infrastructure_bootstrap::{
                InfrastructureBootstrapError,
                declarations::{BootstrapDeclarations, HeldDeclaration, custody_from_binding},
            },
            read_current, write_current,
        },
    },
    icp::IcpCli,
};
use candid::Principal;
use canic_core::{cdk::utils::hash::hex_bytes, ids::CanonicalNetworkId};
use sha2_host::{Digest, Sha256};
use std::collections::BTreeMap;

/// Bind the exact current target before any status attempt can be charged.
pub(in crate::fleet_ensure) fn bind(
    paths: &EnsurePaths,
    desired: &DesiredFleet,
) -> Result<[u8; 32], InfrastructureBootstrapError> {
    let path = paths.plan.with_file_name("clean-reinstall-desired.json");
    match read_current::<DesiredFleet>(&path)? {
        Some(retained) if retained == *desired => {}
        Some(_) => return Err(InfrastructureBootstrapError::Integrity),
        None => write_current(&path, desired)?,
    }
    Ok(Sha256::digest(serde_json::to_vec(desired)?).into())
}

/// Return the completed current survey before resolving a signer or making another paid read.
pub(in crate::fleet_ensure) fn retained(
    paths: &EnsurePaths,
) -> Result<Option<InfrastructureBootstrapRecord>, InfrastructureBootstrapError> {
    let record = read_current::<InfrastructureBootstrapSurveyRecord>(
        &paths
            .plan
            .with_file_name("infrastructure-bootstrap-survey.json"),
    )?;
    Ok(record.and_then(|record| record.source))
}

/// Construct an agent bound to the selected operator and current network authority.
pub(in crate::fleet_ensure) fn agent(
    icp: &IcpCli,
    desired: &DesiredFleet,
) -> Result<ic_agent::Agent, InfrastructureBootstrapError> {
    let agent = icp
        .authenticated_agent_with_response_limit(2 * 1024 * 1024)
        .map_err(|error| InfrastructureBootstrapError::Observation(error.to_string()))?;
    if agent.get_principal().ok() != Principal::from_text(&desired.operator).ok()
        || CanonicalNetworkId::from_der_root_trust_anchor(&agent.read_root_key()).ok()
            != desired
                .bootstrap
                .as_ref()
                .map(|bootstrap| bootstrap.canonical_network_id)
    {
        return Err(InfrastructureBootstrapError::Integrity);
    }
    Ok(agent)
}

/// Read only certified physical fields, preserving the original selection across retries.
pub(in crate::fleet_ensure) async fn custody(
    paths: &EnsurePaths,
    desired: &DesiredFleet,
    agent: &ic_agent::Agent,
) -> Result<BTreeMap<String, InfrastructureBootstrapCustodyRecord>, InfrastructureBootstrapError> {
    let mut observed = BTreeMap::new();
    for configured in &desired.canisters {
        let id = configured
            .principal
            .as_deref()
            .and_then(|id| Principal::from_text(id).ok())
            .ok_or(InfrastructureBootstrapError::Integrity)?;
        let entry = certified_custody::observe_one(agent, id)
            .await
            .map_err(|error| InfrastructureBootstrapError::Observation(error.to_string()))?;
        let module_sha256 = entry
            .module_sha256
            .as_deref()
            .map(|hash| {
                canic_core::cdk::utils::hash::decode_hex(hash)
                    .ok()
                    .and_then(|hash| hash.try_into().ok())
                    .ok_or(InfrastructureBootstrapError::Integrity)
            })
            .transpose()?;
        if entry.subnet.into_principal().to_text() != configured.subnet {
            return Err(InfrastructureBootstrapError::Integrity);
        }
        observed.insert(
            configured.name.clone(),
            InfrastructureBootstrapCustodyRecord {
                canister: id,
                subnet: entry.subnet,
                controllers: entry.controllers,
                module_sha256,
            },
        );
    }
    let path = paths.plan.with_file_name("clean-reinstall-custody.json");
    match read_current::<BTreeMap<String, InfrastructureBootstrapCustodyRecord>>(&path)? {
        Some(retained) if retained == observed => {}
        Some(_) => return Err(InfrastructureBootstrapError::Integrity),
        None => write_current(&path, &observed)?,
    }
    Ok(observed)
}

/// Select only directly controlled infrastructure for management reads before replacing Root.
pub(in crate::fleet_ensure) fn direct_ids(
    desired: &DesiredFleet,
    observed: &BTreeMap<String, InfrastructureBootstrapCustodyRecord>,
) -> Result<Vec<Principal>, InfrastructureBootstrapError> {
    let operator = Principal::from_text(&desired.operator)
        .map_err(|_| InfrastructureBootstrapError::Integrity)?;
    desired
        .canisters
        .iter()
        .filter(|entry| entry.kind != DesiredCanisterKind::Pool)
        .map(|entry| {
            let source = observed
                .get(&entry.name)
                .ok_or(InfrastructureBootstrapError::Integrity)?;
            if !source.controllers.contains(&operator) {
                return Err(InfrastructureBootstrapError::Integrity);
            }
            Ok(source.canister)
        })
        .collect()
}

/// The explicit reinstall disposition binds current physical facts, never historical defaults.
pub(in crate::fleet_ensure) fn declarations(
    desired: &DesiredFleet,
    observed: &BTreeMap<String, InfrastructureBootstrapCustodyRecord>,
    samples: &[CapacityImportSampleRecord],
    root_key: &[u8],
) -> Result<String, InfrastructureBootstrapError> {
    let mut document = BootstrapDeclarations {
        schema_version: 1,
        operator: desired.operator.clone(),
        network_root_key_sha256: hex_bytes(Sha256::digest(root_key)),
        canisters: Vec::new(),
        root_owned: Vec::new(),
    };
    for configured in &desired.canisters {
        let custody = observed
            .get(&configured.name)
            .ok_or(InfrastructureBootstrapError::Integrity)?;
        let controllers = custody.controllers.iter().map(Principal::to_text).collect();
        let module_sha256 = custody
            .module_sha256
            .map_or_else(|| "empty".into(), hex_bytes);
        let evidence =
            "Explicit clean reinstall of selected disposable Fleet application and framework state"
                .to_string();
        if configured.kind == DesiredCanisterKind::Pool {
            document.root_owned.push(HeldDeclaration {
                canister: custody.canister.to_text(),
                subnet: custody.subnet.to_string(),
                controllers,
                module_sha256,
                disposition: CapacityImportDispositionKind::Retired,
                no_external_obligations: true,
                no_other_fleet_ownership: true,
                evidence,
            });
        } else {
            let sample = samples
                .iter()
                .find(|sample| sample.binding.canister_id == custody.canister)
                .ok_or(InfrastructureBootstrapError::Integrity)?;
            if custody_from_binding(&sample.binding) != *custody {
                return Err(InfrastructureBootstrapError::Integrity);
            }
            document.canisters.push(CapacityImportDeclaration {
                canister: custody.canister.to_text(),
                subnet: custody.subnet.to_string(),
                controllers,
                module_sha256,
                canister_version: sample.binding.canister_version,
                disposition: CapacityImportDispositionKind::Retired,
                no_external_obligations: true,
                no_other_fleet_ownership: true,
                evidence,
            });
        }
    }
    super::declarations::held_sources(desired, &document, [0; 32])?;
    toml::to_string(&document).map_err(|_| InfrastructureBootstrapError::Integrity)
}

/// One current Root management observation, after the workflow reserves its durable allowance.
pub(in crate::fleet_ensure) async fn observe_child(
    agent: &ic_agent::Agent,
    root: Principal,
    child: Principal,
) -> Result<
    CapacityImportSampleRecord,
    crate::fleet_ensure::ops::capacity_import::journal::CapacityImportJournalError,
> {
    crate::fleet_ensure::ops::capacity_import::admission::observer::management::observe_root_owned(
        agent, root, child,
    )
    .await
}

//! Retain the original supplied estate survey before initialization review or paid inspection.

use crate::{
    fleet_ensure::{
        model::{
            DesiredCanisterKind, DesiredFleet,
            capacity_import::survey::CapacityImportSampleRecord,
            infrastructure_bootstrap::{
                BootstrapCoordinatorSelection, InfrastructureBootstrapRecord,
                InfrastructureBootstrapSourceRecord, InfrastructureBootstrapSurveyRecord,
            },
        },
        ops::{
            self, EnsurePaths,
            capacity_import::admission::observer::{inventory, management},
            infrastructure_bootstrap::{
                InfrastructureBootstrapError, LedgerAccount, authenticated_agent, declarations,
                seal_sources,
            },
        },
    },
    icp::IcpCli,
};

use candid::{Nat, Principal};
use canic_core::cdk::utils::hash::{decode_hex, hex_bytes};
use sha2_host::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

use ic_host_fs::durable::{read_optional_regular_bytes_bounded, write_bytes};

const MAXIMUM_BYTES: usize = 2 * 1024 * 1024;

/// Immutable local selection; construction performs no network calls.
pub(in crate::fleet_ensure) struct BootstrapSurvey {
    record: InfrastructureBootstrapSurveyRecord,
    selected: BTreeMap<String, Principal>,
    original: InfrastructureBootstrapRecord,
}

impl BootstrapSurvey {
    /// Freeze request identity before any management attempt can consume cycles.
    /// The caller holds the Fleet lock throughout the survey.
    pub(in crate::fleet_ensure) fn begin(
        paths: &EnsurePaths,
        desired: &DesiredFleet,
        coordinator: BootstrapCoordinatorSelection,
        declarations_toml: &str,
    ) -> Result<Self, InfrastructureBootstrapError> {
        let invalid = || InfrastructureBootstrapError::Integrity;
        if declarations_toml.len() > 256 * 1024 {
            return Err(invalid());
        }
        let declarations: declarations::BootstrapDeclarations =
            toml::from_str(declarations_toml).map_err(|_| invalid())?;
        let operator = Principal::from_text(&desired.operator).map_err(|_| invalid())?;
        let network_root_key_sha256 = decode_hex(&declarations.network_root_key_sha256)
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or_else(invalid)?;
        let selected = selected(desired, coordinator)?;
        let declarations_sha256 = Sha256::digest(declarations_toml.as_bytes()).into();
        let held_sources = declarations::held_sources(desired, &declarations, declarations_sha256)?;
        let declared = declarations
            .canisters
            .iter()
            .map(|entry| {
                let binding = entry.binding().map_err(|_| invalid())?;
                if !entry.no_external_obligations
                    || !entry.no_other_fleet_ownership
                    || entry.evidence.trim().is_empty()
                    || !binding.controllers.contains(&operator)
                {
                    return Err(invalid());
                }
                Ok(binding.canister_id)
            })
            .chain(
                held_sources
                    .values()
                    .map(|source| Ok(source.custody.canister)),
            )
            .collect::<Result<BTreeSet<_>, _>>()?;
        if declarations.schema_version != 1
            || declarations.operator != desired.operator
            || declared.len() != declarations.canisters.len() + held_sources.len()
            || declared != selected.values().copied().collect()
        {
            return Err(invalid());
        }
        let mut hash = Sha256::new();
        hash.update(b"canic.infrastructure-bootstrap.survey.v1\0");
        hash.update(serde_json::to_vec(&(
            desired,
            coordinator,
            declarations_toml,
        ))?);
        let request_sha256 = hash.finalize().into();
        let record = retain_request(paths, desired, request_sha256)?;
        let original = InfrastructureBootstrapRecord {
            schema_version: 1,
            operator,
            network_root_key_sha256,
            coordinator,
            estate_seed: None,
            coordinator_registry_candid_hex: None,
            declarations_toml: declarations_toml.into(),
            declarations_sha256: Sha256::digest(declarations_toml.as_bytes()).into(),
            sources: BTreeMap::new(),
            held_sources,
            operator_cycles: 0,
            ledger_fee_cycles: 0,
            source_sha256: [0; 32],
        };
        if let Some(source) = &record.source {
            verify_retained_source(&original, source, &selected)?;
        }
        Ok(Self {
            record,
            selected,
            original,
        })
    }

    pub(in crate::fleet_ensure) const fn request_sha256(&self) -> [u8; 32] {
        self.record.request_sha256
    }

    pub(in crate::fleet_ensure) const fn completed(
        &self,
    ) -> Option<&InfrastructureBootstrapRecord> {
        self.record.source.as_ref()
    }

    pub(in crate::fleet_ensure) fn canisters(&self) -> Vec<Principal> {
        self.selected
            .iter()
            .filter(|(name, _)| !self.original.held_sources.contains_key(*name))
            .map(|(_, id)| *id)
            .collect()
    }

    pub(in crate::fleet_ensure) fn agent(
        &self,
        icp: &IcpCli,
        desired: &DesiredFleet,
    ) -> Result<ic_agent::Agent, InfrastructureBootstrapError> {
        authenticated_agent(icp, desired, &self.original)
    }

    /// Check custody before workflow reserves the corresponding durable paid-read allowance.
    pub(in crate::fleet_ensure) async fn prepare_sample(
        agent: &ic_agent::Agent,
        canister: Principal,
    ) -> Result<management::PreparedManagementObservation, InfrastructureBootstrapError> {
        Ok(management::prepare(agent, canister).await?)
    }

    /// Retain the original balances and exact operator declarations without rebasing samples.
    pub(in crate::fleet_ensure) async fn finish(
        mut self,
        paths: &EnsurePaths,
        desired: &DesiredFleet,
        agent: &ic_agent::Agent,
        samples: Vec<CapacityImportSampleRecord>,
    ) -> Result<InfrastructureBootstrapRecord, InfrastructureBootstrapError> {
        let invalid = || InfrastructureBootstrapError::Integrity;
        let declarations: declarations::BootstrapDeclarations =
            toml::from_str(&self.original.declarations_toml).map_err(|_| invalid())?;
        if samples.len() != self.selected.len() - self.original.held_sources.len() {
            return Err(invalid());
        }
        for (name, id) in &self.selected {
            if let Some(source) = self.original.held_sources.get(name) {
                declarations::observe_held(agent, source).await?;
                continue;
            }
            let sample = samples
                .iter()
                .find(|sample| sample.binding.canister_id == *id)
                .ok_or_else(invalid)?;
            let declaration = declarations
                .canisters
                .iter()
                .find(|entry| entry.canister == id.to_text())
                .ok_or_else(invalid)?;
            if sample.binding.snapshots_size_bytes != 0 {
                return Err(invalid());
            }
            self.original.sources.insert(
                name.clone(),
                InfrastructureBootstrapSourceRecord {
                    sample: sample.clone(),
                    disposition: declaration.disposition(self.original.declarations_sha256),
                },
            );
        }
        if self.original.coordinator == BootstrapCoordinatorSelection::Ready {
            let name = &desired.bootstrap.as_ref().ok_or_else(invalid)?.coordinator;
            let registry =
                inventory::registry(agent, *self.selected.get(name).ok_or_else(invalid)?).await?;
            self.original.coordinator_registry_candid_hex = Some(hex_bytes(
                candid::encode_one(registry).map_err(|_| invalid())?,
            ));
        }
        let source = seal_sources(self.original)?;
        self.record.source = Some(source.clone());
        let bytes = serde_json::to_vec_pretty(&self.record)?;
        if bytes.len() > MAXIMUM_BYTES {
            return Err(invalid());
        }
        write_bytes(
            &paths
                .plan
                .with_file_name("infrastructure-bootstrap-survey.json"),
            &bytes,
        )?;
        Ok(source)
    }

    pub(in crate::fleet_ensure) fn ledger(
        &mut self,
        desired: &DesiredFleet,
        icp: &IcpCli,
    ) -> Result<(), InfrastructureBootstrapError> {
        let invalid = |error: crate::icp::IcpCandidCallError| {
            InfrastructureBootstrapError::Observation(error.to_string())
        };
        let fee: Nat = icp
            .canister_query_candid(&desired.cycles_ledger, "icrc1_fee", &(), None)
            .map_err(invalid)?;
        let balance: Nat = icp
            .canister_query_candid(
                &desired.cycles_ledger,
                "icrc1_balance_of",
                &LedgerAccount {
                    owner: self.original.operator,
                    subaccount: None,
                },
                None,
            )
            .map_err(invalid)?;
        self.original.ledger_fee_cycles = fee
            .0
            .try_into()
            .map_err(|_| InfrastructureBootstrapError::Integrity)?;
        self.original.operator_cycles = balance
            .0
            .try_into()
            .map_err(|_| InfrastructureBootstrapError::Integrity)?;
        Ok(())
    }
}

fn selected(
    desired: &DesiredFleet,
    coordinator: BootstrapCoordinatorSelection,
) -> Result<BTreeMap<String, Principal>, InfrastructureBootstrapError> {
    let invalid = || InfrastructureBootstrapError::Integrity;
    let mut selected = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for configured in &desired.canisters {
        if configured.kind == DesiredCanisterKind::Coordinator
            && coordinator == BootstrapCoordinatorSelection::Create
        {
            if configured.principal.is_some() {
                return Err(invalid());
            }
            continue;
        }
        let id = configured
            .principal
            .as_deref()
            .and_then(|id| Principal::from_text(id).ok())
            .ok_or_else(invalid)?;
        if id == Principal::anonymous()
            || id == Principal::management_canister()
            || !ids.insert(id)
            || selected.insert(configured.name.clone(), id).is_some()
        {
            return Err(invalid());
        }
    }
    Ok(selected)
}

fn retain_request(
    paths: &EnsurePaths,
    desired: &DesiredFleet,
    request_sha256: [u8; 32],
) -> Result<InfrastructureBootstrapSurveyRecord, InfrastructureBootstrapError> {
    let invalid = || InfrastructureBootstrapError::Integrity;
    let path = paths
        .plan
        .with_file_name("infrastructure-bootstrap-survey.json");
    if let Some(bytes) =
        read_optional_regular_bytes_bounded(&path, MAXIMUM_BYTES).map_err(|_| invalid())?
    {
        let record: InfrastructureBootstrapSurveyRecord = serde_json::from_slice(&bytes)?;
        if record.schema_version != 1 || record.request_sha256 != request_sha256 {
            return Err(invalid());
        }
        return Ok(record);
    }
    let state = ops::read_state(paths, &desired.fleet)?;
    if ops::read_plan(paths)?.is_some()
        || ops::read_journal(paths)?.is_some()
        || !state.principals.is_empty()
        || !state.pending_principals.is_empty()
        || !state.topology.is_empty()
        || state.active_registry.is_some()
    {
        return Err(invalid());
    }
    let record = InfrastructureBootstrapSurveyRecord {
        schema_version: 1,
        request_sha256,
        source: None,
    };
    write_bytes(&path, &serde_json::to_vec_pretty(&record)?)?;
    Ok(record)
}

fn verify_retained_source(
    original: &InfrastructureBootstrapRecord,
    source: &InfrastructureBootstrapRecord,
    selected: &BTreeMap<String, Principal>,
) -> Result<(), InfrastructureBootstrapError> {
    let invalid = || InfrastructureBootstrapError::Integrity;
    let expected = (
        &original.operator,
        &original.network_root_key_sha256,
        original.coordinator,
        &original.declarations_toml,
    );
    let actual = (
        &source.operator,
        &source.network_root_key_sha256,
        source.coordinator,
        &source.declarations_toml,
    );
    let identities = source
        .sources
        .iter()
        .map(|(name, value)| (name.clone(), value.sample.binding.canister_id))
        .chain(
            source
                .held_sources
                .iter()
                .map(|(name, value)| (name.clone(), value.custody.canister)),
        )
        .collect::<BTreeMap<_, _>>();
    if expected != actual || &identities != selected || seal_sources(source.clone())? != *source {
        return Err(invalid());
    }
    Ok(())
}

//! Reviewed physical infrastructure and initialization selection for a new Fleet.
//!
//! These records preserve observations; they never authorize replacement of a supplied ID.

pub mod registration_recovery;

use crate::fleet_ensure::model::capacity_import::{
    CapacityImportDisposition, survey::CapacityImportSampleRecord,
};
use candid::Principal;
use canic_contracts::ids::SubnetId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Durable attempts per whole-estate inspection boundary.
pub(in crate::fleet_ensure) const BOOTSTRAP_PHASE_INSPECTION_ROUNDS: u32 = 2;
/// Durable attempts per preparation/reconciliation boundary.
pub(in crate::fleet_ensure) const BOOTSTRAP_EFFECT_INSPECTION_ROUNDS: u32 = 8;
/// Preparation, target balance and destination balance within one effect round.
pub(in crate::fleet_ensure) const BOOTSTRAP_EFFECT_OBSERVATIONS_PER_ROUND: u32 = 3;

///
/// InfrastructureBootstrapFundingTarget
///
/// Model projection of a physical owner and native floor for the readiness forecast.
///

pub(in crate::fleet_ensure) struct InfrastructureBootstrapFundingTarget {
    pub name: String,
    pub principal: String,
    pub controllers: Vec<String>,
    pub minimum_cycles: u128,
    pub observation_burn_cycles: u128,
    pub update_burn_cycles: u128,
}

/// Explicit Coordinator prerequisite, independent of Root-local capacity import.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapCoordinatorSelection {
    Create,
    Initialize,
    Ready,
}

/// Frozen survey identity and original completed samples, before an initialization plan exists.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InfrastructureBootstrapSurveyRecord {
    pub schema_version: u16,
    pub request_sha256: [u8; 32],
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub source: Option<InfrastructureBootstrapRecord>,
}

/// Original supplied custody and disposition retained before infrastructure effects.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InfrastructureBootstrapSourceRecord {
    pub sample: CapacityImportSampleRecord,
    pub disposition: CapacityImportDisposition,
}

/// Certified physical custody without inventing a management version or cycle balance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InfrastructureBootstrapCustodyRecord {
    pub canister: Principal,
    pub subnet: SubnetId,
    pub controllers: Vec<Principal>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub module_sha256: Option<[u8; 32]>,
}

/// A child remains controlled by its exact Root while current infrastructure is installed.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InfrastructureBootstrapHeldSourceRecord {
    pub root: Principal,
    pub custody: InfrastructureBootstrapCustodyRecord,
    pub disposition: CapacityImportDisposition,
}

/// Exact no-replacement initialization authority sealed by the enclosing Ensure plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InfrastructureBootstrapRecord {
    pub schema_version: u16,
    pub operator: Principal,
    pub network_root_key_sha256: [u8; 32],
    pub coordinator: BootstrapCoordinatorSelection,
    /// Exact operator seed to publish after setup; creation resolves only its Coordinator slot.
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub estate_seed: Option<InfrastructureBootstrapSeedRecord>,
    /// Ready selection requires an independently verified current Registry witness.
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub coordinator_registry_candid_hex: Option<String>,
    /// Original operator-owned TOML; its exact bytes bind every source disposition.
    pub declarations_toml: String,
    pub declarations_sha256: [u8; 32],
    pub sources: BTreeMap<String, InfrastructureBootstrapSourceRecord>,
    pub held_sources: BTreeMap<String, InfrastructureBootstrapHeldSourceRecord>,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub operator_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub ledger_fee_cycles: u128,
    /// Digest of the original survey and dispositions, also held by each new Root.
    pub source_sha256: [u8; 32],
}

/// Original seed bytes are part of the reviewed initialization authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InfrastructureBootstrapSeedRecord {
    pub relative_path: String,
    pub original: String,
    pub before_sha256: [u8; 32],
}

/// Intent and completion of the single local identity publication after receipted setup.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InfrastructureBootstrapPublicationRecord {
    pub schema_version: u16,
    pub plan: crate::fleet_ensure::model::FleetEnsurePlan,
    pub replacement: String,
    pub after_sha256: [u8; 32],
    pub terminal_receipt_sha256: [u8; 32],
    pub coordinator: Principal,
    pub completed: bool,
}

/// Terminal phase receipt, including reserved balances, retained before local completion publication.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InfrastructureBootstrapTerminalRecord {
    pub schema_version: u16,
    pub plan_sha256: String,
    pub journal_sha256: [u8; 32],
    pub state_sha256: [u8; 32],
    pub actual: crate::fleet_ensure::model::ActualCycleConservation,
    pub canisters: BTreeMap<String, CapacityImportSampleRecord>,
}

/// Persisted inspection allowances for the reviewed initialization phase.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InfrastructureBootstrapInspectionRecord {
    pub attempt_recoveries:
        Vec<crate::fleet_ensure::model::attempt_recovery::AttemptRecoveryGrantRecord>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub registration_recovery_sha256: Option<String>,
    pub schema_version: u16,
    pub source_sha256: [u8; 32],
    pub planned_at_time: u64,
    pub plan_sha256: String,
    pub review_attempts: u32,
    pub apply_attempts: u32,
    pub terminal_attempts: u32,
    pub registration_attempts: u32,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub registration_plan_sha256: Option<String>,
    pub effect_observations: BTreeMap<String, u32>,
}

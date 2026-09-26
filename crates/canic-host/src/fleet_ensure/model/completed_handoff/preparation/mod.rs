//! Current preparation authority for a completed source, separate from old executable plans.

use crate::fleet_ensure::model::{
    EnsureAction, FleetTerminalSourceRecord, completed_handoff::CompletedEstateCustodyRecord,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A separately reviewed, bounded set of authority seals; no wipe or payment is authorized.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedPreparationReviewRecord {
    pub schema_version: u16,
    pub cli_release: String,
    pub environment: String,
    pub fleet: String,
    pub operation_id: String,
    pub source: FleetTerminalSourceRecord,
    pub custody: CompletedEstateCustodyRecord,
    pub source_artifacts: BTreeMap<String, String>,
    pub actions: Vec<EnsureAction>,
    /// Every retained physical ID and its inspection owner; None means direct operator custody.
    pub inspection_roots: BTreeMap<String, Option<String>>,
    pub source_accounting: CompletedSourceAccountingRecord,
    pub maximum_attempts_per_action: u32,
    pub maximum_observations_per_action: u32,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub maximum_execution_burn_cycles: u128,
    pub review_sha256: String,
}

/// A consumed seal submission and its exact completion evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedPreparationEffectRecord {
    pub action_sha256: String,
    pub submission_attempts: u32,
    pub observation_attempts: u32,
    pub applied: bool,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub before: Option<CompletedPreparationBalanceRecord>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub after: Option<CompletedPreparationBalanceRecord>,
}

/// Intent is retained before the first seal; each submission consumes its reviewed allowance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedPreparationJournalRecord {
    pub schema_version: u16,
    pub review_sha256: String,
    pub effects: Vec<CompletedPreparationEffectRecord>,
    pub inspections: BTreeMap<String, CompletedPreparationInspectionRecord>,
    pub prepared: bool,
}

/// Original receipt baseline is retained independently of newly sampled balances.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedSourceAccountingRecord {
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub initial_native_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub recorded_funding_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub maximum_source_burn_cycles: u128,
}

/// A bounded management observation; consumed attempts survive an interrupted reply.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedPreparationInspectionRecord {
    pub attempts: u32,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub balance: Option<CompletedPreparationBalanceRecord>,
}

/// Native and reserved balances remain distinct; reserved funds cannot fund preparation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedPreparationBalanceRecord {
    pub status: crate::fleet_ensure::model::CanisterRuntimeStatus,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub native_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub reserved_cycles: u128,
}

//! Explicit registration funding and budget authority within an unfinished bootstrap operation.

use crate::fleet_ensure::model::{
    EnsureAction, capacity_import::survey::CapacityImportSampleRecord,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Fixed additional inspection rounds; retrying a review never replenishes them.
pub(in crate::fleet_ensure) const RECOVERY_INSPECTION_ROUNDS: u32 = 2;

/// One retained recovery request, including observations consumed before a review exists.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapRegistrationRecoveryRecord {
    pub review_attempts: u32,
    pub approval_attempts: u32,
    pub approved: bool,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub review: Option<BootstrapRegistrationReviewRecord>,
}

/// Immutable supplementary authority. The original plan and applied effects remain unchanged.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapRegistrationReviewRecord {
    pub schema_version: u16,
    pub plan_sha256: String,
    pub operation_id: String,
    pub applied_effects_sha256: String,
    pub created_at_time: u64,
    pub canisters: BTreeMap<String, CapacityImportSampleRecord>,
    pub protocol_actions: Vec<EnsureAction>,
    pub funding_actions: Vec<EnsureAction>,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub observed_execution_debit_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub successor_burn_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub recovery_burn_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub maximum_execution_burn_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub additional_funding_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub additional_ledger_fee_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub operator_cycles: u128,
    pub review_sha256: String,
}

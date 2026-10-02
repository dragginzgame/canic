//! Durable bounded management observations used to construct an initial import review.

use crate::fleet_ensure::model::capacity_import::CapacityImportSourceBinding;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// First successful physical sample; restarting planning never rebases it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportSampleRecord {
    pub binding: CapacityImportSourceBinding,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub reserved_cycles: u128,
}

/// One canister's inspection allowance, reserved durably before issuing a request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportSurveyCanisterRecord {
    pub attempts: u32,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub sample: Option<CapacityImportSampleRecord>,
}

/// Exact initial survey inputs and monotonic per-canister inspection progress.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportSurveyRecord {
    pub attempt_recoveries:
        Vec<crate::fleet_ensure::model::attempt_recovery::AttemptRecoveryGrantRecord>,
    pub schema_version: u16,
    pub request_sha256: [u8; 32],
    pub canisters: BTreeMap<String, CapacityImportSurveyCanisterRecord>,
}

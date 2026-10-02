//! Reviewed physical custody and debit bounds for whole-Fleet release to capacity.
//!
//! A review is not an execution journal or proof of live quiescence. Existing Host
//! effect ownership must retain and authenticate the observations before any reset.

use crate::fleet_ensure::model::capacity_import::CapacityImportSourceBinding;
use candid::Principal;
use canic_core::ids::FleetBinding;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Exact current-build authority for one whole-Fleet disposal review.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetReleaseAuthority {
    pub fleet: FleetBinding,
    pub network_root_key_sha256: [u8; 32],
    pub release_build_sha256: [u8; 32],
    pub operation_id: [u8; 32],
    pub operator: Principal,
    pub coordinator: Principal,
    pub cycles_ledger: Principal,
}

/// Former role and exact Root ownership; all selected physical identities survive.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetReleaseRole {
    Coordinator,
    Root,
    Store { root: Principal },
    Child { root: Principal },
}

/// Selected final custody, separate from temporary operator custody during reset.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetReleaseDestination {
    OperatorHeld,
    IndependentPool { root: Principal },
}

/// Reviewed physical binding, destructive disposition and native/reserved balances.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetReleaseSourceRecord {
    pub binding: CapacityImportSourceBinding,
    pub snapshots: Vec<Vec<u8>>,
    pub role: FleetReleaseRole,
    pub destination: FleetReleaseDestination,
    /// Hash of retained explicit state-discard and external-obligation evidence.
    pub disposition_sha256: [u8; 32],
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub observed_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub observed_reserved_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub minimum_retained_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub maximum_debit_cycles: u128,
    /// Reviewed worst-case charge for one paid management call.
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub maximum_call_debit_cycles: u128,
    pub maximum_paid_calls: u32,
}

/// Explicit continued custody of an external account after its owner's code is cleared.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetReleaseAccountRecord {
    pub ledger: Principal,
    pub owner: Principal,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub subaccount: Option<[u8; 32]>,
    /// Qualified current-build artifact retaining the ability to operate this account.
    pub recovery_artifact_sha256: [u8; 32],
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub observed_balance: u128,
}

/// Immutable review material; no field permits deleting or substituting a selected ID.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetReleaseReviewRecord {
    pub schema_version: u16,
    pub authority: FleetReleaseAuthority,
    pub sources: Vec<FleetReleaseSourceRecord>,
    pub accounts: Vec<FleetReleaseAccountRecord>,
    pub review_sha256: [u8; 32],
}

/// Release authority and spent read allowances inside the ordinary Fleet journal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetReleaseExecutionRecord {
    /// Executable operation identity remains separate from the release review digest.
    pub plan_sha256: String,
    pub review: FleetReleaseReviewRecord,
    /// Includes failed requests and uncertain replies; successful reads never refund it.
    pub reserved_paid_calls: BTreeMap<String, u32>,
}

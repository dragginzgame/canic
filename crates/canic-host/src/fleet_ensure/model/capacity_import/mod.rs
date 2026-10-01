//! Exact reviewed authority and cycle bounds for enrolling supplied Fleet capacity.
//! These records carry no permission to create replacement canisters.

pub mod admission;
pub mod funding;
pub mod operation;
pub mod retirement;
pub mod survey;

use candid::Principal;
use canic_core::ids::{FleetBinding, SubnetId};
use serde::{Deserialize, Serialize};

/// Source debit used by clean reinstall and its advance headroom forecast.
pub const DEFAULT_IMPORT_SOURCE_DEBIT_CYCLES: u128 = 100_000_000_000;

/// Exact supplied capacity retained in reviewed Root initialization before any handoff.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportBootstrapRecord {
    pub review_sha256: [u8; 32],
    pub operator: Principal,
    pub sources: Vec<Principal>,
}

/// Destination and signer identity frozen by one capacity review.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportAuthority {
    pub fleet: FleetBinding,
    pub network_root_key_sha256: [u8; 32],
    pub operator: Principal,
    pub coordinator: Principal,
    pub root: Principal,
    pub subnet: SubnetId,
    pub root_authority_sha256: [u8; 32],
    pub import_sequence: u64,
    pub recovery_controllers: Vec<Principal>,
}

/// Exact management identity whose change invalidates an unissued handoff.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportSourceBinding {
    pub canister_id: Principal,
    pub subnet: SubnetId,
    pub controllers: Vec<Principal>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub module_sha256: Option<[u8; 32]>,
    pub canister_version: u64,
    pub stopped: bool,
    pub snapshots_size_bytes: u64,
}

/// Reviewed disposition of obligations erased with a candidate's code and state.
/// The digest identifies retained evidence; it does not authenticate that evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum CapacityImportDisposition {
    AbsenceEvidence { evidence_sha256: [u8; 32] },
    Retired { evidence_sha256: [u8; 32] },
}

/// One physical source and its complete retained-balance equation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportSourceRecord {
    pub binding: CapacityImportSourceBinding,
    pub disposition: CapacityImportDisposition,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub observed_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub observed_reserved_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub minimum_ready_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub maximum_debit_cycles: u128,
}

/// Immutable current-contract review of supplied physical capacity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportPlanRecord {
    /// Explicit supplementary credits; empty keeps an existing uncredited review's digest intact.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub funding_credits: Vec<funding::CapacityImportFundingCreditRecord>,
    pub schema_version: u16,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub admission: Option<admission::CapacityImportAdmissionRecord>,
    pub authority: CapacityImportAuthority,
    pub sources: Vec<CapacityImportSourceRecord>,
    pub transitional_controllers: Vec<Principal>,
    pub final_controllers: Vec<Principal>,
    /// Source costs are separate from Root's paid observation and reset costs.
    pub root_budget: CapacityImportRootBudget,
    pub plan_sha256: [u8; 32],
}

/// One supplied source's controller intent, retained independently of other sources.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportHandoffRecord {
    pub canister_id: Principal,
    /// Authenticated terminal status remains attached to its original signed ingress.
    pub retirements: Vec<retirement::CapacityImportHandoffRetirementRecord>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub effect: Option<crate::fleet_ensure::model::EffectRecord>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub request: Option<CapacityImportHandoffRequestRecord>,
    #[serde(with = "crate::fleet_ensure::model::option_u128_text")]
    pub before_reserved_cycles: Option<u128>,
    #[serde(with = "crate::fleet_ensure::model::option_u128_text")]
    pub after_reserved_cycles: Option<u128>,
}

/// Exact local handoff journal under the existing Fleet operation lock.
/// Root reset receipts and inventory publication are separate completion prerequisites.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportJournalRecord {
    pub plan: CapacityImportPlanRecord,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub operation: Option<operation::CapacityImportOperationRecord>,
    /// Explicit approval of this digest never authorizes another source or allowance.
    pub approved: bool,
    /// Authenticated Root reservation evidence is retained before any host handoff intent.
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub reservation: Option<CapacityImportReservationRecord>,
    pub handoffs: Vec<CapacityImportHandoffRecord>,
}

/// Protected Root evidence that allocation is fenced for this exact reviewed import.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportReservationRecord {
    pub plan_sha256: [u8; 32],
    pub authority: CapacityImportAuthority,
    pub sources: Vec<Principal>,
}

/// Original Root balance and the complete paid-call allowance bound into review.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportRootBudget {
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub observed_reserved_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub observed_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub minimum_retained_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub maximum_debit_cycles: u128,
    pub maximum_paid_calls: u32,
}

/// Exact signed ingress; resubmission cannot create a second controller update.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityImportHandoffRequestRecord {
    pub request_id: [u8; 32],
    pub ingress_expiry: u64,
    pub signed_envelope_hex: String,
}

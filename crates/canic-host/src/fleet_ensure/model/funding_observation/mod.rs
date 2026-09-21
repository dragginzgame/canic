//! Module: fleet_ensure::model::funding_observation
//!
//! Responsibility: retain exact observation approval and consumed single-pass attempts.
//! Boundary: a missing reply never restores an attempt; these records do not issue calls.

use candid::Principal;
use canic_core::ids::{
    CanisterRole, ComponentBinding, ComponentInstanceId, FleetRegistryAuthority,
    FleetSubnetRootReleaseSet,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Which bounded observation-derived requirement a native credit restores.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FundingQuoteStage {
    Observation,
    Recovery,
}

/// Exact reviewed evidence and stage bound into a separately approved native credit.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FundingQuoteSourceRecord {
    pub review_sha256: String,
    pub stage: FundingQuoteStage,
}

/// Exact independently observed registry and Component heads sealed by one review.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FundingObservationAuthorityRecord {
    pub registry: FleetRegistryAuthority,
    pub revision: u64,
    pub content_hash: [u8; 32],
    pub components: BTreeMap<ComponentInstanceId, FundingComponentHeadRecord>,
}

/// One Component's exact directory revision, independent of the Fleet registry head.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FundingComponentHeadRecord {
    pub revision: u64,
    pub content_hash: [u8; 32],
}

/// One exact child balance and parent-local ledger observation; physical pool custody supplies no funding edge.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FundingObservationRequestRecord {
    pub component: ComponentBinding,
    pub release_set: FleetSubnetRootReleaseSet,
    pub parent: Principal,
    #[serde(deserialize_with = "super::serialization::required_option")]
    pub parent_role: Option<CanisterRole>,
    pub child: Principal,
    pub role: CanisterRole,
}

/// Immutable review input; only this body participates in the approval digest.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FundingObservationReviewBodyRecord {
    /// Selected configuration source, recompiled against the plan's Spec hashes on restore.
    pub configuration_source: String,
    pub operation_id: String,
    pub plan_sha256: String,
    pub root: String,
    pub authority: FundingObservationAuthorityRecord,
    pub requests: Vec<FundingObservationRequestRecord>,
    #[serde(with = "super::u128_text")]
    pub per_attempt_cycles: u128,
    #[serde(with = "super::u128_text")]
    pub maximum_cycles: u128,
    #[serde(with = "super::u128_text")]
    pub recovery_floor_cycles: u128,
}

/// One bounded observation pass within the existing Ensure journal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FundingObservationReviewRecord {
    #[serde(with = "super::option_u128_text")]
    pub final_root_cycles: Option<u128>,
    pub body: FundingObservationReviewBodyRecord,
    pub review_sha256: String,
    pub approved: bool,
    /// Prefix of consumed requests, persisted before their calls. None means reply unresolved.
    pub attempts: Vec<FundingObservationAttemptRecord>,
}

/// A consumed request index and its retained response, if one reached durable storage.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FundingObservationAttemptRecord {
    pub request_index: usize,
    #[serde(deserialize_with = "super::serialization::required_option")]
    pub outcome: Option<FundingObservationOutcomeRecord>,
}

/// Retained ledger evidence is diagnostic until the complete observation is requalified.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FundingObservationOutcomeRecord {
    Observed(FundingChildAccountingRecord),
    ObservationFailed,
    Interrupted,
    AuthorityChanged,
    RecoveryRequired,
}

/// Parent-local accounting with exact participants, preserving unknown reservations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FundingChildAccountingRecord {
    #[serde(with = "super::u128_text")]
    pub native_cycles: u128,
    pub parent: Principal,
    pub child: Principal,
    pub observed_at_ns: u64,
    #[serde(with = "super::u128_text")]
    pub accounted_cycles: u128,
    pub last_accounted_at_secs: u64,
    pub pending_operations: u32,
    #[serde(with = "super::option_u128_text")]
    pub reserved_cycles: Option<u128>,
}

/// Invalid review state or an effect that cannot be admitted under the retained approval.
#[derive(Clone, Copy, Debug, Eq, thiserror::Error, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FundingObservationError {
    #[error("funding observation authority or retained record differs")]
    AuthorityMismatch,
    #[error("funding observation approval digest differs")]
    ApprovalMismatch,
    #[error("funding observation requires an in-progress operation")]
    OperationNotInProgress,
    #[error("funding observation is unavailable or has invalid bounds")]
    Unavailable,
    #[error("funding observation requires native recovery before another attempt")]
    Underfunded,
    #[error("funding observation accounting overflowed")]
    ArithmeticOverflow,
}

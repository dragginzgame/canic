//! Module: fleet_ensure::model::completed_handoff
//!
//! Responsibility: bind the reviewed local replacement of completed-estate evidence.
//! Does not own: live admission, effects, historical execution or persistence.
//! Boundary: source and replacement documents have separate immutable byte identities.

pub mod preparation;

use crate::fleet_ensure::model::FleetTerminalSourceRecord;
use candid::Principal;
use canic_core::ids::{CanonicalNetworkId, SubnetId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bounded final accounting sweeps, including a lost response or interrupted local publication.
pub const COMPLETED_RESET_MAXIMUM_TERMINAL_OBSERVATIONS: u32 = 2;

/// Exact archive/removal intent for consumed local approvals; excludes the active Fleet operation.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::fleet_ensure) struct CompletedAuthorityRetirementRecord {
    pub schema_version: u16,
    pub review_sha256: String,
    pub files: BTreeMap<String, String>,
}

/// Current reset authority derived from a separately completed source preparation.
/// Historical physical facts never become an executable predecessor desired document.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedEstateResetRecord {
    pub maximum_terminal_observations: u32,
    pub preparation: preparation::CompletedPreparationReviewRecord,
    pub prepared: preparation::CompletedPreparationJournalRecord,
    /// Fresh logical name to immutable source evidence name, bound by physical Principal.
    pub source_names: std::collections::BTreeMap<String, String>,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub operator_cycles: u128,
    #[serde(with = "crate::fleet_ensure::model::u128_text_map")]
    pub root_ledger_cycles: std::collections::BTreeMap<String, u128>,
    #[serde(with = "crate::fleet_ensure::model::u128_text_map")]
    pub other_ledger_cycles: std::collections::BTreeMap<String, u128>,
}

/// Durable intent retained before a paid final accounting sweep.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedResetAccountingIntentRecord {
    pub schema_version: u16,
    pub operation_id: String,
    pub plan_sha256: String,
    pub attempts: u32,
}

/// Terminal conservation receipt bound to the exact completed current journal and state bytes.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedResetTerminalRecord {
    pub schema_version: u16,
    pub operation_id: String,
    pub plan_sha256: String,
    pub journal_document_sha256: String,
    pub state_document_sha256: String,
    pub actual: crate::fleet_ensure::model::ActualCycleConservation,
    pub balances: CompletedResetBalancesRecord,
    pub receipt_sha256: String,
}

/// Every observed retained cycle domain after the final reset effects.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedResetBalancesRecord {
    pub canisters:
        std::collections::BTreeMap<String, preparation::CompletedPreparationBalanceRecord>,
    #[serde(with = "crate::fleet_ensure::model::u128_text_map")]
    pub ledger: std::collections::BTreeMap<String, u128>,
    #[serde(with = "crate::fleet_ensure::model::u128_text")]
    pub operator_cycles: u128,
}

/// Exact observed physical authority selected for local handoff review.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedPhysicalBindingRecord {
    pub principal: Principal,
    pub subnet: SubnetId,
    pub controllers: Vec<Principal>,
    #[serde(deserialize_with = "crate::fleet_ensure::model::serialization::required_option")]
    pub module_sha256: Option<String>,
}

/// Certificate tree identity retained alongside its decoded physical authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedCanisterCustodyRecord {
    pub binding: CompletedPhysicalBindingRecord,
    pub certificate_tree_sha256: [u8; 32],
}

/// Certified observations bound into the publication digest; no balance or fence authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedEstateCustodyRecord {
    pub network: CanonicalNetworkId,
    pub operator: Principal,
    pub canisters: BTreeMap<String, CompletedCanisterCustodyRecord>,
}

/// Exact current documents to publish together after completed-source admission.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedEstateDocumentsRecord {
    pub plan_sha256: String,
    pub journal_sha256: String,
    pub state_sha256: String,
}

/// Separate review of local publication; this grants no remote reset or payment authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedEstatePublicationReviewRecord {
    pub schema_version: u16,
    pub environment: String,
    pub fleet: String,
    pub operation_id: String,
    pub plan_sha256: String,
    pub source: FleetTerminalSourceRecord,
    pub custody: CompletedEstateCustodyRecord,
    pub replacement: CompletedEstateDocumentsRecord,
    /// Workspace-relative source interface/finalization paths and their exact archived bytes.
    pub source_artifacts: BTreeMap<String, String>,
    pub review_sha256: String,
}

/// Intent is durable before replacing any active document; completed replay preserves progress.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedEstatePublicationRecord {
    pub schema_version: u16,
    pub review_sha256: String,
    pub review_document_sha256: String,
    pub complete: bool,
}

//! Fleet admission view.
//!
//! Host owns authority and operations; command parsing and rendering belong to CLI.

use ic_platform::principal::ExternalPrincipal as Principal;
use serde::Serialize;

/// One Root's compact convergence report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AdmissionRootReport {
    pub root: Principal,
    pub active_generation: u64,
    pub active_policy_digest: String,
    pub operation_id: Option<String>,
    pub phase: Option<String>,
    pub participant_count: u64,
    pub pending_count: u64,
    pub prepared_count: u64,
    pub activated_count: u64,
    pub open_count: u64,
    pub first_unresolved: Option<Principal>,
}

/// Protected admission health shared by status and Medic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AdmissionStatusReport {
    pub fleet: String,
    pub environment: String,
    pub coordinator: Principal,
    pub registry_revision: u64,
    pub generation: u64,
    pub policy_digest: String,
    pub fleet_principal_count: u64,
    pub narrower_rule_count: u16,
    pub narrower_principal_reference_count: u16,
    pub current_operation: Option<String>,
    pub current_phase: Option<String>,
    pub last_operation: Option<String>,
    pub last_phase: Option<String>,
    pub roots: Vec<AdmissionRootReport>,
}

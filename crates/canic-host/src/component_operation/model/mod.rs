//! Durable authority and progress for one host-owned Component invocation.
//! Root storage remains the authoritative allocation and activation owner.

use candid::Principal;
use canic_core::ids::{
    CanisterRole, ComponentBinding, ComponentInstanceId, ComponentSpecId, FleetSubnetRootBinding,
    FleetSubnetRootReleaseSet,
};
use serde::{Deserialize, Serialize};

/// Exact reviewed local, installation and placement authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentAuthorityRecord {
    pub environment: String,
    pub fleet: String,
    pub root_name: String,
    /// Historical review provenance; live authority can survive an unchanged-Fleet replan.
    pub source_plan_sha256: String,
    pub binding: FleetSubnetRootBinding,
    pub release_set: FleetSubnetRootReleaseSet,
    pub root_module_sha256: String,
    pub root_candid_sha256: [u8; 32],
    pub root_controllers: Vec<String>,
    pub registry_sha256: [u8; 32],
    pub operator: Principal,
    pub component_spec: ComponentSpecId,
    pub spec_hash: [u8; 32],
    pub role: CanisterRole,
}

/// Immutable review for one local name and one Root operation identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentPlanRecord {
    pub schema_version: u16,
    pub name: String,
    pub operation_id: [u8; 32],
    pub authority: ComponentAuthorityRecord,
    pub review_sha256: String,
}

/// Host projection of the maintained Root allocation phases.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentPhase {
    Reserved,
    CreationIntent,
    Created,
    InstallIntent,
    Installed,
    Verified,
    Committed,
    Removed,
}

/// Root-observed identity and terminal activation evidence retained by the host.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentProgressRecord {
    pub allocation_sequence: u64,
    pub component: ComponentInstanceId,
    pub phase: ComponentPhase,
    pub binding: Option<ComponentBinding>,
    pub complete: bool,
}

/// One atomically published local operation record; intent precedes submission.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentOperationRecord {
    pub document_sha256: String,
    pub plan: ComponentPlanRecord,
    pub submission_attempts: u64,
    pub progress: Option<ComponentProgressRecord>,
}

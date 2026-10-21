//! Fleet admission model.
//!
//! Host owns authority and operations; command parsing and rendering belong to CLI.

use crate::{icp::IcpCli, protocol_binding::ResolvedProtocolBinding};
use canic_contracts::{
    dto::{
        fleet_admission::{FleetAdmissionMutationRequest, FleetAdmissionStatusResponse},
        fleet_registry::{FleetRegistry, FleetRegistryVersion},
    },
    ids::{FleetAdmissionPolicy, ManagedCanisterBinding},
};
use ic_platform::Principal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Persisted authority evidence for a reviewed admission mutation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionPlanRecord {
    pub schema_version: u16,
    pub fleet: String,
    pub environment: String,
    pub coordinator: Principal,
    pub predecessor_registry: FleetRegistryVersion,
    pub predecessor_policy: FleetAdmissionPolicy,
    pub successor_policy: FleetAdmissionPolicy,
    pub participant_catalogs: Vec<AdmissionParticipantCatalogRecord>,
    pub request: FleetAdmissionMutationRequest,
}

/// Exact Root participant catalog retained in the reviewed plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionParticipantCatalogRecord {
    pub fleet_subnet_root: Principal,
    pub participant_catalog_digest: [u8; 32],
    pub participants: Vec<ManagedCanisterBinding>,
}

/// Current resolved transport and live authority observations.
pub(super) struct AdmissionConnection {
    pub coordinator: Principal,
    pub icp: IcpCli,
    pub coordinator_binding: ResolvedProtocolBinding,
    pub root_bindings: BTreeMap<Principal, ResolvedProtocolBinding>,
    pub registry: FleetRegistry,
    pub registry_version: FleetRegistryVersion,
    pub admission: FleetAdmissionStatusResponse,
}

/// Exact operator-selected membership mutation; authority is resolved by the workflow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmissionMutation {
    pub action: canic_contracts::dto::fleet_admission::FleetAdmissionMutationAction,
    pub selector: canic_contracts::ids::FleetAdmissionSelector,
    pub principal: Principal,
}

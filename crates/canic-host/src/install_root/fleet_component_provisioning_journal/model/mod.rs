//! Module: install_root::fleet_component_provisioning_journal::model
//!
//! Responsibility: define the durable fresh-install provisioning document and typed host phases.
//! Does not own: transition orchestration, authority validation, filesystem I/O, or remote effects.
//! Boundary: this schema is the exact reinstall-only JSON document persisted by the host.

use crate::{
    fleet_catalog::FleetCatalogEntryV1, fleet_install_plan::PersistedFleetInstallPlan,
    install_root::fleet_component_provisioning_plan::CompiledFleetComponentProvisioningPlan,
};
use std::{io, path::PathBuf};

use candid::Principal;
use canic_core::{
    dto::component_provisioning::{
        FleetComponentProvisioningAdvanceRequest, FleetComponentProvisioningPrepareRequest,
        FleetComponentProvisioningStatusResponse,
    },
    ids::FleetName,
};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

pub(super) const JOURNAL_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::install_root) enum FleetComponentProvisioningInstallPhase {
    Planned,
    PreparationInFlight,
    Prepared,
    AdvanceInFlight,
    RuntimesActivated,
    CatalogPublicationInFlight,
    CatalogPublished,
    Complete,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::install_root) struct FleetComponentProvisioningInstallJournal {
    pub schema_version: u32,
    pub sequence: u64,
    pub phase: FleetComponentProvisioningInstallPhase,
    pub fleet_install_plan_digest: [u8; 32],
    pub coordinator: Principal,
    pub fleet_name: FleetName,
    pub environment: String,
    pub prepare_request: FleetComponentProvisioningPrepareRequest,
    pub plan_hash: [u8; 32],
    pub last_status: Option<FleetComponentProvisioningStatusResponse>,
    pub advance_request: Option<FleetComponentProvisioningAdvanceRequest>,
    pub catalog_entry: Option<FleetCatalogEntryV1>,
    pub catalog_hash: Option<[u8; 32]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::install_root) struct ResolvedFleetComponentProvisioningInstall {
    pub journal: FleetComponentProvisioningInstallJournal,
    pub path: PathBuf,
}

pub(in crate::install_root) struct PlanFleetComponentProvisioningInstallRequest<'a> {
    pub fleet_install_plan: &'a PersistedFleetInstallPlan,
    pub coordinator: Principal,
    pub fleet_name: FleetName,
    pub environment: String,
    pub compiled: CompiledFleetComponentProvisioningPlan,
}

#[derive(Debug, ThisError)]
pub(in crate::install_root) enum FleetComponentProvisioningInstallJournalError {
    #[error(
        "Fleet Component provisioning install journal already has different immutable authority: {path}"
    )]
    ConflictingAuthority { path: PathBuf },

    #[error("invalid Fleet Component provisioning install journal {path}: {reason}")]
    InvalidDocument { path: PathBuf, reason: String },

    #[error(
        "Fleet Component provisioning install journal cannot transition from {observed:?} to {requested:?}"
    )]
    InvalidTransition {
        observed: FleetComponentProvisioningInstallPhase,
        requested: FleetComponentProvisioningInstallPhase,
    },

    #[error("failed to access Fleet Component provisioning install journal {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Fleet Component provisioning install journal is not a regular no-follow file: {path}")]
    UnsafeFile { path: PathBuf },

    #[error(
        "Fleet Component provisioning install journal lock is not a regular no-follow file: {path}"
    )]
    UnsafeLock { path: PathBuf },
}

pub(super) const fn resolved(
    journal: FleetComponentProvisioningInstallJournal,
    path: PathBuf,
) -> ResolvedFleetComponentProvisioningInstall {
    ResolvedFleetComponentProvisioningInstall { journal, path }
}

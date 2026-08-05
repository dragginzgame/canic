//! Module: fleet_install_plan::model
//!
//! Responsibility: define passive pre-effect Fleet planning input, authority, and errors.
//! Does not own: topology compilation, artifact projection, persistence, or external effects.
//! Boundary: exact cycle amounts use canonical decimal strings only in the durable JSON shape.

use crate::{
    component_topology::{FleetTopologyPlanError, RootComponentAdmissionInput},
    release_build::ReleaseBuildPlanError,
    release_set::{ApplicationArtifactUnionPersistenceError, ApplicationReleaseSetError},
};
use std::{
    io,
    path::{Path, PathBuf},
};

use canic_core::{
    bootstrap::compiled::ConfigModel,
    cdk::types::{Cycles, Principal},
    ids::{
        ComponentSpecAdmission, ComponentTopologyDigest, CyclesFundingBudget, FleetBinding,
        FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseSetDigest,
        SubnetId,
    },
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use thiserror::Error as ThisError;

///
/// PlannedCanisterCreationFunding
///
/// Exact positive funding method resolved for one host-created initial Canister.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlannedCanisterCreationFunding {
    Cycles { cycles: u128 },
    Icp { e8s: u64 },
}

///
/// PlannedFleetCoordinator
///
/// Exact Coordinator placement and funding resolved before creation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedFleetCoordinator {
    pub coordinator_subnet: SubnetId,
    pub creation_funding: PlannedCanisterCreationFunding,
}

///
/// PlannedFleetSubnetRootInput
///
/// Explicit host input for one pre-creation Fleet Subnet Root plan.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedFleetSubnetRootInput {
    pub placement_subnet: SubnetId,
    pub component_admissions: Vec<RootComponentAdmissionInput>,
    pub limits: FleetSubnetRootLimits,
    pub canister_pool_imports: Vec<Principal>,
    pub root_creation_funding: PlannedCanisterCreationFunding,
    pub wasm_store_creation_funding: PlannedCanisterCreationFunding,
}

///
/// PlannedFleetSubnetRoot
///
/// Canonical root placement, topology, release set, limits, and funding before creation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedFleetSubnetRoot {
    pub placement_subnet: SubnetId,
    pub component_admissions: Vec<ComponentSpecAdmission>,
    pub component_topology_digest: ComponentTopologyDigest,
    pub initial_release_set: FleetSubnetRootReleaseSet,
    #[serde(with = "root_limits_document")]
    pub limits: FleetSubnetRootLimits,
    pub canister_pool_imports: Vec<Principal>,
    pub root_creation_funding: PlannedCanisterCreationFunding,
    pub wasm_store_creation_funding: PlannedCanisterCreationFunding,
}

///
/// FleetInstallPlan
///
/// Immutable pre-effect authority for one fresh multi-root Fleet installation.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetInstallPlan {
    pub fleet: FleetBinding,
    pub release_build_id: ReleaseBuildId,
    pub application_artifact_union_digest: [u8; 32],
    pub coordinator: PlannedFleetCoordinator,
    pub fleet_subnet_roots: Vec<PlannedFleetSubnetRoot>,
}

///
/// PersistedFleetSubnetRootReleaseSet
///
/// Exact durable release-set manifest for one planned root placement.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedFleetSubnetRootReleaseSet {
    pub placement_subnet: SubnetId,
    pub manifest: crate::release_set::FleetSubnetRootReleaseSetManifest,
    pub digest: ReleaseSetDigest,
    pub path: PathBuf,
}

///
/// PersistedFleetInstallPlan
///
/// Canonical plan plus every exact immutable root release-set file it admits.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistedFleetInstallPlan {
    pub plan: FleetInstallPlan,
    pub digest: [u8; 32],
    pub path: PathBuf,
    pub root_release_sets: Vec<PersistedFleetSubnetRootReleaseSet>,
}

///
/// FleetInstallPlanRequest
///
/// Complete explicit input required to freeze one pre-effect Fleet plan.
///

pub struct FleetInstallPlanRequest<'a> {
    pub root: &'a Path,
    pub config: &'a ConfigModel,
    pub fleet: FleetBinding,
    pub release_build_id: ReleaseBuildId,
    pub coordinator: PlannedFleetCoordinator,
    pub fleet_subnet_roots: Vec<PlannedFleetSubnetRootInput>,
}

///
/// FleetInstallPlanError
///
/// Typed rejection while compiling, publishing, or loading Fleet install authority.
///

#[derive(Debug, ThisError)]
pub enum FleetInstallPlanError {
    #[error("Fleet plan App '{fleet_app}' does not match configured App '{configured_app}'")]
    AppMismatch {
        configured_app: String,
        fleet_app: String,
    },

    #[error("Fleet install plan already exists with different canonical bytes: {path}")]
    ConflictingPlan { path: PathBuf },

    #[error("root release-set manifest already exists with different canonical bytes: {path}")]
    ConflictingRootReleaseSet { path: PathBuf },

    #[error("{owner} creation funding amount must be positive")]
    NonPositiveCreationFunding { owner: String },

    #[error("Coordinator placement Subnet must not be anonymous")]
    AnonymousCoordinatorSubnet,

    #[error("Fleet Subnet Root placement Subnet must not be anonymous")]
    AnonymousRootSubnet,

    #[error("Fleet install plan application artifact union digest does not match durable evidence")]
    ApplicationArtifactUnionDigestMismatch,

    #[error("Fleet Subnet Root plans are not in canonical placement order")]
    NonCanonicalRootOrder,

    #[error("root {placement_subnet} release build does not match the Fleet install plan")]
    RootReleaseBuildMismatch { placement_subnet: SubnetId },

    #[error("failed to serialize Fleet install plan: {0}")]
    PlanSerialization(serde_json::Error),

    #[error("invalid Fleet install plan {path}: {reason}")]
    InvalidPlanDocument { path: PathBuf, reason: String },

    #[error("invalid root release-set manifest {path}: {reason}")]
    InvalidRootReleaseSetDocument { path: PathBuf, reason: String },

    #[error("Fleet install plan is missing: {path}")]
    MissingPlan { path: PathBuf },

    #[error("root release-set manifest is missing: {path}")]
    MissingRootReleaseSet { path: PathBuf },

    #[error("Fleet install plan exceeds the {maximum_bytes}-byte bound: {actual_bytes}")]
    PlanTooLarge {
        maximum_bytes: usize,
        actual_bytes: usize,
    },

    #[error("root release-set manifest exceeds the {maximum_bytes}-byte bound: {actual_bytes}")]
    RootReleaseSetTooLarge {
        maximum_bytes: usize,
        actual_bytes: usize,
    },

    #[error("Fleet install plan is not a regular no-follow file: {path}")]
    UnsafePlan { path: PathBuf },

    #[error("Fleet install plan lock is not a regular no-follow file: {path}")]
    UnsafePlanLock { path: PathBuf },

    #[error("root release-set manifest is not a regular no-follow file: {path}")]
    UnsafeRootReleaseSet { path: PathBuf },

    #[error("failed to access Fleet install authority {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(transparent)]
    ApplicationUnion(#[from] ApplicationArtifactUnionPersistenceError),

    #[error(transparent)]
    ReleaseBuild(#[from] ReleaseBuildPlanError),

    #[error(transparent)]
    ReleaseSet(#[from] ApplicationReleaseSetError),

    #[error(transparent)]
    Topology(#[from] FleetTopologyPlanError),

    #[error(transparent)]
    ComponentTopology(#[from] canic_core::bootstrap::compiled::ComponentTopologyError),
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum CreationFundingDocument {
    Cycles { cycles: String },
    Icp { e8s: u64 },
}

impl Serialize for PlannedCanisterCreationFunding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Cycles { cycles } => CreationFundingDocument::Cycles {
                cycles: cycles.to_string(),
            },
            Self::Icp { e8s } => CreationFundingDocument::Icp { e8s: *e8s },
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PlannedCanisterCreationFunding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match CreationFundingDocument::deserialize(deserializer)? {
            CreationFundingDocument::Cycles { cycles } => {
                let cycles = cycles.parse().map_err(de::Error::custom)?;
                Ok(Self::Cycles { cycles })
            }
            CreationFundingDocument::Icp { e8s } => Ok(Self::Icp { e8s }),
        }
    }
}

mod root_limits_document {
    use super::*;

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct CyclesFundingBudgetDocument {
        window_secs: u64,
        maximum_cycles: String,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct CanisterPoolDocument {
        minimum_size: u32,
        maximum_size: u32,
        canister_cycles: String,
    }

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct RootLimitsDocument {
        maximum_component_instances: u32,
        maximum_registry_bytes: u64,
        maximum_wasm_store_bytes: u64,
        canister_pool: CanisterPoolDocument,
        cycles_funding: CyclesFundingBudgetDocument,
    }

    pub(super) fn serialize<S>(
        limits: &FleetSubnetRootLimits,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        RootLimitsDocument {
            maximum_component_instances: limits.maximum_component_instances,
            maximum_registry_bytes: limits.maximum_registry_bytes,
            maximum_wasm_store_bytes: limits.maximum_wasm_store_bytes,
            canister_pool: CanisterPoolDocument {
                minimum_size: limits.canister_pool.minimum_size,
                maximum_size: limits.canister_pool.maximum_size,
                canister_cycles: limits.canister_pool.canister_cycles.to_u128().to_string(),
            },
            cycles_funding: CyclesFundingBudgetDocument {
                window_secs: limits.cycles_funding.window_secs,
                maximum_cycles: limits.cycles_funding.maximum_cycles.to_u128().to_string(),
            },
        }
        .serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<FleetSubnetRootLimits, D::Error>
    where
        D: Deserializer<'de>,
    {
        let document = RootLimitsDocument::deserialize(deserializer)?;
        let maximum_cycles = document
            .cycles_funding
            .maximum_cycles
            .parse()
            .map_err(de::Error::custom)?;
        let canister_cycles = document
            .canister_pool
            .canister_cycles
            .parse()
            .map_err(de::Error::custom)?;
        Ok(FleetSubnetRootLimits {
            maximum_component_instances: document.maximum_component_instances,
            maximum_registry_bytes: document.maximum_registry_bytes,
            maximum_wasm_store_bytes: document.maximum_wasm_store_bytes,
            canister_pool: canic_core::ids::FleetSubnetCanisterPoolConfig {
                minimum_size: document.canister_pool.minimum_size,
                maximum_size: document.canister_pool.maximum_size,
                canister_cycles: Cycles::new(canister_cycles),
            },
            cycles_funding: CyclesFundingBudget {
                window_secs: document.cycles_funding.window_secs,
                maximum_cycles: Cycles::new(maximum_cycles),
            },
        })
    }
}

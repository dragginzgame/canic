//! Module: fleet_install_plan::model
//!
//! Responsibility: define passive pre-effect Fleet planning input, authority, and errors.
//! Does not own: topology compilation, artifact projection, persistence, or external effects.
//! Boundary: exact cycle amounts use canonical decimal strings only in the durable JSON shape.

use crate::{
    canister_build::CanisterBuildProfile,
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
        CanonicalNetworkId, ComponentGroupDeploymentId, ComponentSpecAdmission,
        ComponentTopologyDigest, CyclesFundingBudget, FleetAdmissionPolicy,
        FleetAdmissionPolicyTemplate, FleetBinding, FleetCoordinatorRootFundingPolicy,
        FleetFundingProfile, FleetName, FleetSubnetRootFundingAuthority, FleetSubnetRootLimits,
        FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseSetDigest, SubnetId,
    },
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use thiserror::Error as ThisError;

///
/// PlannedCanisterCreationFunding
///
/// Exact positive funding method resolved for one host-created initial Canister.
///

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
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
    pub placement_cost: PlannedSubnetPlacementCostEvidence,
    pub creation_funding: PlannedCanisterCreationFunding,
    pub root_funding: Option<FleetCoordinatorRootFundingPolicy>,
}

/// Trusted physical-placement and explicit Fiduciary cost authority retained in the plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedSubnetPlacementCostEvidence {
    pub subnet: SubnetId,
    pub catalog_sha256: Option<String>,
    pub subnet_specialization: String,
    pub node_count: u64,
    pub cost_multiplier_numerator: u64,
    pub cost_multiplier_denominator: u64,
    pub acknowledge_fiduciary_cost: bool,
    pub warning: Option<String>,
}

/// One explicit initial Component Group placement assigned to a planned root Subnet.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedComponentGroupPlacementAssignment {
    pub deployment: ComponentGroupDeploymentId,
    pub ordinal: u32,
}

///
/// PlannedFleetSubnetRootInput
///
/// Explicit host input for one pre-creation Fleet Subnet Root plan.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedFleetSubnetRootInput {
    pub placement_subnet: SubnetId,
    pub placement_cost: PlannedSubnetPlacementCostEvidence,
    pub component_group_placements: Vec<PlannedComponentGroupPlacementAssignment>,
    pub component_admissions: Vec<RootComponentAdmissionInput>,
    pub limits: FleetSubnetRootLimits,
    pub funding: FleetSubnetRootFundingAuthority,
    pub canister_pool_imports: Vec<Principal>,
    pub root_creation_funding: PlannedCanisterCreationFunding,
    pub wasm_store_creation_funding: PlannedCanisterCreationFunding,
}

/// Proof that preflight precedes build, deployment-state writes, and IC mutations.
///
/// Catalog evidence acquisition is an input-loading concern. Its private cache
/// disposition is not part of this decision authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FreshFleetPreflightEffectsV1 {
    pub build_started: bool,
    pub workspace_mutation_started: bool,
    pub ic_mutation_started: bool,
}

impl FreshFleetPreflightEffectsV1 {
    /// Construct the only effect state admitted by pure preflight.
    #[must_use]
    pub const fn none_started() -> Self {
        Self {
            build_started: false,
            workspace_mutation_started: false,
            ic_mutation_started: false,
        }
    }

    #[must_use]
    pub const fn no_effects_started(self) -> bool {
        !self.build_started && !self.workspace_mutation_started && !self.ic_mutation_started
    }
}

/// Complete named authority accepted by the pure fresh-Fleet preflight compiler.
pub struct FreshFleetPreflightRequest<'a> {
    pub config: &'a ConfigModel,
    pub app: &'a str,
    pub fleet_name: &'a FleetName,
    pub coordinator: &'a PlannedFleetCoordinator,
    pub admission: &'a FleetAdmissionPolicyTemplate,
    pub fleet_subnet_roots: &'a [PlannedFleetSubnetRootInput],
    pub build_profile: CanisterBuildProfile,
    pub release_build_id: Option<ReleaseBuildId>,
    pub effects: FreshFleetPreflightEffectsV1,
}

/// One canonical root plan proven before release-build allocation or another effect.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FreshFleetSubnetRootPlanV1 {
    pub placement_subnet: SubnetId,
    pub placement_cost: PlannedSubnetPlacementCostEvidence,
    pub component_group_placements: Vec<PlannedComponentGroupPlacementAssignment>,
    pub component_admissions: Vec<ComponentSpecAdmission>,
    pub component_topology_digest: ComponentTopologyDigest,
    pub admission_projections: Vec<PlannedFleetAdmissionProjection>,
    #[serde(with = "root_limits_document")]
    pub limits: FleetSubnetRootLimits,
    pub funding: FleetSubnetRootFundingAuthority,
    pub canister_pool_imports: Vec<Principal>,
    pub root_creation_funding: PlannedCanisterCreationFunding,
    pub wasm_store_creation_funding: PlannedCanisterCreationFunding,
    pub initial_component_canisters: u32,
    pub initial_pool_canisters: u32,
    pub pool_canister_creations: u32,
    pub remaining_pool_canisters: u32,
}

/// One protected pre-allocation effective admission projection bound into the Fleet plan.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedFleetAdmissionProjection {
    pub component_spec: canic_core::ids::ComponentSpecId,
    pub participant_roles: Vec<canic_core::ids::CanisterRole>,
    pub effective_principal_count: u32,
    pub template_projection_digest: [u8; 32],
}

/// Canonical placement, admission and funding result shared by plan and install.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FreshFleetPreflightV1 {
    pub schema_version: u16,
    pub app: String,
    pub fleet_name: FleetName,
    pub funding_profile: FleetFundingProfile,
    pub coordinator: PlannedFleetCoordinator,
    pub admission: FleetAdmissionPolicyTemplate,
    pub fleet_subnet_roots: Vec<FreshFleetSubnetRootPlanV1>,
    pub build_profile: String,
    pub release_build_id: Option<ReleaseBuildId>,
    pub effects: FreshFleetPreflightEffectsV1,
    #[serde(skip)]
    pub(super) component_topology: canic_core::bootstrap::compiled::ComponentTopology,
}

/// One expected build artifact bound into a workspace or finalized release source.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FreshFleetExpectedArtifactV1 {
    pub role: String,
    pub package: String,
}

/// Exact release-source identity admitted by a complete fresh-Fleet decision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FreshFleetReleaseSourceV1 {
    Workspace {
        builder_version: String,
        cargo_lock_sha256: String,
        source_snapshot_sha256: String,
        expected_artifacts: Vec<FreshFleetExpectedArtifactV1>,
    },
    Finalized {
        release_build_id: ReleaseBuildId,
        builder_version: String,
        release_build_plan_sha256: String,
        release_set_manifest_sha256: String,
        expected_artifacts: Vec<FreshFleetExpectedArtifactV1>,
    },
}

impl FreshFleetReleaseSourceV1 {
    #[must_use]
    pub fn expected_artifacts(&self) -> &[FreshFleetExpectedArtifactV1] {
        match self {
            Self::Workspace {
                expected_artifacts, ..
            }
            | Self::Finalized {
                expected_artifacts, ..
            } => expected_artifacts,
        }
    }
}

/// Catalog authority bound into a complete plan; local networks need no NNS catalog.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FreshFleetCatalogEvidenceV1 {
    NotRequired {
        network: String,
    },
    Validated {
        network: String,
        assurance: String,
        source_endpoints: Vec<String>,
        registry_version: u64,
        catalog_sha256: String,
    },
}

/// Exact operator funding account and one bounded balance observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FreshFleetOperatorFundingEvidenceV1 {
    pub principal: String,
    pub funding_account: String,
    pub balance: PlannedCanisterCreationFunding,
    pub source: String,
    pub observed_at_unix_secs: u64,
    pub valid_until_unix_secs: u64,
    pub balance_fresh: bool,
}

/// Complete loader-owned authority passed to the pure decision compiler.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FreshFleetDecisionAuthorityV1 {
    pub app_config_sha256: String,
    pub requested_environment: String,
    pub canonical_network_id: CanonicalNetworkId,
    pub fleet_input_schema_version: u32,
    pub fleet_input_sha256: String,
    pub release_source: FreshFleetReleaseSourceV1,
    pub catalog: FreshFleetCatalogEvidenceV1,
    pub operator: FreshFleetOperatorFundingEvidenceV1,
}

/// Expected physical and role-bearing Canister counts after initial placement.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FreshFleetCanisterCountsV1 {
    pub coordinator_canisters: u32,
    pub root_canisters: u32,
    pub wasm_store_canisters: u32,
    pub component_canisters: u32,
    pub ready_pool_canisters: u32,
    pub role_canisters: u32,
    pub total_canisters: u32,
}

/// Account that bears one category of fresh-install funding.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshFleetFundingPayerV1 {
    Operator,
    FleetSubnetRoot,
}

/// One checked creation-amount or creation-fee maximum included in the plan digest.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FreshFleetFundingRequirementV1 {
    pub category: String,
    pub owner: String,
    pub payer: FreshFleetFundingPayerV1,
    pub canister_count: u32,
    pub per_canister: PlannedCanisterCreationFunding,
    pub maximum: PlannedCanisterCreationFunding,
}

/// Complete successful fresh-Fleet admission decision and its canonical identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FreshFleetDeploymentPlanV1 {
    pub schema_version: u16,
    pub preflight: FreshFleetPreflightV1,
    pub authority: FreshFleetDecisionAuthorityV1,
    pub counts: FreshFleetCanisterCountsV1,
    pub funding_requirements: Vec<FreshFleetFundingRequirementV1>,
    pub maximum_operator_debit: PlannedCanisterCreationFunding,
    pub operator_balance_sufficient: bool,
    pub plan_digest: String,
}

/// Complete evidence accepted by the pure canonical decision compiler.
pub struct FreshFleetDeploymentPlanRequest {
    pub preflight: FreshFleetPreflightV1,
    pub authority: FreshFleetDecisionAuthorityV1,
}

/// Typed rejection produced exclusively by the pure fresh-Fleet compiler.
#[derive(Debug, ThisError)]
pub enum FreshFleetPreflightError {
    #[error("preflight App '{requested_app}' does not match configured App '{configured_app}'")]
    AppMismatch {
        configured_app: String,
        requested_app: String,
    },

    #[error(
        "fresh-Fleet preflight began after an effect boundary: build={build_started}, workspace_mutation={workspace_mutation_started}, ic_mutation={ic_mutation_started}"
    )]
    EffectsAlreadyStarted {
        build_started: bool,
        workspace_mutation_started: bool,
        ic_mutation_started: bool,
    },

    #[error("Coordinator placement Subnet must not be anonymous")]
    AnonymousCoordinatorSubnet,

    #[error("{owner} creation funding amount must be positive")]
    NonPositiveCreationFunding { owner: String },

    #[error("validated topology root {placement_subnet} has no exact resolved input")]
    MissingResolvedRoot { placement_subnet: SubnetId },

    #[error("initial Component Group placement assignments are invalid: {reason}")]
    InvalidComponentGroupPlacementAssignments { reason: String },

    #[error("{subject} count does not fit u32")]
    CountDoesNotFitU32 { subject: &'static str },

    #[error("protected Fleet admission policy is invalid: {reason}")]
    InvalidAdmissionPolicy { reason: String },

    #[error("generation-one Fleet admission selects unknown Component Spec '{component_spec}'")]
    UnknownAdmissionComponentSpec { component_spec: String },

    #[error("generation-one Fleet admission selects unknown Root Subnet {placement_subnet}")]
    UnknownAdmissionFleetSubnetRoot { placement_subnet: SubnetId },

    #[error("generation-one Fleet admission does not accept Fleet or Component-instance rules")]
    UnsupportedAdmissionSelector,

    #[error(transparent)]
    Topology(#[from] FleetTopologyPlanError),
}

/// Typed blocker emitted before a complete fresh-Fleet decision can be admitted.
#[derive(Debug, ThisError)]
pub enum FreshFleetDeploymentPlanError {
    #[error("{field} must be exactly 64 lowercase hexadecimal characters")]
    InvalidSha256 { field: &'static str },

    #[error("{field} must not be empty")]
    EmptyAuthority { field: &'static str },

    #[error("operator principal must not be anonymous")]
    AnonymousOperator,

    #[error("release-source artifact inventory is not strictly canonical")]
    NonCanonicalArtifactInventory,

    #[error("finalized release source differs from preflight release-build identity")]
    ReleaseBuildIdentityMismatch,

    #[error("{subject} count overflowed")]
    CountOverflow { subject: &'static str },

    #[error("{subject} funding overflowed")]
    FundingOverflow { subject: String },

    #[error("operator-funded creation requirements use more than one funding unit")]
    MixedOperatorFunding,

    #[error("operator balance uses a different funding unit from maximum debit")]
    OperatorBalanceUnitMismatch,

    #[error("operator balance is insufficient for the maximum debit")]
    InsufficientOperatorBalance,

    #[error("operator balance evidence is not fresh")]
    StaleOperatorBalance,

    #[error("failed to encode canonical fresh-Fleet plan input: {0}")]
    PlanSerialization(serde_json::Error),
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
    pub placement_cost: PlannedSubnetPlacementCostEvidence,
    pub component_group_placements: Vec<PlannedComponentGroupPlacementAssignment>,
    pub component_admissions: Vec<ComponentSpecAdmission>,
    pub component_topology_digest: ComponentTopologyDigest,
    pub admission_projections: Vec<PlannedFleetAdmissionProjection>,
    pub initial_release_set: FleetSubnetRootReleaseSet,
    #[serde(with = "root_limits_document")]
    pub limits: FleetSubnetRootLimits,
    pub funding: FleetSubnetRootFundingAuthority,
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
    pub fresh_fleet_plan_digest: String,
    pub release_build_id: ReleaseBuildId,
    pub application_artifact_union_digest: [u8; 32],
    pub admission: FleetAdmissionPolicy,
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
    pub fleet_name: FleetName,
    pub fresh_fleet_plan_digest: String,
    pub release_build_id: ReleaseBuildId,
    pub coordinator: PlannedFleetCoordinator,
    pub admission: FleetAdmissionPolicyTemplate,
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

    #[error(
        "Fleet install planning crossed an effect boundary before pure preflight: build={build_started}, workspace_mutation={workspace_mutation_started}, ic_mutation={ic_mutation_started}"
    )]
    EffectsAlreadyStarted {
        build_started: bool,
        workspace_mutation_started: bool,
        ic_mutation_started: bool,
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

    #[error("validated topology root {placement_subnet} has no exact resolved input")]
    MissingResolvedRoot { placement_subnet: SubnetId },

    #[error("Fleet install plan application artifact union digest does not match durable evidence")]
    ApplicationArtifactUnionDigestMismatch,

    #[error("Fleet install plan fresh-Fleet digest is not canonical SHA-256 text")]
    InvalidFreshFleetPlanDigest,

    #[error("Fleet install admission policy is invalid: {reason}")]
    InvalidAdmissionPolicy { reason: String },

    #[error("Fleet Subnet Root plans are not in canonical placement order")]
    NonCanonicalRootOrder,

    #[error("initial Component Group placement assignments are invalid: {reason}")]
    InvalidComponentGroupPlacementAssignments { reason: String },

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
        maximum_group_placements: u32,
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
            maximum_group_placements: limits.maximum_group_placements,
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
            maximum_group_placements: document.maximum_group_placements,
        })
    }
}

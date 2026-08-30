//! Module: fleet_ensure::generate
//!
//! Responsibility: compile high-level Fleet policy plus explicit live identities into desired state.
//! Does not own: artifact builds, identity invention, paid effects, or convergence.
//! Boundary: release artifacts are local authority; every retained Principal is explicit and live-verified.

#[cfg(test)]
mod tests;

use crate::{
    canister_protocol::query_with_candid,
    component_topology::{
        PlannedFleetSubnetRootTopology, PlannedFleetSubnetRootTopologyInput, PlannedFleetTopology,
        RootComponentAdmissionInput, RootPoolCapacityError, RootPoolCapacityInput,
        plan_initial_fleet_topology, validate_root_pool_capacity,
    },
    durable_io::{
        RegularFileReadError, create_new_bytes_with_parents, read_optional_regular_bytes,
    },
    fleet_ensure::model::{
        CanisterRuntimeStatus, DesiredCanister, DesiredCanisterInit, DesiredCanisterKind,
        DesiredComponentGroupPlacement, DesiredFleet, DesiredFleetBootstrap,
        DesiredFleetBootstrapRoot, DesiredFleetProtocol, DesiredPresence,
        FLEET_ENSURE_SCHEMA_VERSION, MAX_FLEET_ENSURE_CANISTERS, RetainedRootStartAuthorityRecord,
        RetainedRootStartBinding,
    },
    fleet_ensure::ops::{
        EnsurePaths, predecessor_root_status, read_root_start_authority, root_owned_lifecycle,
        verify_root_start_release_authority, write_root_start_authority,
    },
    icp::IcpCli,
    icp_config::resolve_icp_build_network_from_root,
    network::resolve_canonical_network_id_from_root,
    release_build::validate_finalized_release_build_manifest,
    release_set::{
        AppConfigSnapshot, CanicInfrastructureArtifactEntry, CanicInfrastructureRole,
        load_persisted_application_artifact_union,
        load_persisted_canic_infrastructure_artifact_manifest,
        load_persisted_current_release_set_manifest,
    },
    subnet_catalog::load_mainnet_subnet_catalog,
};
use candid::{Nat, Principal};
use canic_core::{
    cdk::types::Cycles,
    control_plane_support::config::ComponentDeploymentConfiguration,
    dto::fleet_subnet_root::FleetSubnetRootAuthority,
    dto::pool::{CanisterPoolResponse, CanisterPoolStatusRequest},
    ids::{
        BuildNetwork, ComponentGroupDeploymentId, ComponentSpecId, CyclesFundingBudget,
        FleetAdmissionPolicyTemplate, FleetBinding, FleetCoordinatorBinding,
        FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetKey, FleetRegistryAuthority,
        FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding, FleetSubnetRootFundingAuthority,
        FleetSubnetRootFundingPolicy, FleetSubnetRootLimits, FleetSubnetWasmStoreAuthority,
        ReleaseBuildId, SubnetId,
    },
    protocol,
    shared_support::fleet_admission_policy::compile_fleet_admission_policy_template,
};
use ic_query::subnet_catalog::SubnetSpecialization;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const MAX_GENERATOR_INPUT_BYTES: usize = 1024 * 1024;
const MAINNET_CYCLES_LEDGER: &str = "um5iw-rqaaa-aaaaq-qaaba-cai";
const GENERATED_RETAINED_MATERIAL_CYCLE_THRESHOLD: u128 = 1_000_000;
const GENERATED_RETAINED_MAXIMUM_OBSERVATION_BURN_CYCLES: u128 = 1_000_000_000_000;
const GENERATED_RETAINED_MAXIMUM_UPDATE_BURN_CYCLES: u128 = 100_000_000_000;

/// Exact local and live inputs for one no-effect desired-state generation.
pub struct FleetGenerateRequest<'a> {
    pub app_config: &'a Path,
    pub environment: &'a str,
    pub fleet: &'a str,
    pub icp_executable: &'a str,
    pub release_build_id: ReleaseBuildId,
    pub root: &'a Path,
    pub seed: &'a Path,
    pub source: &'a Path,
}

/// Exact authority used to create one durable empty-estate identity seed.
pub struct FreshEstateSeedRequest<'a> {
    pub cycles_ledger: &'a str,
    pub management_creation_fee_cycles: u128,
    pub seed: &'a Path,
    pub source: &'a Path,
}

/// Generated desired state plus its explicit observation summary.
pub struct GeneratedDesiredFleet {
    pub desired: DesiredFleet,
    pub observed_canisters: usize,
    pub observed_controlled_cycles: u128,
    pub release_build_id: ReleaseBuildId,
}

/// Typed no-effect Fleet generation failure.
#[derive(Debug, ThisError)]
pub enum FleetGenerateError {
    #[error("failed to read Fleet generator input {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write Fleet generator input {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Fleet generator input is not a safe regular file: {path} ({reason})")]
    UnsafeInput { path: PathBuf, reason: &'static str },

    #[error("Fleet generator input is too large: {path} ({actual} bytes; maximum {maximum})")]
    TooLarge {
        actual: usize,
        maximum: usize,
        path: PathBuf,
    },

    #[error("failed to decode Fleet generator input {path}: {source}")]
    Decode {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("unsupported Fleet generator {kind} schema {actual}; expected 1")]
    Schema { actual: u32, kind: &'static str },

    #[error("invalid Fleet generator authority: {0}")]
    Authority(String),

    #[error("existing Fleet identity seed conflicts with requested fresh-estate authority: {0}")]
    FreshSeedConflict(String),

    #[error("Fleet identity seed and policy topology differ: {0}")]
    SeedTopology(String),

    #[error("retained canister {canister} is unavailable: {reason}")]
    CanisterUnavailable { canister: String, reason: String },

    #[error(transparent)]
    StoppedRootStartRequired(Box<StoppedRootStartPrerequisite>),

    #[error(
        "retained canister {canister} controller set differs: actual {actual:?}, expected {expected:?}"
    )]
    ControllerMismatch {
        actual: Vec<String>,
        canister: String,
        expected: Vec<String>,
    },

    #[error("retained canister {canister} is on Subnet {actual}, expected {expected}")]
    SubnetMismatch {
        actual: String,
        canister: String,
        expected: String,
    },

    #[error("release authority is invalid: {0}")]
    Release(String),

    #[error("artifact Candid sidecar is missing or changed: {0}")]
    Candid(String),

    #[error(transparent)]
    ComponentPoolCapacity(#[from] RootPoolCapacityError),

    #[error("system clock is before the Unix epoch")]
    Clock,
}

/// Exact management evidence for the reviewed same-Root Start prerequisite.
#[derive(Debug, ThisError)]
#[error(
    "retained Root {root} is stopped after exact management verification \
     (Subnet {subnet}, controller {controller}, module SHA-256 {module_sha256}); \
     no protected Root query or desired-output mutation was attempted. Generator authority \
     {authority_sha256} binds this exact module to desired successor {successor_module_sha256}. \
     The authority covers {root_count} stopped Root(s). Review and apply only the same-ID Start \
     through the current retained \
     `canic fleet ensure {fleet}` authority, \
     then rerun `canic fleet generate`"
)]
pub struct StoppedRootStartPrerequisite {
    pub authority_sha256: String,
    pub controller: String,
    pub fleet: String,
    pub module_sha256: String,
    pub root: String,
    pub root_count: usize,
    pub subnet: String,
    pub successor_module_sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FleetSource {
    schema_version: u32,
    funding_profile: FleetFundingProfile,
    operator: String,
    admission: AdmissionSource,
    coordinator: CoordinatorSource,
    fleet_subnet_roots: Vec<RootSource>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdmissionSource {
    principals: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoordinatorSource {
    subnet: ExplicitSubnetSource,
    creation_funding: CyclesCreationSource,
    root_funding: CoordinatorFundingSource,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExplicitSubnetSource {
    kind: String,
    subnet: String,
    acknowledge_fiduciary_cost: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CyclesCreationSource {
    kind: String,
    #[serde(deserialize_with = "Cycles::from_human_config")]
    cycles: Cycles,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoordinatorFundingSource {
    #[serde(deserialize_with = "Cycles::from_human_config")]
    minimum_reserve_cycles: Cycles,
    window_secs: u64,
    #[serde(deserialize_with = "Cycles::from_human_config")]
    maximum_cycles: Cycles,
    maximum_automatic_grants: u32,
    #[serde(deserialize_with = "Cycles::from_human_config")]
    maximum_automatic_cycles: Cycles,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootSource {
    placement_subnet: String,
    #[serde(default)]
    acknowledge_fiduciary_cost: bool,
    #[serde(default)]
    component_group_placements: BTreeMap<ComponentGroupDeploymentId, Vec<u32>>,
    component_admissions: BTreeMap<ComponentSpecId, u32>,
    canister_pool: PoolSource,
    root_funding: RootFundingSource,
    limits: LimitsSource,
    root_creation_funding: CyclesCreationSource,
    wasm_store_creation_funding: CyclesCreationSource,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PoolSource {
    minimum_size: u32,
    maximum_size: u32,
    #[serde(deserialize_with = "Cycles::from_human_config")]
    canister_cycles: Cycles,
    #[serde(default)]
    imports: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootFundingSource {
    #[serde(deserialize_with = "Cycles::from_human_config")]
    request_threshold: Cycles,
    #[serde(deserialize_with = "Cycles::from_human_config")]
    target_balance: Cycles,
    cooldown_secs: u64,
    window_secs: u64,
    #[serde(deserialize_with = "Cycles::from_human_config")]
    maximum_cycles: Cycles,
    maximum_automatic_grants: u32,
    #[serde(deserialize_with = "Cycles::from_human_config")]
    maximum_automatic_cycles: Cycles,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LimitsSource {
    maximum_component_instances: u32,
    maximum_registry_bytes: u64,
    maximum_wasm_store_bytes: u64,
    maximum_group_placements: u32,
    cycles_funding: CyclesFundingSource,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CyclesFundingSource {
    window_secs: u64,
    #[serde(deserialize_with = "Cycles::from_human_config")]
    maximum_cycles: Cycles,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct EstateSeed {
    schema_version: u32,
    fleet_id: canic_core::ids::FleetId,
    #[serde(default)]
    fresh_estate: bool,
    coordinator: String,
    #[serde(default)]
    treasury: Option<TreasurySeed>,
    #[serde(default = "mainnet_cycles_ledger")]
    cycles_ledger: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    management_creation_fee_cycles: Option<String>,
    roots: Vec<RootSeed>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct TreasurySeed {
    principal: String,
    subnet: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RootSeed {
    placement_subnet: String,
    root: String,
    store: String,
    #[serde(default)]
    pool_imports: Vec<String>,
}

/// Create or replay one durable no-effect seed for a literally empty estate.
pub fn initialize_fresh_estate_seed(
    request: &FreshEstateSeedRequest<'_>,
) -> Result<canic_core::ids::FleetId, FleetGenerateError> {
    let source: FleetSource = load_toml(request.source, "source")?;
    require_schema(source.schema_version, "source")?;
    parse_principal("Cycles Ledger", request.cycles_ledger)?;
    if let Some(existing) = read_seed(request.seed)? {
        require_fresh_seed_authority(&existing, &source, request)?;
        return Ok(existing.fleet_id);
    }
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| {
        FleetGenerateError::Authority(format!("Fleet ID generation failed: {error}"))
    })?;
    let seed = fresh_seed(
        &source,
        canic_core::ids::FleetId::from_generated_bytes(bytes),
        request.cycles_ledger,
        request.management_creation_fee_cycles,
    )?;
    let encoded = toml::to_string_pretty(&seed).map_err(|error| {
        FleetGenerateError::Authority(format!("fresh seed encoding failed: {error}"))
    })?;
    match create_new_bytes_with_parents(request.seed, encoded.as_bytes()) {
        Ok(()) => Ok(seed.fleet_id),
        Err(io_error) if io_error.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = read_seed(request.seed)?.ok_or_else(|| FleetGenerateError::Read {
                path: request.seed.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "concurrent fresh seed publication disappeared",
                ),
            })?;
            require_fresh_seed_authority(&existing, &source, request)?;
            Ok(existing.fleet_id)
        }
        Err(source) => Err(FleetGenerateError::Write {
            path: request.seed.to_path_buf(),
            source,
        }),
    }
}

fn read_seed(path: &Path) -> Result<Option<EstateSeed>, FleetGenerateError> {
    let bytes = match read_optional_regular_bytes(path) {
        Ok(bytes) => bytes,
        Err(RegularFileReadError::NotRegular) => {
            return Err(FleetGenerateError::UnsafeInput {
                path: path.to_path_buf(),
                reason: "not a regular no-follow file",
            });
        }
        Err(RegularFileReadError::Io(source)) => {
            return Err(FleetGenerateError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(FleetGenerateError::UnsafeInput {
                path: path.to_path_buf(),
                reason: "safe no-follow reads are unsupported on this platform",
            });
        }
    };
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    if bytes.len() > MAX_GENERATOR_INPUT_BYTES {
        return Err(FleetGenerateError::TooLarge {
            actual: bytes.len(),
            maximum: MAX_GENERATOR_INPUT_BYTES,
            path: path.to_path_buf(),
        });
    }
    toml::from_slice(&bytes)
        .map(Some)
        .map_err(|source| FleetGenerateError::Decode {
            path: path.to_path_buf(),
            source,
        })
}

fn fresh_seed(
    source: &FleetSource,
    fleet_id: canic_core::ids::FleetId,
    cycles_ledger: &str,
    management_creation_fee_cycles: u128,
) -> Result<EstateSeed, FleetGenerateError> {
    if source
        .fleet_subnet_roots
        .iter()
        .any(|root| !root.canister_pool.imports.is_empty())
    {
        return Err(FleetGenerateError::FreshSeedConflict(
            "fresh estate policy must not import retained pool identities".to_string(),
        ));
    }
    let controlled = source
        .fleet_subnet_roots
        .iter()
        .try_fold(1_usize, |total, root| {
            let pools = usize::try_from(root.canister_pool.minimum_size).map_err(|_| {
                FleetGenerateError::FreshSeedConflict(
                    "fresh pool minimum cannot be represented on this host".to_string(),
                )
            })?;
            total
                .checked_add(2)
                .and_then(|value| value.checked_add(pools))
                .ok_or_else(|| {
                    FleetGenerateError::FreshSeedConflict(
                        "fresh estate role count overflowed".to_string(),
                    )
                })
        })?;
    if controlled > MAX_FLEET_ENSURE_CANISTERS {
        return Err(FleetGenerateError::FreshSeedConflict(format!(
            "fresh estate declares {controlled} controlled roles, above limit {MAX_FLEET_ENSURE_CANISTERS}"
        )));
    }
    let roots = source
        .fleet_subnet_roots
        .iter()
        .enumerate()
        .map(|(index, root)| RootSeed {
            placement_subnet: root.placement_subnet.clone(),
            root: format!("root-{index}"),
            store: format!("store-{index}"),
            pool_imports: (0..root.canister_pool.minimum_size)
                .map(|pool| format!("root-{index}-pool-{pool}"))
                .collect(),
        })
        .collect();
    Ok(EstateSeed {
        schema_version: 1,
        fleet_id,
        fresh_estate: true,
        coordinator: "coordinator".to_string(),
        treasury: None,
        cycles_ledger: cycles_ledger.to_string(),
        management_creation_fee_cycles: Some(config_cycles(management_creation_fee_cycles)),
        roots,
    })
}

fn require_fresh_seed_authority(
    actual: &EstateSeed,
    source: &FleetSource,
    request: &FreshEstateSeedRequest<'_>,
) -> Result<(), FleetGenerateError> {
    let expected = fresh_seed(
        source,
        actual.fleet_id,
        request.cycles_ledger,
        request.management_creation_fee_cycles,
    )?;
    if actual == &expected {
        Ok(())
    } else {
        Err(FleetGenerateError::FreshSeedConflict(
            "existing seed is retained or differs from the requested ledger, fee, or topology"
                .to_string(),
        ))
    }
}

#[derive(Clone)]
struct ObservedCanister {
    cycles: u128,
    module_sha256: Option<String>,
    subnet: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WasmStoreIdentityRelationship {
    authority: canic_core::ids::FleetRegistryAuthority,
    placement_subnet: SubnetId,
    fleet_subnet_root: Principal,
    wasm_store: Principal,
}

impl From<&FleetSubnetWasmStoreAuthority> for WasmStoreIdentityRelationship {
    fn from(value: &FleetSubnetWasmStoreAuthority) -> Self {
        Self {
            authority: value.authority.clone(),
            placement_subnet: value.placement_subnet,
            fleet_subnet_root: value.fleet_subnet_root,
            wasm_store: value.wasm_store,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RootIdentityRelationship {
    authority: canic_core::ids::FleetRegistryAuthority,
    placement_subnet: SubnetId,
    fleet_subnet_root: Principal,
}

impl From<&FleetSubnetRootBinding> for RootIdentityRelationship {
    fn from(value: &FleetSubnetRootBinding) -> Self {
        Self {
            authority: value.authority.clone(),
            placement_subnet: value.placement_subnet,
            fleet_subnet_root: value.fleet_subnet_root,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RootDesiredPolicy {
    component_admissions: Vec<canic_core::ids::ComponentSpecAdmission>,
    component_topology_digest: canic_core::ids::ComponentTopologyDigest,
    funding: FleetSubnetRootFundingAuthority,
    installation_controller: Principal,
    limits: FleetSubnetRootLimits,
}

impl From<&FleetSubnetRootAuthority> for RootDesiredPolicy {
    fn from(value: &FleetSubnetRootAuthority) -> Self {
        Self {
            component_admissions: value.binding.component_admissions.clone(),
            component_topology_digest: value.binding.component_topology_digest,
            funding: value.binding.funding.clone(),
            installation_controller: value.wasm_store_authority.installation_controller,
            limits: value.binding.limits.clone(),
        }
    }
}

struct ExpectedRootEstateAuthority {
    coordinator: FleetCoordinatorBinding,
    root: RootIdentityRelationship,
    store: WasmStoreIdentityRelationship,
}

struct RootEstateAuthorityRequest<'a, 'request> {
    app: &'a canic_core::ids::AppId,
    generation: &'a FleetGenerateRequest<'request>,
    root_candid: &'a Path,
    root_wasm_sha256: &'a str,
    predecessor_status_roots: &'a BTreeSet<String>,
    seed: &'a EstateSeed,
    source: &'a FleetSource,
    topology: &'a crate::component_topology::PlannedFleetTopology,
}

struct EstateObservationRequest<'a, 'request> {
    app: &'a canic_core::ids::AppId,
    generation: &'a FleetGenerateRequest<'request>,
    operator: Principal,
    root_candid: &'a Path,
    root_wasm_sha256: &'a str,
    seed: &'a EstateSeed,
    source: &'a FleetSource,
    topology: &'a crate::component_topology::PlannedFleetTopology,
}

#[derive(candid::CandidType)]
enum RootEstateStatusRequest {
    FleetAuthority,
    Pool(CanisterPoolStatusRequest),
}

#[derive(candid::CandidType, Deserialize)]
enum RootEstateStatusResponse {
    FleetAuthority(Box<FleetSubnetRootAuthority>),
    Pool(Box<CanisterPoolResponse>),
}

/// Generate one exact low-level desired document without issuing an IC update.
#[expect(
    clippy::too_many_lines,
    reason = "generation keeps one visible fail-closed authority-validation sequence"
)]
pub fn generate_desired_fleet(
    request: &FleetGenerateRequest<'_>,
) -> Result<GeneratedDesiredFleet, FleetGenerateError> {
    let source: FleetSource = load_toml(request.source, "source")?;
    let seed: EstateSeed = load_toml(request.seed, "seed")?;
    require_schema(source.schema_version, "source")?;
    require_schema(seed.schema_version, "seed")?;
    validate_identity_seed(&source, &seed)?;
    require_cycles_creation(&source.coordinator.creation_funding, "Coordinator")?;
    if source.coordinator.subnet.kind != "explicit" {
        return Err(FleetGenerateError::Authority(
            "Coordinator Subnet must be an exact explicit protected Subnet".to_string(),
        ));
    }
    let operator = parse_principal("operator", &source.operator)?;
    parse_subnet("Coordinator", &source.coordinator.subnet.subnet)?;
    let config = AppConfigSnapshot::load(request.app_config)
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    let deployment_configuration = ComponentDeploymentConfiguration::compile(config.model())
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    let admission_principals = source
        .admission
        .principals
        .iter()
        .map(|value| parse_principal("Fleet admission", value))
        .collect::<Result<Vec<_>, _>>()?;
    let admission: FleetAdmissionPolicyTemplate =
        compile_fleet_admission_policy_template(admission_principals, Vec::new())
            .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    let root_inputs = source
        .fleet_subnet_roots
        .iter()
        .map(|root| {
            Ok(PlannedFleetSubnetRootTopologyInput {
                placement_subnet: parse_subnet("Fleet Subnet Root", &root.placement_subnet)?,
                component_admissions: root
                    .component_admissions
                    .iter()
                    .map(
                        |(component_spec, maximum_root_instances)| RootComponentAdmissionInput {
                            component_spec: component_spec.clone(),
                            maximum_root_instances: *maximum_root_instances,
                        },
                    )
                    .collect(),
                limits: root_limits(root),
            })
        })
        .collect::<Result<Vec<_>, FleetGenerateError>>()?;
    let topology = plan_initial_fleet_topology(config.model(), root_inputs)
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    let capacity_roots = bind_root_generation_inputs(
        &source.fleet_subnet_roots,
        &seed.roots,
        &topology.fleet_subnet_roots,
    )?
    .into_iter()
    .map(|binding| RootPoolCapacityInput {
        component_admissions: binding.planned.component_admissions.clone(),
        pool_target_cycles: binding
            .planned
            .limits
            .canister_pool
            .canister_cycles
            .to_u128(),
        root: binding.seed.root.clone(),
    })
    .collect::<Vec<_>>();
    validate_root_pool_capacity(config.model(), &capacity_roots)?;
    let (infrastructure, complete) = release_authority(request, config.component_topology())?;
    if complete.manifest.infrastructure_artifact_manifest_sha256 != infrastructure.digest {
        return Err(FleetGenerateError::Release(
            "complete release set does not bind its infrastructure manifest".to_string(),
        ));
    }
    let coordinator_artifact = artifact(
        &infrastructure.manifest.entries,
        CanicInfrastructureRole::FleetCoordinator,
    )?;
    let root_artifact = artifact(
        &infrastructure.manifest.entries,
        CanicInfrastructureRole::FleetSubnetRoot,
    )?;
    let store_artifact = artifact(
        &infrastructure.manifest.entries,
        CanicInfrastructureRole::WasmStore,
    )?;
    let coordinator_candid = candid_sidecar(request.root, coordinator_artifact)?;
    let root_candid = candid_sidecar(request.root, root_artifact)?;
    let store_candid = candid_sidecar(request.root, store_artifact)?;
    let root_candid_path = request.root.join(&root_candid);
    let observed = if seed.fresh_estate {
        validate_fresh_generation_authority(request, &source, operator)?;
        BTreeMap::new()
    } else {
        observe_estate(&EstateObservationRequest {
            app: config.model().app_id(),
            generation: request,
            operator,
            root_candid: &root_candid_path,
            root_wasm_sha256: &root_artifact.wasm_sha256_hex,
            seed: &seed,
            source: &source,
            topology: &topology,
        })?
    };
    let treasury = seed
        .treasury
        .as_ref()
        .map_or(seed.coordinator.as_str(), |treasury| {
            treasury.principal.as_str()
        });
    if !seed.fresh_estate && !observed.contains_key(treasury) {
        return Err(FleetGenerateError::Authority(
            "treasury must be the seeded Coordinator or another explicitly seeded controlled canister"
                .to_string(),
        ));
    }
    let ledger_fee_cycles = observe_ledger_fee(request, &seed.cycles_ledger)?;
    let desired = compile_desired(CompileDesiredRequest {
        request,
        source: &source,
        seed: &seed,
        admission,
        app: config.model().app_id().clone(),
        deployment_configuration,
        topology,
        coordinator_artifact,
        root_artifact,
        store_artifact,
        coordinator_candid: &coordinator_candid,
        root_candid: &root_candid,
        store_candid: &store_candid,
        observed: &observed,
        treasury,
        ledger_fee_cycles,
    })?;
    let observed_controlled_cycles = observed.values().try_fold(0_u128, |total, canister| {
        total.checked_add(canister.cycles).ok_or_else(|| {
            FleetGenerateError::Authority("observed controlled cycle total overflowed".to_string())
        })
    })?;
    Ok(GeneratedDesiredFleet {
        desired,
        observed_canisters: observed.len(),
        observed_controlled_cycles,
        release_build_id: request.release_build_id,
    })
}

fn validate_fresh_generation_authority(
    request: &FleetGenerateRequest<'_>,
    source: &FleetSource,
    operator: Principal,
) -> Result<(), FleetGenerateError> {
    let icp = IcpCli::new(
        request.icp_executable,
        Some(request.environment.to_string()),
    )
    .with_cwd(request.root.to_path_buf());
    let active = icp
        .identity_principal_text()
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    if active != operator.to_text() {
        return Err(FleetGenerateError::Authority(format!(
            "active identity {active} differs from protected operator {operator}"
        )));
    }
    let network = resolve_icp_build_network_from_root(request.root, request.environment)
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    if network != BuildNetwork::Ic {
        return Ok(());
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| FleetGenerateError::Clock)?
        .as_secs();
    let catalog = load_mainnet_subnet_catalog(request.root, now)
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    require_fiduciary_acknowledgement(
        &catalog.catalog,
        &source.coordinator.subnet.subnet,
        source.coordinator.subnet.acknowledge_fiduciary_cost,
        "Coordinator",
    )?;
    for root in &source.fleet_subnet_roots {
        require_fiduciary_acknowledgement(
            &catalog.catalog,
            &root.placement_subnet,
            root.acknowledge_fiduciary_cost,
            "Fleet Subnet Root",
        )?;
    }
    Ok(())
}

struct CompileDesiredRequest<'a> {
    request: &'a FleetGenerateRequest<'a>,
    source: &'a FleetSource,
    seed: &'a EstateSeed,
    admission: FleetAdmissionPolicyTemplate,
    app: canic_core::ids::AppId,
    deployment_configuration: ComponentDeploymentConfiguration,
    topology: crate::component_topology::PlannedFleetTopology,
    coordinator_artifact: &'a CanicInfrastructureArtifactEntry,
    root_artifact: &'a CanicInfrastructureArtifactEntry,
    store_artifact: &'a CanicInfrastructureArtifactEntry,
    coordinator_candid: &'a str,
    root_candid: &'a str,
    store_candid: &'a str,
    observed: &'a BTreeMap<String, ObservedCanister>,
    treasury: &'a str,
    ledger_fee_cycles: u128,
}

struct RootGenerationBinding<'a> {
    source: &'a RootSource,
    seed: &'a RootSeed,
    planned: &'a PlannedFleetSubnetRootTopology,
}

fn bind_root_generation_inputs<'a>(
    sources: &'a [RootSource],
    seeds: &'a [RootSeed],
    planned_roots: &'a [PlannedFleetSubnetRootTopology],
) -> Result<Vec<RootGenerationBinding<'a>>, FleetGenerateError> {
    let mut sources_by_subnet = BTreeMap::new();
    for source in sources {
        let placement = parse_subnet("Fleet policy Root", &source.placement_subnet)?;
        if sources_by_subnet.insert(placement, source).is_some() {
            return Err(FleetGenerateError::SeedTopology(format!(
                "Fleet policy repeats Root placement Subnet {placement}"
            )));
        }
    }
    let mut seeds_by_subnet = BTreeMap::new();
    for seed in seeds {
        let placement = parse_subnet("Fleet identity-seed Root", &seed.placement_subnet)?;
        if seeds_by_subnet.insert(placement, seed).is_some() {
            return Err(FleetGenerateError::SeedTopology(format!(
                "Fleet identity seed repeats Root placement Subnet {placement}"
            )));
        }
    }
    let mut planned_by_subnet = BTreeMap::new();
    for planned in planned_roots {
        if planned_by_subnet
            .insert(planned.placement_subnet, planned)
            .is_some()
        {
            return Err(FleetGenerateError::SeedTopology(format!(
                "compiled topology repeats Root placement Subnet {}",
                planned.placement_subnet
            )));
        }
    }
    let source_subnets = sources_by_subnet.keys().copied().collect::<BTreeSet<_>>();
    let seed_subnets = seeds_by_subnet.keys().copied().collect::<BTreeSet<_>>();
    let planned_subnets = planned_by_subnet.keys().copied().collect::<BTreeSet<_>>();
    if source_subnets != seed_subnets || source_subnets != planned_subnets {
        return Err(FleetGenerateError::SeedTopology(
            "Fleet policy, identity-seed and compiled Root placement Subnet sets differ"
                .to_string(),
        ));
    }
    Ok(planned_by_subnet
        .into_iter()
        .map(|(placement, planned)| RootGenerationBinding {
            source: sources_by_subnet[&placement],
            seed: seeds_by_subnet[&placement],
            planned,
        })
        .collect())
}

#[expect(
    clippy::too_many_lines,
    reason = "one compiler constructs the complete canonical desired document"
)]
fn compile_desired(input: CompileDesiredRequest<'_>) -> Result<DesiredFleet, FleetGenerateError> {
    let mut canisters = Vec::new();
    canisters.push(infrastructure_canister(
        "coordinator",
        DesiredCanisterKind::Coordinator,
        Some(DesiredCanisterInit::Coordinator),
        (!input.seed.fresh_estate).then_some(input.seed.coordinator.as_str()),
        None,
        &input.source.operator,
        &input.source.coordinator.subnet.subnet,
        input.source.coordinator.creation_funding.cycles.to_u128(),
        input
            .source
            .coordinator
            .root_funding
            .minimum_reserve_cycles
            .to_u128(),
        input.coordinator_artifact,
    ));
    let mut bootstrap_roots = Vec::new();
    let mut placements = Vec::new();
    let root_bindings = bind_root_generation_inputs(
        &input.source.fleet_subnet_roots,
        &input.seed.roots,
        &input.topology.fleet_subnet_roots,
    )?;
    for (index, binding) in root_bindings.into_iter().enumerate() {
        let RootGenerationBinding {
            source,
            seed,
            planned,
        } = binding;
        require_cycles_creation(&source.root_creation_funding, "Fleet Subnet Root")?;
        require_cycles_creation(&source.wasm_store_creation_funding, "Wasm Store")?;
        let root_name = format!("root-{index}");
        let store_name = format!("store-{index}");
        canisters.push(infrastructure_canister(
            &root_name,
            DesiredCanisterKind::Root,
            Some(DesiredCanisterInit::Root {
                root: root_name.clone(),
            }),
            (!input.seed.fresh_estate).then_some(seed.root.as_str()),
            Some("coordinator"),
            &input.source.operator,
            &source.placement_subnet,
            source.root_creation_funding.cycles.to_u128(),
            source.root_funding.request_threshold.to_u128(),
            input.root_artifact,
        ));
        let mut store = infrastructure_canister(
            &store_name,
            DesiredCanisterKind::Store,
            Some(DesiredCanisterInit::Store {
                root: root_name.clone(),
            }),
            (!input.seed.fresh_estate).then_some(seed.store.as_str()),
            Some(&root_name),
            &input.source.operator,
            &source.placement_subnet,
            source.wasm_store_creation_funding.cycles.to_u128(),
            0,
            input.store_artifact,
        );
        if input.seed.fresh_estate {
            store.controller_canisters.push(root_name.clone());
        } else {
            store.controllers.push(seed.root.clone());
            store.controllers.sort();
        }
        canisters.push(store);
        let mut pool_names = Vec::new();
        for (pool_index, pool) in seed.pool_imports.iter().enumerate() {
            let name = format!("root-{index}-pool-{pool_index}");
            pool_names.push(name.clone());
            canisters.push(DesiredCanister {
                canic_init: None,
                controller_canisters: if input.seed.fresh_estate {
                    vec![root_name.clone()]
                } else {
                    Vec::new()
                },
                controllers: if input.seed.fresh_estate {
                    Vec::new()
                } else {
                    vec![seed.root.clone()]
                },
                drain: None,
                initial_cycles: source.canister_pool.canister_cycles.to_config_string(),
                init_arg: None,
                init_candid: None,
                kind: DesiredCanisterKind::Pool,
                minimum_cycles: source.canister_pool.canister_cycles.to_config_string(),
                name,
                parent: Some(root_name.clone()),
                presence: DesiredPresence::Present,
                principal: (!input.seed.fresh_estate).then(|| pool.clone()),
                protocol_binding: None,
                replace: false,
                subnet: source.placement_subnet.clone(),
                wasm: None,
            });
        }
        if seed.pool_imports != source.canister_pool.imports
            && !source.canister_pool.imports.is_empty()
        {
            return Err(FleetGenerateError::SeedTopology(format!(
                "Root {} pool imports differ between policy source and identity seed",
                source.placement_subnet
            )));
        }
        bootstrap_roots.push(DesiredFleetBootstrapRoot {
            canister_pool_imports: pool_names,
            component_admissions: planned.component_admissions.clone(),
            component_topology_digest: planned.component_topology_digest,
            funding: root_funding(input.source.funding_profile, &source.root_funding),
            limits: root_limits(source),
            placement_subnet: planned.placement_subnet,
            root: root_name.clone(),
            store: store_name,
        });
        for (deployment, ordinals) in &source.component_group_placements {
            for ordinal in ordinals {
                placements.push(DesiredComponentGroupPlacement {
                    deployment: deployment.to_string(),
                    ordinal: *ordinal,
                    root: root_name.clone(),
                });
            }
        }
    }
    if !input.seed.fresh_estate && input.treasury != input.seed.coordinator {
        let treasury_observation = input
            .observed
            .get(input.treasury)
            .expect("verified treasury observation");
        canisters.push(DesiredCanister {
            canic_init: None,
            controller_canisters: Vec::new(),
            controllers: vec![input.source.operator.clone()],
            drain: None,
            initial_cycles: config_cycles(0),
            init_arg: None,
            init_candid: None,
            kind: DesiredCanisterKind::Auxiliary,
            minimum_cycles: config_cycles(0),
            name: "treasury".to_string(),
            parent: None,
            presence: DesiredPresence::Present,
            principal: Some(input.treasury.to_string()),
            protocol_binding: None,
            replace: false,
            subnet: treasury_observation.subnet.clone(),
            wasm: None,
        });
    }
    let app_config = relative_path(input.request.root, input.request.app_config)?;
    let app = input.app.clone();
    Ok(DesiredFleet {
        bootstrap: Some(DesiredFleetBootstrap {
            admission: input.admission,
            app,
            canonical_network_id: canonical_network_id(input.request, input.source)?,
            component_deployment_configuration: input.deployment_configuration,
            coordinator: "coordinator".to_string(),
            coordinator_subnet: parse_subnet(
                "Coordinator",
                &input.source.coordinator.subnet.subnet,
            )?,
            fleet_id: input.seed.fleet_id,
            fresh_estate: input.seed.fresh_estate,
            release_build_id: input.request.release_build_id,
            root_funding: Some(coordinator_funding(
                input.source.funding_profile,
                &input.source.coordinator.root_funding,
            )),
            roots: bootstrap_roots,
        }),
        canisters,
        cycles_ledger: input.seed.cycles_ledger.clone(),
        environment: input.request.environment.to_string(),
        fleet: input.request.fleet.to_string(),
        ledger_fee_cycles: config_cycles(input.ledger_fee_cycles),
        // Retained seeds bind paid canisters to observed Principals and carry no creation fee.
        // Fresh seeds retain their explicit fee before any generated Principal exists.
        management_creation_fee_cycles: input
            .seed
            .management_creation_fee_cycles
            .clone()
            .unwrap_or_else(|| config_cycles(0)),
        material_cycle_threshold: config_cycles(GENERATED_RETAINED_MATERIAL_CYCLE_THRESHOLD),
        maximum_observation_burn_cycles: config_cycles(
            GENERATED_RETAINED_MAXIMUM_OBSERVATION_BURN_CYCLES,
        ),
        maximum_stalled_observations: 8,
        maximum_update_burn_cycles: config_cycles(GENERATED_RETAINED_MAXIMUM_UPDATE_BURN_CYCLES),
        operator: input.source.operator.clone(),
        protocol: Some(DesiredFleetProtocol {
            app_config,
            component_group_placements: placements,
            coordinator_candid: input.coordinator_candid.to_string(),
            root_candid: input.root_candid.to_string(),
            store_candid: input.store_candid.to_string(),
        }),
        protocol_steps: Vec::new(),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        treasury: if input.treasury == input.seed.coordinator {
            "coordinator".to_string()
        } else {
            "treasury".to_string()
        },
    })
}

fn observe_ledger_fee(
    request: &FleetGenerateRequest<'_>,
    cycles_ledger: &str,
) -> Result<u128, FleetGenerateError> {
    parse_principal("Cycles Ledger", cycles_ledger)?;
    let icp = IcpCli::new(
        request.icp_executable,
        Some(request.environment.to_string()),
    )
    .with_cwd(request.root.to_path_buf());
    let value: Nat = icp
        .canister_query_candid(cycles_ledger, "icrc1_fee", &(), None)
        .map_err(|error| {
            FleetGenerateError::Authority(format!(
                "failed to observe the exact Cycles Ledger fee: {error}"
            ))
        })?;
    let rendered = value.to_string();
    u128::try_from(value.0).map_err(|_| {
        FleetGenerateError::Authority(format!(
            "live Cycles Ledger fee exceeds the supported cycle range: {rendered}"
        ))
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "one observation boundary validates identity, controllers, placement, and cycles together"
)]
fn observe_estate(
    input: &EstateObservationRequest<'_, '_>,
) -> Result<BTreeMap<String, ObservedCanister>, FleetGenerateError> {
    let EstateObservationRequest {
        app,
        generation: request,
        operator,
        root_candid,
        root_wasm_sha256,
        seed,
        source,
        topology,
    } = input;
    let icp = IcpCli::new(
        request.icp_executable,
        Some(request.environment.to_string()),
    )
    .with_cwd(request.root.to_path_buf());
    let active = icp
        .identity_principal_text()
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    if active != operator.to_text() {
        return Err(FleetGenerateError::Authority(format!(
            "active identity {active} differs from protected operator {operator}"
        )));
    }
    let network = resolve_icp_build_network_from_root(request.root, request.environment)
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    let catalog = if network == BuildNetwork::Ic {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| FleetGenerateError::Clock)?
            .as_secs();
        Some(
            load_mainnet_subnet_catalog(request.root, now)
                .map_err(|error| FleetGenerateError::Authority(error.to_string()))?,
        )
    } else {
        None
    };
    if let Some(catalog) = &catalog {
        require_fiduciary_acknowledgement(
            &catalog.catalog,
            &source.coordinator.subnet.subnet,
            source.coordinator.subnet.acknowledge_fiduciary_cost,
            "Coordinator",
        )?;
        for root in &source.fleet_subnet_roots {
            require_fiduciary_acknowledgement(
                &catalog.catalog,
                &root.placement_subnet,
                root.acknowledge_fiduciary_cost,
                "Fleet Subnet Root",
            )?;
        }
    }
    let mut expected = BTreeMap::new();
    insert_expected_canister(
        &mut expected,
        seed.coordinator.clone(),
        (
            source.coordinator.subnet.subnet.clone(),
            vec![source.operator.clone()],
        ),
        "Coordinator",
    )?;
    if let Some(treasury) = &seed.treasury
        && treasury.principal != seed.coordinator
    {
        insert_expected_canister(
            &mut expected,
            treasury.principal.clone(),
            (treasury.subnet.clone(), vec![source.operator.clone()]),
            "treasury",
        )?;
    }
    let mut stopped_roots = Vec::new();
    for root in &seed.roots {
        insert_expected_canister(
            &mut expected,
            root.root.clone(),
            (root.placement_subnet.clone(), vec![source.operator.clone()]),
            "Fleet Subnet Root",
        )?;
    }
    let mut observed = BTreeMap::new();
    let root_principals = seed
        .roots
        .iter()
        .map(|root| root.root.as_str())
        .collect::<BTreeSet<_>>();
    let mut root_statuses = BTreeMap::new();
    for (canister, (expected_subnet, required_controllers)) in expected {
        parse_principal("retained canister", &canister)?;
        let status = icp.canister_status_report(&canister).map_err(|error| {
            FleetGenerateError::CanisterUnavailable {
                canister: canister.clone(),
                reason: error.to_string(),
            }
        })?;
        if status.id != canister {
            return Err(FleetGenerateError::SeedTopology(format!(
                "retained canister query for {canister} returned identity {}",
                status.id
            )));
        }
        let controllers = status
            .settings
            .map(|settings| settings.controllers)
            .unwrap_or_default();
        require_exact_controllers(&canister, controllers, required_controllers)?;
        let subnet = if let Some(catalog) = &catalog {
            catalog
                .catalog
                .resolve_canister_route(&status.id)
                .map_err(|error| FleetGenerateError::Authority(error.to_string()))?
                .subnet
                .to_text()
        } else {
            expected_subnet.clone()
        };
        if subnet != expected_subnet {
            return Err(FleetGenerateError::SubnetMismatch {
                actual: subnet,
                canister,
                expected: expected_subnet,
            });
        }
        let cycles = status
            .cycles
            .as_deref()
            .unwrap_or("0")
            .replace('_', "")
            .parse::<u128>()
            .map_err(|_| FleetGenerateError::Authority("invalid live cycle balance".to_string()))?;
        let module_sha256 = status
            .module_hash
            .as_deref()
            .map(|hash| normalize_observed_module_sha256(&canister, hash))
            .transpose()?;
        if root_principals.contains(canister.as_str()) {
            let runtime_status = parse_observed_runtime_status(&canister, &status.status)?;
            if module_sha256.is_none() {
                return Err(FleetGenerateError::CanisterUnavailable {
                    canister,
                    reason: "retained Root has no installed module SHA-256".to_string(),
                });
            }
            root_statuses.insert(status.id.clone(), runtime_status);
        }
        if observed
            .insert(
                status.id.clone(),
                ObservedCanister {
                    cycles,
                    module_sha256,
                    subnet,
                },
            )
            .is_some()
        {
            return Err(FleetGenerateError::SeedTopology(format!(
                "retained identity {} is assigned to more than one role",
                status.id
            )));
        }
    }
    for root in &seed.roots {
        match root_statuses.get(&root.root) {
            Some(CanisterRuntimeStatus::Running) => {}
            Some(CanisterRuntimeStatus::Stopped) => {
                let observation = observed.get(&root.root).ok_or_else(|| {
                    FleetGenerateError::SeedTopology(format!(
                        "retained Root {} has no exact management observation",
                        root.root
                    ))
                })?;
                let module_sha256 = observation.module_sha256.clone().ok_or_else(|| {
                    FleetGenerateError::CanisterUnavailable {
                        canister: root.root.clone(),
                        reason: "retained Root has no installed module SHA-256".to_string(),
                    }
                })?;
                let root_name = retained_root_name(source, seed, topology, &root.root)?;
                stopped_roots.push(RetainedRootStartBinding {
                    controllers: vec![operator.to_text()],
                    name: root_name,
                    predecessor_module_sha256: module_sha256,
                    principal: root.root.clone(),
                    subnet: observation.subnet.clone(),
                });
            }
            Some(CanisterRuntimeStatus::Stopping) => {
                return Err(FleetGenerateError::CanisterUnavailable {
                    canister: root.root.clone(),
                    reason: "retained Root is stopping; wait for terminal Stopped state and rerun generation"
                        .to_string(),
                });
            }
            None => {
                return Err(FleetGenerateError::SeedTopology(format!(
                    "retained Root {} has no runtime-status observation",
                    root.root
                )));
            }
        }
    }
    if let Some(first) = stopped_roots.first() {
        let authority = write_root_start_authority(
            &EnsurePaths::under(request.root, request.environment, request.fleet),
            RetainedRootStartAuthorityRecord {
                authority_sha256: String::new(),
                environment: request.environment.to_string(),
                fleet: request.fleet.to_string(),
                fleet_id: seed.fleet_id,
                release_build_id: request.release_build_id,
                roots: stopped_roots.clone(),
                schema_version: FLEET_ENSURE_SCHEMA_VERSION,
                successor_module_sha256: (*root_wasm_sha256).to_string(),
            },
        )
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
        return Err(FleetGenerateError::StoppedRootStartRequired(Box::new(
            StoppedRootStartPrerequisite {
                authority_sha256: authority.authority_sha256,
                controller: operator.to_text(),
                fleet: request.fleet.to_string(),
                module_sha256: first.predecessor_module_sha256.clone(),
                root: first.principal.clone(),
                root_count: stopped_roots.len(),
                subnet: first.subnet.clone(),
                successor_module_sha256: (*root_wasm_sha256).to_string(),
            },
        )));
    }
    let predecessor_status_roots = predecessor_status_roots(
        request,
        *operator,
        seed,
        source,
        topology,
        &observed,
        root_wasm_sha256,
    )?;
    observe_root_owned_pool_assets(
        &icp,
        &RootEstateAuthorityRequest {
            app,
            generation: request,
            root_candid,
            root_wasm_sha256,
            predecessor_status_roots: &predecessor_status_roots,
            seed,
            source,
            topology,
        },
        &mut observed,
    )?;
    Ok(observed)
}

fn predecessor_status_roots(
    request: &FleetGenerateRequest<'_>,
    operator: Principal,
    seed: &EstateSeed,
    source: &FleetSource,
    topology: &PlannedFleetTopology,
    observed: &BTreeMap<String, ObservedCanister>,
    successor_module_sha256: &str,
) -> Result<BTreeSet<String>, FleetGenerateError> {
    let paths = EnsurePaths::under(request.root, request.environment, request.fleet);
    let Some(authority) = read_root_start_authority(&paths)
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?
    else {
        return Ok(BTreeSet::new());
    };
    let has_matching_predecessor = authority.roots.iter().any(|binding| {
        observed
            .get(&binding.principal)
            .and_then(|canister| canister.module_sha256.as_deref())
            == Some(binding.predecessor_module_sha256.as_str())
    });
    if !has_matching_predecessor {
        return Ok(BTreeSet::new());
    }
    let identity_matches = authority.environment == request.environment
        && authority.fleet == request.fleet
        && authority.fleet_id == seed.fleet_id
        && authority.release_build_id == request.release_build_id
        && authority.successor_module_sha256 == successor_module_sha256;
    if !identity_matches {
        return Err(FleetGenerateError::Authority(
            "retained Root-start authority does not bind the requested successor generation"
                .to_string(),
        ));
    }
    verify_root_start_release_authority(request.root, &authority)
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))?;
    let expected_controller = operator.to_text();
    let mut accepted = BTreeSet::new();
    for binding in &authority.roots {
        let root = seed
            .roots
            .iter()
            .find(|root| root.root == binding.principal)
            .ok_or_else(|| {
                FleetGenerateError::Authority(format!(
                    "retained Root-start authority names unknown Root {}",
                    binding.principal
                ))
            })?;
        let expected_name = retained_root_name(source, seed, topology, &root.root)?;
        let exact_binding = binding.name == expected_name
            && binding.subnet == root.placement_subnet
            && binding.controllers == [expected_controller.clone()];
        if !exact_binding {
            return Err(FleetGenerateError::Authority(format!(
                "retained predecessor status authority for Root {} conflicts with current Fleet identity",
                binding.principal
            )));
        }
        let live_module = observed
            .get(&binding.principal)
            .and_then(|canister| canister.module_sha256.as_deref())
            .ok_or_else(|| {
                FleetGenerateError::Authority(format!(
                    "retained predecessor status authority has no live module for Root {}",
                    binding.principal
                ))
            })?;
        if live_module == binding.predecessor_module_sha256 {
            accepted.insert(binding.principal.clone());
        } else if live_module != successor_module_sha256 {
            return Err(FleetGenerateError::Authority(format!(
                "Root {} module is neither the sealed predecessor nor successor",
                binding.principal
            )));
        }
    }
    Ok(accepted)
}

fn retained_root_name(
    source: &FleetSource,
    seed: &EstateSeed,
    topology: &PlannedFleetTopology,
    principal: &str,
) -> Result<String, FleetGenerateError> {
    bind_root_generation_inputs(
        &source.fleet_subnet_roots,
        &seed.roots,
        &topology.fleet_subnet_roots,
    )?
    .into_iter()
    .enumerate()
    .find_map(|(index, binding)| (binding.seed.root == principal).then(|| format!("root-{index}")))
    .ok_or_else(|| {
        FleetGenerateError::SeedTopology(format!(
            "retained Root {principal} is absent from the compiled topology"
        ))
    })
}

fn parse_observed_runtime_status(
    canister: &str,
    value: &str,
) -> Result<CanisterRuntimeStatus, FleetGenerateError> {
    match value.to_ascii_lowercase().as_str() {
        "running" => Ok(CanisterRuntimeStatus::Running),
        "stopped" => Ok(CanisterRuntimeStatus::Stopped),
        "stopping" => Ok(CanisterRuntimeStatus::Stopping),
        _ => Err(FleetGenerateError::CanisterUnavailable {
            canister: canister.to_string(),
            reason: format!("unknown management runtime status {value:?}"),
        }),
    }
}

fn normalize_observed_module_sha256(
    canister: &str,
    value: &str,
) -> Result<String, FleetGenerateError> {
    let normalized = value.trim_start_matches("0x").to_ascii_lowercase();
    if normalized.len() != 64 || !normalized.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(FleetGenerateError::CanisterUnavailable {
            canister: canister.to_string(),
            reason: format!("invalid installed module SHA-256 {value:?}"),
        });
    }
    Ok(normalized)
}

fn require_root_estate_authority(
    root: &str,
    actual: &FleetSubnetRootAuthority,
    expected: &ExpectedRootEstateAuthority,
) -> Result<(), FleetGenerateError> {
    if actual.binding.authority.binding != expected.coordinator {
        return Err(FleetGenerateError::SeedTopology(format!(
            "Root {root} live Coordinator authority differs from protected policy"
        )));
    }
    if RootIdentityRelationship::from(&actual.binding) != expected.root {
        return Err(FleetGenerateError::SeedTopology(format!(
            "Root {root} live identity or placement differs from the identity seed"
        )));
    }
    if WasmStoreIdentityRelationship::from(&actual.wasm_store_authority) != expected.store {
        return Err(FleetGenerateError::SeedTopology(format!(
            "Root {root} live Store relationship differs from the identity seed"
        )));
    }
    Ok(())
}

fn require_root_policy_convergence(
    root: &str,
    actual: &RootDesiredPolicy,
    expected: &RootDesiredPolicy,
    observed_module_sha256: Option<&str>,
    desired_module_sha256: &str,
) -> Result<(), FleetGenerateError> {
    if actual != expected && observed_module_sha256 == Some(desired_module_sha256) {
        return Err(FleetGenerateError::SeedTopology(format!(
            "Root {root} policy differs but its current module cannot converge that init-only drift without a reinstall"
        )));
    }
    Ok(())
}

fn require_fiduciary_acknowledgement(
    catalog: &ic_query::subnet_catalog::ValidatedSubnetCatalog,
    subnet: &str,
    acknowledged: bool,
    role: &str,
) -> Result<(), FleetGenerateError> {
    let info = catalog.subnet_by_principal(subnet).ok_or_else(|| {
        FleetGenerateError::Authority(format!(
            "{role} Subnet {subnet} is absent from validated Registry evidence"
        ))
    })?;
    let fiduciary = info.subnet_specialization == SubnetSpecialization::Fiduciary;
    if fiduciary != acknowledged {
        return Err(FleetGenerateError::Authority(format!(
            "{role} Subnet {subnet} Fiduciary cost acknowledgement differs from validated Registry classification"
        )));
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "bounded Root inventory observation validates one complete authority tuple and page stream"
)]
fn observe_root_owned_pool_assets(
    icp: &IcpCli,
    request: &RootEstateAuthorityRequest<'_, '_>,
    observed: &mut BTreeMap<String, ObservedCanister>,
) -> Result<(), FleetGenerateError> {
    let expected_network = canonical_network_id(request.generation, request.source)?;
    let expected_fleet = request.seed.fleet_id;
    let expected_coordinator = parse_principal("Fleet Coordinator", &request.seed.coordinator)?;
    let expected_coordinator_subnet = parse_subnet(
        "Fleet Coordinator",
        &request.source.coordinator.subnet.subnet,
    )?;
    for root in &request.seed.roots {
        let source_root = request
            .source
            .fleet_subnet_roots
            .iter()
            .find(|candidate| candidate.placement_subnet == root.placement_subnet)
            .ok_or_else(|| {
                FleetGenerateError::SeedTopology(format!(
                    "Root {} has no policy source",
                    root.placement_subnet
                ))
            })?;
        let planned = request
            .topology
            .fleet_subnet_roots
            .iter()
            .find(|candidate| candidate.placement_subnet.to_string() == root.placement_subnet)
            .ok_or_else(|| {
                FleetGenerateError::SeedTopology(format!(
                    "Root {} has no compiled topology authority",
                    root.placement_subnet
                ))
            })?;
        let expected = std::iter::once(root.store.as_str())
            .chain(root.pool_imports.iter().map(String::as_str))
            .collect::<std::collections::BTreeSet<_>>();
        let root_principal = parse_principal("Fleet Subnet Root", &root.root)?;
        let response: RootEstateStatusResponse = query_with_candid(
            icp,
            request.root_candid,
            root_principal,
            protocol::CANIC_STATUS,
            &RootEstateStatusRequest::FleetAuthority,
        )
        .map_err(|error| FleetGenerateError::CanisterUnavailable {
            canister: root.root.clone(),
            reason: format!("protected Root authority observation failed: {error}"),
        })?;
        let RootEstateStatusResponse::FleetAuthority(authority) = response else {
            return Err(FleetGenerateError::SeedTopology(format!(
                "Root {} returned the wrong authority projection",
                root.root
            )));
        };
        let expected_coordinator = FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: expected_network,
                    fleet_id: expected_fleet,
                },
                app: request.app.clone(),
            },
            coordinator_subnet: expected_coordinator_subnet,
            coordinator: expected_coordinator,
        };
        let expected_registry_authority = FleetRegistryAuthority {
            binding: expected_coordinator.clone(),
            epoch: authority.binding.authority.epoch,
        };
        let expected_authority = ExpectedRootEstateAuthority {
            coordinator: expected_coordinator,
            root: RootIdentityRelationship {
                authority: expected_registry_authority.clone(),
                placement_subnet: planned.placement_subnet,
                fleet_subnet_root: root_principal,
            },
            store: WasmStoreIdentityRelationship {
                authority: expected_registry_authority,
                placement_subnet: planned.placement_subnet,
                fleet_subnet_root: root_principal,
                wasm_store: parse_principal("Wasm Store", &root.store)?,
            },
        };
        require_root_estate_authority(&root.root, &authority, &expected_authority)?;
        let expected_policy = RootDesiredPolicy {
            component_admissions: planned.component_admissions.clone(),
            component_topology_digest: planned.component_topology_digest,
            funding: root_funding(request.source.funding_profile, &source_root.root_funding),
            installation_controller: parse_principal("operator", &request.source.operator)?,
            limits: planned.limits.clone(),
        };
        require_root_policy_convergence(
            &root.root,
            &RootDesiredPolicy::from(authority.as_ref()),
            &expected_policy,
            observed
                .get(&root.root)
                .and_then(|value| value.module_sha256.as_deref()),
            request.root_wasm_sha256,
        )?;
        let mut found = BTreeMap::new();
        let mut start_after = None;
        loop {
            let status_request = RootEstateStatusRequest::Pool(CanisterPoolStatusRequest {
                start_after,
                limit: 256,
            });
            let page = if request.predecessor_status_roots.contains(&root.root) {
                predecessor_root_status::query_pool(icp, root_principal, start_after, 256).map_err(
                    |error| FleetGenerateError::CanisterUnavailable {
                        canister: root.root.clone(),
                        reason: format!(
                            "protected predecessor Root pool observation failed: {error}"
                        ),
                    },
                )?
            } else {
                let response: RootEstateStatusResponse = query_with_candid(
                    icp,
                    request.root_candid,
                    root_principal,
                    protocol::CANIC_STATUS,
                    &status_request,
                )
                .map_err(|error| FleetGenerateError::CanisterUnavailable {
                    canister: root.root.clone(),
                    reason: format!("protected Root pool observation failed: {error}"),
                })?;
                let RootEstateStatusResponse::Pool(page) = response else {
                    return Err(FleetGenerateError::SeedTopology(format!(
                        "Root {} returned the wrong pool projection",
                        root.root
                    )));
                };
                *page
            };
            if page.config != authority.binding.limits.canister_pool {
                return Err(FleetGenerateError::SeedTopology(format!(
                    "Root {} live pool policy conflicts with its retained authority",
                    root.root
                )));
            }
            for asset in page.entries {
                let principal = asset.canister_id.to_text();
                if !expected.contains(principal.as_str()) {
                    return Err(FleetGenerateError::SeedTopology(format!(
                        "Root {} retains unseeded controlled pool asset {principal}",
                        root.root
                    )));
                }
                if principal == root.store
                    && root_owned_lifecycle(DesiredCanisterKind::Store, &asset.status).is_none()
                {
                    return Err(FleetGenerateError::SeedTopology(format!(
                        "Root {} does not retain {} as its exact Store",
                        root.root, root.store
                    )));
                }
                if principal != root.store
                    && root_owned_lifecycle(DesiredCanisterKind::Pool, &asset.status).is_none()
                {
                    return Err(FleetGenerateError::SeedTopology(format!(
                        "Root {} pool identity {principal} is outside the retained bootstrap/workload lifecycle",
                        root.root
                    )));
                }
                if found.insert(principal, asset.cycles.to_u128()).is_some() {
                    return Err(FleetGenerateError::SeedTopology(format!(
                        "Root {} repeats a pool identity",
                        root.root
                    )));
                }
            }
            let next = page.next_start_after;
            if next.is_none() {
                break;
            }
            if next == start_after {
                return Err(FleetGenerateError::SeedTopology(format!(
                    "Root {} pool cursor did not advance",
                    root.root
                )));
            }
            start_after = next;
        }
        for principal in expected {
            let cycles = found.get(principal).copied().ok_or_else(|| {
                FleetGenerateError::SeedTopology(format!(
                    "Root {} does not retain seeded identity {principal}",
                    root.root
                ))
            })?;
            if let Some(entry) = observed.get(principal) {
                if entry.cycles != cycles {
                    return Err(FleetGenerateError::Authority(format!(
                        "Root-owned and direct cycle observations differ for {principal}"
                    )));
                }
            } else {
                observed.insert(
                    principal.to_string(),
                    ObservedCanister {
                        cycles,
                        module_sha256: None,
                        subnet: root.placement_subnet.clone(),
                    },
                );
            }
        }
    }
    Ok(())
}

fn validate_identity_seed(
    source: &FleetSource,
    seed: &EstateSeed,
) -> Result<(), FleetGenerateError> {
    if seed.fresh_estate {
        let fee = seed
            .management_creation_fee_cycles
            .as_deref()
            .ok_or_else(|| {
                FleetGenerateError::FreshSeedConflict(
                    "fresh estate seed is missing its exact management creation fee".to_string(),
                )
            })?;
        let fee = Cycles::from_human_config_str(fee)
            .map_err(|_| {
                FleetGenerateError::FreshSeedConflict(
                    "fresh estate seed management creation fee must use B, T, or Q units"
                        .to_string(),
                )
            })?
            .to_u128();
        let expected = fresh_seed(source, seed.fleet_id, &seed.cycles_ledger, fee)?;
        if seed == &expected {
            return Ok(());
        }
        return Err(FleetGenerateError::FreshSeedConflict(
            "fresh estate seed differs from current protected topology".to_string(),
        ));
    }
    if seed.management_creation_fee_cycles.is_some() {
        return Err(FleetGenerateError::SeedTopology(
            "retained estate seed must not declare fresh creation fee authority".to_string(),
        ));
    }
    let mut identities = BTreeSet::new();
    insert_seed_identity(&mut identities, "Coordinator", &seed.coordinator)?;
    if let Some(treasury) = &seed.treasury {
        parse_subnet("treasury", &treasury.subnet)?;
        if treasury.principal != seed.coordinator {
            insert_seed_identity(&mut identities, "treasury", &treasury.principal)?;
        } else if treasury.subnet != source.coordinator.subnet.subnet {
            return Err(FleetGenerateError::SeedTopology(
                "Coordinator treasury Subnet differs from Coordinator placement".to_string(),
            ));
        }
    }
    let source_roots = source
        .fleet_subnet_roots
        .iter()
        .map(|root| root.placement_subnet.as_str())
        .collect::<BTreeSet<_>>();
    let seed_roots = seed
        .roots
        .iter()
        .map(|root| root.placement_subnet.as_str())
        .collect::<BTreeSet<_>>();
    if source_roots.len() != source.fleet_subnet_roots.len()
        || seed_roots.len() != seed.roots.len()
        || source_roots != seed_roots
    {
        return Err(FleetGenerateError::SeedTopology(
            "Root placement Subnet sets differ or contain duplicates".to_string(),
        ));
    }
    for root in &seed.roots {
        let source_root = source
            .fleet_subnet_roots
            .iter()
            .find(|candidate| candidate.placement_subnet == root.placement_subnet)
            .expect("validated Root placement");
        insert_seed_identity(&mut identities, "Fleet Subnet Root", &root.root)?;
        insert_seed_identity(&mut identities, "Wasm Store", &root.store)?;
        for pool in &root.pool_imports {
            insert_seed_identity(&mut identities, "pool import", pool)?;
        }
        if !source_root.canister_pool.imports.is_empty()
            && source_root.canister_pool.imports != root.pool_imports
        {
            return Err(FleetGenerateError::SeedTopology(format!(
                "Root {} pool imports differ between policy source and identity seed",
                root.placement_subnet
            )));
        }
    }
    Ok(())
}

fn insert_seed_identity(
    identities: &mut BTreeSet<String>,
    role: &str,
    principal: &str,
) -> Result<(), FleetGenerateError> {
    parse_principal(role, principal)?;
    if !identities.insert(principal.to_string()) {
        return Err(FleetGenerateError::SeedTopology(format!(
            "Principal {principal} is assigned to more than one retained role"
        )));
    }
    Ok(())
}

fn insert_expected_canister(
    expected: &mut BTreeMap<String, (String, Vec<String>)>,
    principal: String,
    authority: (String, Vec<String>),
    role: &str,
) -> Result<(), FleetGenerateError> {
    if expected.insert(principal.clone(), authority).is_some() {
        return Err(FleetGenerateError::SeedTopology(format!(
            "{role} Principal {principal} is assigned more than once"
        )));
    }
    Ok(())
}

fn require_exact_controllers(
    canister: &str,
    mut actual: Vec<String>,
    mut expected: Vec<String>,
) -> Result<(), FleetGenerateError> {
    actual.sort();
    expected.sort();
    if actual != expected {
        return Err(FleetGenerateError::ControllerMismatch {
            actual,
            canister: canister.to_string(),
            expected,
        });
    }
    Ok(())
}

fn release_authority(
    request: &FleetGenerateRequest<'_>,
    component_topology: &canic_core::control_plane_support::config::ComponentTopology,
) -> Result<
    (
        crate::release_set::PersistedCanicInfrastructureArtifactManifest,
        crate::release_set::PersistedCurrentReleaseSetManifest,
    ),
    FleetGenerateError,
> {
    let complete =
        load_persisted_current_release_set_manifest(request.root, request.release_build_id)
            .map_err(|error| FleetGenerateError::Release(error.to_string()))?;
    let expected_network = resolve_icp_build_network_from_root(request.root, request.environment)
        .map_err(|error| FleetGenerateError::Release(error.to_string()))?;
    require_release_build_network(
        request.release_build_id,
        complete.manifest.build_network,
        request.environment,
        expected_network,
    )?;
    validate_finalized_release_build_manifest(
        request.root,
        request.release_build_id,
        &complete.path,
    )
    .map_err(|error| FleetGenerateError::Release(error.to_string()))?;
    let infrastructure = load_persisted_canic_infrastructure_artifact_manifest(
        request.root,
        request.release_build_id,
    )
    .map_err(|error| FleetGenerateError::Release(error.to_string()))?;
    let application = load_persisted_application_artifact_union(
        request.root,
        component_topology,
        request.release_build_id,
    )
    .map_err(|error| FleetGenerateError::Release(error.to_string()))?;
    if complete.manifest.application_artifact_union_sha256 != application.digest {
        return Err(FleetGenerateError::Release(
            "complete release set does not bind its application artifact union".to_string(),
        ));
    }
    Ok((infrastructure, complete))
}

fn require_release_build_network(
    release_build_id: ReleaseBuildId,
    actual: BuildNetwork,
    environment: &str,
    expected: BuildNetwork,
) -> Result<(), FleetGenerateError> {
    if actual == expected {
        return Ok(());
    }
    Err(FleetGenerateError::Release(format!(
        "release build {release_build_id} targets {actual}, but environment {environment} requires {expected}",
    )))
}

#[expect(
    clippy::too_many_arguments,
    reason = "single constructor makes every authority-bearing desired canister field explicit"
)]
fn infrastructure_canister(
    name: &str,
    kind: DesiredCanisterKind,
    canic_init: Option<DesiredCanisterInit>,
    principal: Option<&str>,
    parent: Option<&str>,
    operator: &str,
    subnet: &str,
    initial_cycles: u128,
    minimum_cycles: u128,
    artifact: &CanicInfrastructureArtifactEntry,
) -> DesiredCanister {
    DesiredCanister {
        canic_init,
        controller_canisters: Vec::new(),
        controllers: vec![operator.to_string()],
        drain: None,
        initial_cycles: config_cycles(initial_cycles),
        init_arg: None,
        init_candid: None,
        kind,
        minimum_cycles: config_cycles(minimum_cycles),
        name: name.to_string(),
        parent: parent.map(str::to_string),
        presence: DesiredPresence::Present,
        principal: principal.map(str::to_string),
        protocol_binding: None,
        replace: false,
        subnet: subnet.to_string(),
        wasm: Some(artifact.wasm_relative_path.clone()),
    }
}

fn config_cycles(cycles: u128) -> String {
    Cycles::new(cycles).to_config_string()
}

fn coordinator_funding(
    profile: FleetFundingProfile,
    source: &CoordinatorFundingSource,
) -> FleetCoordinatorRootFundingPolicy {
    FleetCoordinatorRootFundingPolicy {
        funding_profile: profile,
        minimum_reserve_cycles: source.minimum_reserve_cycles.clone(),
        budget: CyclesFundingBudget {
            window_secs: source.window_secs,
            maximum_cycles: source.maximum_cycles.clone(),
        },
        maximum_automatic_grants: source.maximum_automatic_grants,
        maximum_automatic_cycles: source.maximum_automatic_cycles.clone(),
    }
}

fn root_funding(
    profile: FleetFundingProfile,
    source: &RootFundingSource,
) -> FleetSubnetRootFundingAuthority {
    FleetSubnetRootFundingAuthority {
        root_funding: FleetSubnetRootFundingPolicy {
            funding_profile: profile,
            request_threshold: source.request_threshold.clone(),
            target_balance: source.target_balance.clone(),
            cooldown_secs: source.cooldown_secs,
            budget: CyclesFundingBudget {
                window_secs: source.window_secs,
                maximum_cycles: source.maximum_cycles.clone(),
            },
            maximum_automatic_grants: source.maximum_automatic_grants,
            maximum_automatic_cycles: source.maximum_automatic_cycles.clone(),
        },
        icp_refill: None,
    }
}

fn root_limits(source: &RootSource) -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        maximum_component_instances: source.limits.maximum_component_instances,
        maximum_registry_bytes: source.limits.maximum_registry_bytes,
        maximum_wasm_store_bytes: source.limits.maximum_wasm_store_bytes,
        canister_pool: FleetSubnetCanisterPoolConfig {
            minimum_size: source.canister_pool.minimum_size,
            maximum_size: source.canister_pool.maximum_size,
            canister_cycles: source.canister_pool.canister_cycles.clone(),
        },
        cycles_funding: CyclesFundingBudget {
            window_secs: source.limits.cycles_funding.window_secs,
            maximum_cycles: source.limits.cycles_funding.maximum_cycles.clone(),
        },
        maximum_group_placements: source.limits.maximum_group_placements,
    }
}

fn artifact(
    entries: &[CanicInfrastructureArtifactEntry],
    role: CanicInfrastructureRole,
) -> Result<&CanicInfrastructureArtifactEntry, FleetGenerateError> {
    entries
        .iter()
        .find(|entry| entry.role == role)
        .ok_or_else(|| FleetGenerateError::Release(format!("missing {role:?} artifact")))
}

fn candid_sidecar(
    root: &Path,
    artifact: &CanicInfrastructureArtifactEntry,
) -> Result<String, FleetGenerateError> {
    let wasm = root.join(&artifact.wasm_relative_path);
    let path = wasm.with_extension("did");
    let bytes = match read_optional_regular_bytes(&path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            return Err(FleetGenerateError::Candid(format!(
                "{} is missing",
                path.display()
            )));
        }
        Err(RegularFileReadError::NotRegular) => {
            return Err(FleetGenerateError::Candid(format!(
                "{} is not a regular no-follow file",
                path.display()
            )));
        }
        Err(RegularFileReadError::Io(error)) => {
            return Err(FleetGenerateError::Candid(format!(
                "{}: {error}",
                path.display()
            )));
        }
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(FleetGenerateError::Candid(format!(
                "{} cannot be read without following links on this platform",
                path.display()
            )));
        }
    };
    let actual: [u8; 32] = Sha256::digest(bytes).into();
    if actual != artifact.candid_sha256 {
        return Err(FleetGenerateError::Candid(format!(
            "{} digest differs from release authority",
            path.display()
        )));
    }
    relative_path(root, &path)
}

fn relative_path(root: &Path, path: &Path) -> Result<String, FleetGenerateError> {
    path.strip_prefix(root)
        .map_err(|_| {
            FleetGenerateError::Authority(format!("path is outside workspace: {}", path.display()))
        })?
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| {
            FleetGenerateError::Authority(format!("path is not UTF-8: {}", path.display()))
        })
}

fn canonical_network_id(
    request: &FleetGenerateRequest<'_>,
    _source: &FleetSource,
) -> Result<canic_core::ids::CanonicalNetworkId, FleetGenerateError> {
    resolve_canonical_network_id_from_root(request.root, request.environment)
        .map_err(|error| FleetGenerateError::Authority(error.to_string()))
}

fn load_toml<T: for<'de> Deserialize<'de>>(
    path: &Path,
    _kind: &'static str,
) -> Result<T, FleetGenerateError> {
    let bytes = match read_optional_regular_bytes(path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            return Err(FleetGenerateError::Read {
                path: path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "input is missing"),
            });
        }
        Err(RegularFileReadError::Io(source)) => {
            return Err(FleetGenerateError::Read {
                path: path.to_path_buf(),
                source,
            });
        }
        Err(RegularFileReadError::NotRegular) => {
            return Err(FleetGenerateError::UnsafeInput {
                path: path.to_path_buf(),
                reason: "path is not a regular no-follow file",
            });
        }
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(FleetGenerateError::UnsafeInput {
                path: path.to_path_buf(),
                reason: "no-follow reads are unsupported on this platform",
            });
        }
    };
    if bytes.len() > MAX_GENERATOR_INPUT_BYTES {
        return Err(FleetGenerateError::TooLarge {
            actual: bytes.len(),
            maximum: MAX_GENERATOR_INPUT_BYTES,
            path: path.to_path_buf(),
        });
    }
    toml::from_slice(&bytes).map_err(|source| FleetGenerateError::Decode {
        path: path.to_path_buf(),
        source,
    })
}

const fn require_schema(actual: u32, kind: &'static str) -> Result<(), FleetGenerateError> {
    if actual == 1 {
        Ok(())
    } else {
        Err(FleetGenerateError::Schema { actual, kind })
    }
}

fn require_cycles_creation(
    source: &CyclesCreationSource,
    owner: &str,
) -> Result<(), FleetGenerateError> {
    if source.kind == "cycles" && source.cycles.to_u128() > 0 {
        Ok(())
    } else {
        Err(FleetGenerateError::Authority(format!(
            "{owner} fresh creation must use a positive exact cycles amount"
        )))
    }
}

fn parse_principal(field: &str, value: &str) -> Result<Principal, FleetGenerateError> {
    Principal::from_text(value).map_err(|error| {
        FleetGenerateError::Authority(format!("invalid {field} Principal: {error}"))
    })
}

fn parse_subnet(field: &str, value: &str) -> Result<SubnetId, FleetGenerateError> {
    parse_principal(field, value).map(SubnetId::from_principal)
}

fn mainnet_cycles_ledger() -> String {
    MAINNET_CYCLES_LEDGER.to_string()
}

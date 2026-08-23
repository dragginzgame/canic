//! Module: fleet_install_input
//!
//! Responsibility: load and resolve strict operator placement, admission, limit, and funding input.
//! Does not own: immutable plan publication, Canister creation, installation journals, or runtime.
//! Boundary: IC mainnet selectors and funding are admitted only through trusted Subnet metadata.

#[cfg(test)]
mod tests;

pub use crate::subnet_catalog::{
    SubnetCatalogFailureCacheDispositionV1, SubnetCatalogFailureEffectsV1, SubnetCatalogFieldV1,
    SubnetCatalogLoadFailureEvidenceV1, SubnetCatalogLoadStageV1, SubnetCatalogRefreshTriggerV1,
    SubnetCatalogRegistryRecordEvidenceV1, SubnetCatalogRegistryRecordKindV1,
    SubnetCatalogRegistryValueEncodingV1, SubnetCatalogRetryabilityV1, SubnetCatalogSourceKindV1,
    SubnetCatalogSubjectV1, SubnetCatalogUnknownRetryReasonV1,
};
use crate::{
    component_topology::RootComponentAdmissionInput,
    durable_io::{RegularFileReadError, read_optional_regular_bytes},
    fleet_install_plan::{
        FreshFleetCatalogEvidenceV1, FreshFleetOperatorFundingEvidenceV1,
        PlannedCanisterCreationFunding, PlannedComponentGroupPlacementAssignment,
        PlannedFleetCoordinator, PlannedFleetSubnetRootInput, PlannedSubnetPlacementCostEvidence,
    },
    icp_config::{IcpConfigError, resolve_icp_build_network_from_root},
    subnet_catalog::{load_cached_mainnet_subnet_catalog, load_mainnet_subnet_catalog},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    path::{Path, PathBuf},
    time::{SystemTime, SystemTimeError, UNIX_EPOCH},
};

use candid::Principal;
use canic_core::{
    cdk::types::Cycles,
    ids::{
        BuildNetwork, ComponentGroupDeploymentId, ComponentSpecId, CyclesFundingBudget,
        FleetCoordinatorRootFundingPolicy, FleetFundingProfile, FleetSubnetCanisterPoolConfig,
        FleetSubnetRootAutomaticIcpRefillPolicy, FleetSubnetRootFundingAuthority,
        FleetSubnetRootFundingPolicy, FleetSubnetRootIcpRefillPolicy, FleetSubnetRootLimits,
        SubnetId,
    },
    shared_support::fleet_funding_policy::{
        FleetFundingPolicyValidationError, validate_coordinator_root_funding_policy,
        validate_fleet_root_funding_capacity as validate_fleet_root_funding_capacity_policy,
        validate_fleet_subnet_root_funding_authority,
    },
};
#[cfg(test)]
use ic_query::subnet_catalog::CacheDisposition;
use ic_query::subnet_catalog::{
    CatalogLoadOutcome, SubnetInfo, SubnetKind, SubnetSpecialization, ValidatedSubnetCatalog,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error as ThisError;

const FLEET_INSTALL_INPUT_SCHEMA_VERSION: u32 = 1;
const MAX_FLEET_INSTALL_INPUT_BYTES: usize = 1_024 * 1_024;
const MAX_SUBNET_PROFILE_BYTES: usize = 64;
const STANDARD_SUBNET_NODE_COUNT: u64 = 13;
const TRILLION_CYCLES: u128 = 1_000_000_000_000;
const PROFILE_ROUNDING_CYCLES: u128 = 10 * TRILLION_CYCLES;

///
/// ResolvedFleetInstallInput
///
/// Exact pre-effect input accepted by the immutable Fleet install planner.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedFleetInstallInput {
    pub schema_version: u32,
    pub canonical_sha256: String,
    pub operator: FreshFleetOperatorFundingEvidenceV1,
    pub catalog: FreshFleetCatalogEvidenceV1,
    pub catalog_acquisition: FleetInstallCatalogAcquisitionV1,
    pub funding_profile: FleetFundingProfile,
    pub coordinator: PlannedFleetCoordinator,
    pub fleet_subnet_roots: Vec<PlannedFleetSubnetRootInput>,
}

///
/// FleetInstallCatalogAcquisitionV1
///
/// Transient cache provenance for one Fleet-input resolution, excluded from plan authority.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FleetInstallCatalogAcquisitionV1 {
    NotRequired {
        network: String,
    },
    ValidatedCache {
        cache_path: String,
        cache_disposition: String,
        collected_at: String,
    },
}

///
/// FleetInstallInputError
///
/// Typed rejection while loading or resolving one operator input document.
///

#[derive(Debug, ThisError)]
pub enum FleetInstallInputError {
    #[error("Fleet installation input is missing: {path}")]
    Missing { path: PathBuf },

    #[error("Fleet installation input is not a regular no-follow file: {path}")]
    NotRegular { path: PathBuf },

    #[error("Fleet installation input exceeds the {maximum_bytes}-byte bound: {actual_bytes}")]
    TooLarge {
        maximum_bytes: usize,
        actual_bytes: usize,
    },

    #[error("Fleet installation input has unsupported schema version {actual}; expected 1")]
    UnsupportedSchemaVersion { actual: u32 },

    #[error("could not decode Fleet installation input {path}: {source}")]
    Decode {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("invalid {field} Subnet principal {value:?}: {reason}")]
    InvalidSubnet {
        field: String,
        value: String,
        reason: String,
    },

    #[error("invalid {field} Canister principal {value:?}: {reason}")]
    InvalidCanister {
        field: String,
        value: String,
        reason: String,
    },

    #[error("invalid Fleet Subnet Root Canister pool: {reason}")]
    InvalidCanisterPool { reason: String },

    #[error("imported Canister pool asset {canister} has no trusted IC routing evidence: {reason}")]
    ImportedCanisterRoute { canister: Principal, reason: String },

    #[error(
        "imported Canister pool asset {canister} is routed to Subnet {actual}; expected Fleet Subnet Root placement {expected}"
    )]
    ImportedCanisterSubnetMismatch {
        canister: Principal,
        expected: SubnetId,
        actual: SubnetId,
    },

    #[error("Subnet profile {profile:?} is invalid")]
    InvalidSubnetProfile { profile: String },

    #[error("{selector} requires trusted Subnet metadata for IC mainnet")]
    TrustedMetadataRequired { selector: String },

    #[error("trusted Subnet selector {selector} matched no eligible Subnet")]
    SubnetNotFound { selector: String },

    #[error("trusted Subnet selector {selector} is ambiguous across {matches} eligible Subnets")]
    AmbiguousSubnetSelector { selector: String, matches: usize },

    #[error("Subnet {subnet} is not eligible for Fleet infrastructure: kind is {kind}")]
    IneligibleSubnet { subnet: SubnetId, kind: String },

    #[error(
        "{owner} funding is incompatible with trusted Subnet {subnet} kind {kind}; expected {expected}"
    )]
    FundingMismatch {
        owner: String,
        subnet: SubnetId,
        kind: String,
        expected: &'static str,
    },

    #[error("non-public network funding must use positive cycles for {owner}")]
    NonPublicFunding { owner: String },

    #[error("creation funding amount must be positive for {owner}")]
    NonPositiveCreationFunding { owner: String },

    #[error("Coordinator root-funding policy is required when the Fleet contains roots")]
    MissingCoordinatorRootFundingPolicy,

    #[error("Fleet Subnet Root {placement_subnet} is missing its root-funding policy")]
    MissingRootFundingPolicy { placement_subnet: SubnetId },

    #[error("invalid protected funding policy field {field}: {reason}")]
    InvalidFundingPolicy { field: String, reason: &'static str },

    #[error("funding profile {configured:?} does not match resolved topology {resolved:?}")]
    FundingProfileMismatch {
        configured: FleetFundingProfile,
        resolved: FleetFundingProfile,
    },

    #[error("{owner} on Fiduciary Subnet {subnet} requires acknowledge_fiduciary_cost = true")]
    FiduciaryCostAcknowledgementRequired { owner: String, subnet: SubnetId },

    #[error("{owner} acknowledges Fiduciary cost, but Subnet {subnet} is not Fiduciary")]
    UnexpectedFiduciaryCostAcknowledgement { owner: String, subnet: SubnetId },

    #[error("trusted Subnet {subnet} has no positive node-count evidence")]
    MissingSubnetNodeCount { subnet: SubnetId },

    #[error("{owner} cycles value {actual} is below the topology-profile minimum {minimum}")]
    FundingProfileMinimum {
        owner: String,
        actual: u128,
        minimum: u128,
    },

    #[error("topology-profile arithmetic overflowed while resolving {field}")]
    FundingProfileOverflow { field: &'static str },

    #[error("IC system-Canister override {field} requires allow_ic_system_canister_overrides")]
    UnsafeIcpRefillOverride { field: &'static str },

    #[error("invalid operator principal {value:?}: {reason}")]
    InvalidOperatorPrincipal { value: String, reason: String },

    #[error("operator {field} is invalid")]
    InvalidOperatorEvidence { field: &'static str },

    #[error(
        "operator balance evidence is stale at Unix second {now_unix_secs}; validity ended at {valid_until_unix_secs}"
    )]
    StaleOperatorBalance {
        now_unix_secs: u64,
        valid_until_unix_secs: u64,
    },

    #[error("could not encode canonical Fleet installation input: {0}")]
    CanonicalEncoding(#[from] serde_json::Error),

    #[error("failed to read Fleet installation input {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(transparent)]
    IcpConfig(#[from] IcpConfigError),

    #[error("system clock is before the Unix epoch: {0}")]
    Clock(#[from] SystemTimeError),

    #[error("trusted Subnet catalog resolution failed: {source}")]
    SubnetCatalog {
        #[source]
        source: Box<ic_query::subnet_catalog::SubnetCatalogLoadFailure>,
    },
}

impl FleetInstallInputError {
    /// Borrow the exact detailed catalog failure when this rejection owns one.
    #[must_use]
    pub fn subnet_catalog_failure(
        &self,
    ) -> Option<&ic_query::subnet_catalog::SubnetCatalogLoadFailure> {
        match self {
            Self::SubnetCatalog { source } => Some(source),
            _ => None,
        }
    }
}

impl From<Box<ic_query::subnet_catalog::SubnetCatalogLoadFailure>> for FleetInstallInputError {
    fn from(source: Box<ic_query::subnet_catalog::SubnetCatalogLoadFailure>) -> Self {
        Self::SubnetCatalog { source }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FleetInstallInputDocument {
    schema_version: u32,
    funding_profile: FleetFundingProfile,
    operator: OperatorFundingDocument,
    coordinator: CoordinatorInputDocument,
    fleet_subnet_roots: Vec<FleetSubnetRootInputDocument>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct OperatorFundingDocument {
    principal: String,
    funding_account: String,
    source: String,
    observed_at_unix_secs: u64,
    valid_until_unix_secs: u64,
    balance: CreationFundingDocument,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CoordinatorInputDocument {
    subnet: CoordinatorSubnetSelector,
    creation_funding: CreationFundingDocument,
    #[serde(default)]
    root_funding: Option<CoordinatorRootFundingPolicyDocument>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CoordinatorRootFundingPolicyDocument {
    #[serde(deserialize_with = "Cycles::from_config")]
    minimum_reserve_cycles: Cycles,
    window_secs: u64,
    #[serde(deserialize_with = "Cycles::from_config")]
    maximum_cycles: Cycles,
    maximum_automatic_grants: u32,
    #[serde(deserialize_with = "Cycles::from_config")]
    maximum_automatic_cycles: Cycles,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
enum CoordinatorSubnetSelector {
    Profile {
        profile: String,
        #[serde(default)]
        acknowledge_fiduciary_cost: bool,
    },
    Explicit {
        subnet: String,
        #[serde(default)]
        acknowledge_fiduciary_cost: bool,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
enum CreationFundingDocument {
    Cycles {
        #[serde(deserialize_with = "Cycles::from_config")]
        cycles: Cycles,
    },
    Icp {
        e8s: u64,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FleetSubnetRootInputDocument {
    placement_subnet: String,
    #[serde(default)]
    acknowledge_fiduciary_cost: bool,
    #[serde(default)]
    component_group_placements: BTreeMap<ComponentGroupDeploymentId, Vec<u32>>,
    component_admissions: BTreeMap<ComponentSpecId, u32>,
    limits: FleetSubnetRootLimitsDocument,
    canister_pool: CanisterPoolInputDocument,
    #[serde(default)]
    root_funding: Option<FleetSubnetRootFundingPolicyDocument>,
    #[serde(default)]
    icp_refill: Option<FleetSubnetRootIcpRefillPolicyDocument>,
    root_creation_funding: CreationFundingDocument,
    wasm_store_creation_funding: CreationFundingDocument,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FleetSubnetRootFundingPolicyDocument {
    #[serde(deserialize_with = "Cycles::from_config")]
    request_threshold: Cycles,
    #[serde(deserialize_with = "Cycles::from_config")]
    target_balance: Cycles,
    cooldown_secs: u64,
    window_secs: u64,
    #[serde(deserialize_with = "Cycles::from_config")]
    maximum_cycles: Cycles,
    maximum_automatic_grants: u32,
    #[serde(deserialize_with = "Cycles::from_config")]
    maximum_automatic_cycles: Cycles,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FleetSubnetRootIcpRefillPolicyDocument {
    max_refill_e8s_per_call: u64,
    window_secs: u64,
    maximum_refill_e8s: u64,
    minimum_icp_balance_e8s: u64,
    #[serde(default)]
    min_xdr_permyriad_per_icp: Option<u64>,
    #[serde(default)]
    ledger_canister_id: Option<String>,
    #[serde(default)]
    cmc_canister_id: Option<String>,
    #[serde(default)]
    allow_ic_system_canister_overrides: bool,
    #[serde(default)]
    automatic: Option<FleetSubnetRootAutomaticIcpRefillPolicyDocument>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FleetSubnetRootAutomaticIcpRefillPolicyDocument {
    #[serde(deserialize_with = "Cycles::from_config")]
    emergency_threshold: Cycles,
    #[serde(deserialize_with = "Cycles::from_config")]
    target_balance: Cycles,
    maximum_automatic_refills: u32,
    maximum_automatic_refill_e8s: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CanisterPoolInputDocument {
    minimum_size: u32,
    maximum_size: u32,
    #[serde(deserialize_with = "Cycles::from_config")]
    canister_cycles: Cycles,
    #[serde(default)]
    imports: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FleetSubnetRootLimitsDocument {
    maximum_component_instances: u32,
    maximum_registry_bytes: u64,
    maximum_wasm_store_bytes: u64,
    maximum_group_placements: u32,
    cycles_funding: CyclesFundingBudgetDocument,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CyclesFundingBudgetDocument {
    window_secs: u64,
    #[serde(deserialize_with = "Cycles::from_config")]
    maximum_cycles: Cycles,
}

/// Load and resolve one strict operator input document for the selected network.
pub fn load_and_resolve_fleet_install_input(
    icp_root: &Path,
    environment: &str,
    path: &Path,
) -> Result<ResolvedFleetInstallInput, FleetInstallInputError> {
    let (document, canonical_sha256) = load_document_with_identity(path)?;
    let build_network = resolve_icp_build_network_from_root(icp_root, environment)?;
    let now_unix_secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    if build_network == BuildNetwork::Ic {
        let cached = load_mainnet_subnet_catalog(icp_root, now_unix_secs)?;
        return resolve_document_with_evidence(
            &document,
            &canonical_sha256,
            build_network,
            Some(&cached),
            now_unix_secs,
        );
    }

    resolve_document_with_evidence(
        &document,
        &canonical_sha256,
        build_network,
        None,
        now_unix_secs,
    )
}

/// Load preflight input without a network call or workspace/cache mutation.
pub fn load_and_resolve_fleet_install_input_for_preflight(
    icp_root: &Path,
    environment: &str,
    path: &Path,
) -> Result<ResolvedFleetInstallInput, FleetInstallInputError> {
    let (document, canonical_sha256) = load_document_with_identity(path)?;
    let build_network = resolve_icp_build_network_from_root(icp_root, environment)?;
    let now_unix_secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    if build_network == BuildNetwork::Ic {
        let cached = load_cached_mainnet_subnet_catalog(icp_root, now_unix_secs)?;
        return resolve_document_with_evidence(
            &document,
            &canonical_sha256,
            build_network,
            Some(&cached),
            now_unix_secs,
        );
    }

    resolve_document_with_evidence(
        &document,
        &canonical_sha256,
        build_network,
        None,
        now_unix_secs,
    )
}

fn load_document_with_identity(
    path: &Path,
) -> Result<(FleetInstallInputDocument, String), FleetInstallInputError> {
    let bytes = match read_optional_regular_bytes(path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            return Err(FleetInstallInputError::Missing {
                path: path.to_path_buf(),
            });
        }
        Err(RegularFileReadError::NotRegular) => {
            return Err(FleetInstallInputError::NotRegular {
                path: path.to_path_buf(),
            });
        }
        Err(RegularFileReadError::Io(source)) => {
            return Err(FleetInstallInputError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(FleetInstallInputError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "regular no-follow Fleet input reads are unsupported on this platform",
                ),
            });
        }
    };
    if bytes.len() > MAX_FLEET_INSTALL_INPUT_BYTES {
        return Err(FleetInstallInputError::TooLarge {
            maximum_bytes: MAX_FLEET_INSTALL_INPUT_BYTES,
            actual_bytes: bytes.len(),
        });
    }
    let document = toml::from_slice(&bytes).map_err(|source| FleetInstallInputError::Decode {
        path: path.to_path_buf(),
        source,
    })?;
    validate_schema_version(&document)?;
    let canonical = serde_json::to_vec(&document)?;
    let canonical_sha256 = canic_core::cdk::utils::hash::hex_bytes(Sha256::digest(canonical));
    Ok((document, canonical_sha256))
}

const fn validate_schema_version(
    document: &FleetInstallInputDocument,
) -> Result<(), FleetInstallInputError> {
    if document.schema_version == FLEET_INSTALL_INPUT_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(FleetInstallInputError::UnsupportedSchemaVersion {
            actual: document.schema_version,
        })
    }
}

fn resolve_document_with_evidence(
    document: &FleetInstallInputDocument,
    canonical_sha256: &str,
    build_network: BuildNetwork,
    catalog: Option<&CatalogLoadOutcome>,
    now_unix_secs: u64,
) -> Result<ResolvedFleetInstallInput, FleetInstallInputError> {
    validate_schema_version(document)?;
    let operator = resolve_operator(&document.operator, build_network, now_unix_secs)?;
    let (catalog_evidence, catalog_acquisition) = resolve_catalog_evidence(build_network, catalog)?;
    let validated_catalog = catalog.map(|outcome| &outcome.catalog);
    let (coordinator_subnet, coordinator_acknowledges_fiduciary_cost) = resolve_coordinator_subnet(
        &document.coordinator.subnet,
        build_network,
        validated_catalog,
    )?;
    let coordinator_funding = resolve_funding(
        "Fleet Coordinator",
        coordinator_subnet,
        &document.coordinator.creation_funding,
        build_network,
        validated_catalog,
    )?;
    let coordinator_root_funding = resolve_coordinator_root_funding_policy(
        document.coordinator.root_funding.as_ref(),
        !document.fleet_subnet_roots.is_empty(),
        document.funding_profile,
    )?;
    let coordinator_placement_cost = resolve_placement_cost_evidence(
        "Fleet Coordinator",
        coordinator_subnet,
        coordinator_acknowledges_fiduciary_cost,
        &coordinator_funding,
        coordinator_root_funding
            .as_ref()
            .map_or(0, |policy| policy.maximum_automatic_cycles.to_u128()),
        build_network,
        validated_catalog,
    )?;
    let coordinator = PlannedFleetCoordinator {
        coordinator_subnet,
        placement_cost: coordinator_placement_cost,
        creation_funding: coordinator_funding,
        root_funding: coordinator_root_funding,
    };

    let mut fleet_subnet_roots = Vec::with_capacity(document.fleet_subnet_roots.len());
    let mut imported_canisters = BTreeSet::new();
    for root in &document.fleet_subnet_roots {
        fleet_subnet_roots.push(resolve_root_document(
            root,
            build_network,
            validated_catalog,
            &mut imported_canisters,
            document.funding_profile,
        )?);
    }
    let resolved_profile = resolved_funding_profile(coordinator_subnet, &fleet_subnet_roots);
    if !funding_profile_matches_topology(document.funding_profile, resolved_profile) {
        return Err(FleetInstallInputError::FundingProfileMismatch {
            configured: document.funding_profile,
            resolved: resolved_profile,
        });
    }
    validate_funding_profile_baselines(
        document.funding_profile,
        &coordinator,
        &fleet_subnet_roots,
    )?;
    validate_fleet_root_funding_capacity(&coordinator, &fleet_subnet_roots)?;

    Ok(ResolvedFleetInstallInput {
        schema_version: document.schema_version,
        canonical_sha256: canonical_sha256.to_string(),
        operator,
        catalog: catalog_evidence,
        catalog_acquisition,
        funding_profile: document.funding_profile,
        coordinator,
        fleet_subnet_roots,
    })
}

fn resolve_operator(
    document: &OperatorFundingDocument,
    build_network: BuildNetwork,
    now_unix_secs: u64,
) -> Result<FreshFleetOperatorFundingEvidenceV1, FleetInstallInputError> {
    let principal = Principal::from_text(&document.principal).map_err(|error| {
        FleetInstallInputError::InvalidOperatorPrincipal {
            value: document.principal.clone(),
            reason: error.to_string(),
        }
    })?;
    if principal == Principal::anonymous() || principal.to_text() != document.principal {
        return Err(FleetInstallInputError::InvalidOperatorPrincipal {
            value: document.principal.clone(),
            reason: "principal must be canonical and non-anonymous".to_string(),
        });
    }
    validate_operator_text("funding_account", &document.funding_account)?;
    validate_operator_text("source", &document.source)?;
    if document.observed_at_unix_secs == 0 {
        return Err(FleetInstallInputError::InvalidOperatorEvidence {
            field: "observed_at_unix_secs",
        });
    }
    if document.valid_until_unix_secs <= document.observed_at_unix_secs {
        return Err(FleetInstallInputError::InvalidOperatorEvidence {
            field: "valid_until_unix_secs",
        });
    }
    if now_unix_secs < document.observed_at_unix_secs {
        return Err(FleetInstallInputError::InvalidOperatorEvidence {
            field: "observed_at_unix_secs",
        });
    }
    if now_unix_secs >= document.valid_until_unix_secs {
        return Err(FleetInstallInputError::StaleOperatorBalance {
            now_unix_secs,
            valid_until_unix_secs: document.valid_until_unix_secs,
        });
    }
    let balance = planned_funding("operator balance", &document.balance)?;
    let _ = build_network;

    Ok(FreshFleetOperatorFundingEvidenceV1 {
        principal: document.principal.clone(),
        funding_account: document.funding_account.clone(),
        balance,
        source: document.source.clone(),
        observed_at_unix_secs: document.observed_at_unix_secs,
        valid_until_unix_secs: document.valid_until_unix_secs,
        balance_fresh: true,
    })
}

#[cfg(test)]
fn load_document(path: &Path) -> Result<FleetInstallInputDocument, FleetInstallInputError> {
    load_document_with_identity(path).map(|(document, _)| document)
}

#[cfg(test)]
fn resolve_document(
    document: &FleetInstallInputDocument,
    build_network: BuildNetwork,
    catalog: Option<&ValidatedSubnetCatalog>,
) -> Result<ResolvedFleetInstallInput, FleetInstallInputError> {
    let canonical = serde_json::to_vec(document)?;
    let canonical_sha256 = canic_core::cdk::utils::hash::hex_bytes(Sha256::digest(canonical));
    let outcome = catalog.map(|catalog| CatalogLoadOutcome {
        path: PathBuf::from("test-subnet-catalog.json"),
        catalog: catalog.clone(),
        disposition: CacheDisposition::CacheHit,
    });
    resolve_document_with_evidence(
        document,
        &canonical_sha256,
        build_network,
        outcome.as_ref(),
        document.operator.observed_at_unix_secs,
    )
}

const fn validate_operator_text(
    field: &'static str,
    value: &str,
) -> Result<(), FleetInstallInputError> {
    if value.is_empty() || value.len() > 256 || !value.is_ascii() {
        Err(FleetInstallInputError::InvalidOperatorEvidence { field })
    } else {
        Ok(())
    }
}

fn resolve_catalog_evidence(
    build_network: BuildNetwork,
    outcome: Option<&CatalogLoadOutcome>,
) -> Result<
    (
        FreshFleetCatalogEvidenceV1,
        FleetInstallCatalogAcquisitionV1,
    ),
    FleetInstallInputError,
> {
    if build_network != BuildNetwork::Ic {
        let network = build_network.to_string();
        return Ok((
            FreshFleetCatalogEvidenceV1::NotRequired {
                network: network.clone(),
            },
            FleetInstallCatalogAcquisitionV1::NotRequired { network },
        ));
    }
    let outcome = outcome.ok_or_else(|| FleetInstallInputError::TrustedMetadataRequired {
        selector: "fresh-Fleet catalog evidence".to_string(),
    })?;
    let authority = outcome.snapshot_authority();
    Ok((
        FreshFleetCatalogEvidenceV1::Validated {
            network: outcome.catalog.provenance().network.clone(),
            assurance: authority.assurance.as_str().to_string(),
            source_endpoints: authority.source_endpoints,
            registry_version: authority.registry_version,
            catalog_sha256: authority.catalog_digest,
        },
        FleetInstallCatalogAcquisitionV1::ValidatedCache {
            cache_path: outcome.path.display().to_string(),
            cache_disposition: outcome.disposition.as_str().to_string(),
            collected_at: outcome.catalog.provenance().fetched_at.clone(),
        },
    ))
}

fn resolve_root_document(
    root: &FleetSubnetRootInputDocument,
    build_network: BuildNetwork,
    catalog: Option<&ValidatedSubnetCatalog>,
    imported_canisters: &mut BTreeSet<Principal>,
    funding_profile: FleetFundingProfile,
) -> Result<PlannedFleetSubnetRootInput, FleetInstallInputError> {
    let placement_subnet = parse_subnet(
        "fleet_subnet_roots.placement_subnet",
        &root.placement_subnet,
    )?;
    let root_creation_funding = resolve_funding(
        &format!("Fleet Subnet Root {placement_subnet}"),
        placement_subnet,
        &root.root_creation_funding,
        build_network,
        catalog,
    )?;
    let wasm_store_creation_funding = resolve_funding(
        &format!("Wasm Store for Fleet Subnet Root {placement_subnet}"),
        placement_subnet,
        &root.wasm_store_creation_funding,
        build_network,
        catalog,
    )?;
    let funding =
        resolve_root_funding_authority(root, placement_subnet, build_network, funding_profile)?;
    let placement_cost = resolve_placement_cost_evidence(
        &format!("Fleet Subnet Root {placement_subnet}"),
        placement_subnet,
        root.acknowledge_fiduciary_cost,
        &root_creation_funding,
        funding.root_funding.maximum_automatic_cycles.to_u128(),
        build_network,
        catalog,
    )?;
    let component_admissions = root
        .component_admissions
        .iter()
        .map(
            |(component_spec, maximum_root_instances)| RootComponentAdmissionInput {
                component_spec: component_spec.clone(),
                maximum_root_instances: *maximum_root_instances,
            },
        )
        .collect();
    let canister_pool_imports = root
        .canister_pool
        .imports
        .iter()
        .map(|value| parse_canister("fleet_subnet_roots.canister_pool.imports", value))
        .collect::<Result<Vec<_>, _>>()?;
    validate_canister_pool(root, &canister_pool_imports)?;
    validate_imported_canister_placements(
        placement_subnet,
        &canister_pool_imports,
        build_network,
        catalog,
    )?;
    if let Some(duplicate) = canister_pool_imports
        .iter()
        .find(|canister_id| !imported_canisters.insert(**canister_id))
    {
        return Err(FleetInstallInputError::InvalidCanisterPool {
            reason: format!(
                "imported Canister {duplicate} is assigned to more than one Fleet Subnet Root"
            ),
        });
    }
    Ok(PlannedFleetSubnetRootInput {
        placement_subnet,
        placement_cost,
        component_group_placements: root
            .component_group_placements
            .iter()
            .flat_map(|(deployment, ordinals)| {
                ordinals
                    .iter()
                    .map(|ordinal| PlannedComponentGroupPlacementAssignment {
                        deployment: deployment.clone(),
                        ordinal: *ordinal,
                    })
            })
            .collect(),
        component_admissions,
        limits: FleetSubnetRootLimits {
            maximum_component_instances: root.limits.maximum_component_instances,
            maximum_registry_bytes: root.limits.maximum_registry_bytes,
            maximum_wasm_store_bytes: root.limits.maximum_wasm_store_bytes,
            canister_pool: FleetSubnetCanisterPoolConfig {
                minimum_size: root.canister_pool.minimum_size,
                maximum_size: root.canister_pool.maximum_size,
                canister_cycles: root.canister_pool.canister_cycles.clone(),
            },
            cycles_funding: CyclesFundingBudget {
                window_secs: root.limits.cycles_funding.window_secs,
                maximum_cycles: root.limits.cycles_funding.maximum_cycles.clone(),
            },
            maximum_group_placements: root.limits.maximum_group_placements,
        },
        funding,
        canister_pool_imports,
        root_creation_funding,
        wasm_store_creation_funding,
    })
}

fn resolve_coordinator_root_funding_policy(
    document: Option<&CoordinatorRootFundingPolicyDocument>,
    required: bool,
    funding_profile: FleetFundingProfile,
) -> Result<Option<FleetCoordinatorRootFundingPolicy>, FleetInstallInputError> {
    let Some(document) = document else {
        return if required {
            Err(FleetInstallInputError::MissingCoordinatorRootFundingPolicy)
        } else {
            Ok(None)
        };
    };
    let policy = FleetCoordinatorRootFundingPolicy {
        funding_profile,
        minimum_reserve_cycles: document.minimum_reserve_cycles.clone(),
        budget: CyclesFundingBudget {
            window_secs: document.window_secs,
            maximum_cycles: document.maximum_cycles.clone(),
        },
        maximum_automatic_grants: document.maximum_automatic_grants,
        maximum_automatic_cycles: document.maximum_automatic_cycles.clone(),
    };
    validate_coordinator_root_funding_policy(&policy)
        .map_err(coordinator_policy_validation_error)?;
    Ok(Some(policy))
}

fn resolve_root_funding_authority(
    root: &FleetSubnetRootInputDocument,
    placement_subnet: SubnetId,
    build_network: BuildNetwork,
    funding_profile: FleetFundingProfile,
) -> Result<FleetSubnetRootFundingAuthority, FleetInstallInputError> {
    let document = root
        .root_funding
        .as_ref()
        .ok_or(FleetInstallInputError::MissingRootFundingPolicy { placement_subnet })?;
    let prefix = format!("fleet_subnet_roots[{placement_subnet}]");
    let root_funding = FleetSubnetRootFundingPolicy {
        funding_profile,
        request_threshold: document.request_threshold.clone(),
        target_balance: document.target_balance.clone(),
        cooldown_secs: document.cooldown_secs,
        budget: CyclesFundingBudget {
            window_secs: document.window_secs,
            maximum_cycles: document.maximum_cycles.clone(),
        },
        maximum_automatic_grants: document.maximum_automatic_grants,
        maximum_automatic_cycles: document.maximum_automatic_cycles.clone(),
    };
    let icp_refill = root
        .icp_refill
        .as_ref()
        .map(resolve_root_icp_refill_policy)
        .transpose()?;
    let authority = FleetSubnetRootFundingAuthority {
        root_funding,
        icp_refill,
    };
    validate_fleet_subnet_root_funding_authority(&authority, build_network == BuildNetwork::Ic)
        .map_err(|error| root_policy_validation_error(&prefix, &authority, error))?;
    Ok(authority)
}

fn resolve_root_icp_refill_policy(
    document: &FleetSubnetRootIcpRefillPolicyDocument,
) -> Result<FleetSubnetRootIcpRefillPolicy, FleetInstallInputError> {
    let ledger_canister_id = parse_optional_refill_canister(
        "fleet_subnet_roots.icp_refill.ledger_canister_id",
        document.ledger_canister_id.as_deref(),
    )?;
    let cmc_canister_id = parse_optional_refill_canister(
        "fleet_subnet_roots.icp_refill.cmc_canister_id",
        document.cmc_canister_id.as_deref(),
    )?;
    let automatic =
        document
            .automatic
            .as_ref()
            .map(|automatic| FleetSubnetRootAutomaticIcpRefillPolicy {
                emergency_threshold: automatic.emergency_threshold.clone(),
                target_balance: automatic.target_balance.clone(),
                maximum_automatic_refills: automatic.maximum_automatic_refills,
                maximum_automatic_refill_e8s: automatic.maximum_automatic_refill_e8s,
            });

    Ok(FleetSubnetRootIcpRefillPolicy {
        max_refill_e8s_per_call: document.max_refill_e8s_per_call,
        window_secs: document.window_secs,
        maximum_refill_e8s: document.maximum_refill_e8s,
        minimum_icp_balance_e8s: document.minimum_icp_balance_e8s,
        min_xdr_permyriad_per_icp: document.min_xdr_permyriad_per_icp,
        ledger_canister_id,
        cmc_canister_id,
        allow_ic_system_canister_overrides: document.allow_ic_system_canister_overrides,
        automatic,
    })
}

fn validate_fleet_root_funding_capacity(
    coordinator: &PlannedFleetCoordinator,
    roots: &[PlannedFleetSubnetRootInput],
) -> Result<(), FleetInstallInputError> {
    let Some(policy) = coordinator.root_funding.as_ref() else {
        return if roots.is_empty() {
            Ok(())
        } else {
            Err(FleetInstallInputError::MissingCoordinatorRootFundingPolicy)
        };
    };
    validate_fleet_root_funding_capacity_policy(policy, roots.iter().map(|root| &root.funding))
        .map_err(coordinator_policy_validation_error)
}

fn parse_optional_refill_canister(
    field: &'static str,
    value: Option<&str>,
) -> Result<Option<Principal>, FleetInstallInputError> {
    value.map(|value| parse_canister(field, value)).transpose()
}

fn coordinator_policy_validation_error(
    error: FleetFundingPolicyValidationError,
) -> FleetInstallInputError {
    use FleetFundingPolicyValidationError::{
        CoordinatorMaximumBelowLargestRootTarget, CoordinatorMaximumZero,
        CoordinatorReserveBelowFloor, CoordinatorReserveZero, CoordinatorWindowZero,
    };
    match error {
        CoordinatorReserveZero => invalid_policy(
            "coordinator.root_funding.minimum_reserve_cycles",
            "must be positive",
        ),
        CoordinatorReserveBelowFloor => invalid_policy(
            "coordinator.root_funding.minimum_reserve_cycles",
            "is below the measured Coordinator execution reserve floor",
        ),
        CoordinatorWindowZero => {
            invalid_policy("coordinator.root_funding.window_secs", "must be positive")
        }
        CoordinatorMaximumZero => invalid_policy(
            "coordinator.root_funding.maximum_cycles",
            "must be positive",
        ),
        CoordinatorMaximumBelowLargestRootTarget => invalid_policy(
            "coordinator.root_funding.maximum_cycles",
            "must admit the largest root's legitimate zero-balance grant",
        ),
        _ => invalid_policy(
            "coordinator.root_funding",
            "contains invalid protected policy authority",
        ),
    }
}

fn root_policy_validation_error(
    prefix: &str,
    authority: &FleetSubnetRootFundingAuthority,
    error: FleetFundingPolicyValidationError,
) -> FleetInstallInputError {
    use FleetFundingPolicyValidationError::{
        AutomaticEmergencyNotBelowRequestThreshold, AutomaticEmergencyThresholdBelowFloor,
        AutomaticEmergencyThresholdZero, AutomaticTargetAboveRootTargetBalance,
        AutomaticTargetBalanceZero, AutomaticTargetNotAboveRequestThreshold,
        IcpCmcPrincipalReserved, IcpLedgerPrincipalReserved, IcpMaximumBelowPerCallMaximum,
        IcpMaximumZero, IcpMinimumBalanceZero, IcpMinimumRateZero, IcpOverrideUnsafe,
        IcpPerCallMaximumZero, IcpWindowZero, RootCooldownZero, RootMaximumBelowTargetBalance,
        RootMaximumZero, RootRequestThresholdBelowFloor, RootRequestThresholdZero,
        RootTargetBalanceZero, RootTargetNotAboveRequestThreshold, RootWindowZero,
    };
    let (field, reason) = match error {
        RootRequestThresholdZero => ("root_funding.request_threshold", "must be positive"),
        RootRequestThresholdBelowFloor => (
            "root_funding.request_threshold",
            "is below the measured Root request and recovery floor",
        ),
        RootTargetBalanceZero => ("root_funding.target_balance", "must be positive"),
        RootTargetNotAboveRequestThreshold => (
            "root_funding.target_balance",
            "must be greater than request_threshold",
        ),
        RootCooldownZero => ("root_funding.cooldown_secs", "must be positive"),
        RootWindowZero => ("root_funding.window_secs", "must be positive"),
        RootMaximumZero => ("root_funding.maximum_cycles", "must be positive"),
        RootMaximumBelowTargetBalance => (
            "root_funding.maximum_cycles",
            "must admit the largest legitimate zero-balance grant",
        ),
        IcpPerCallMaximumZero => ("icp_refill.max_refill_e8s_per_call", "must be positive"),
        IcpWindowZero => ("icp_refill.window_secs", "must be positive"),
        IcpMaximumZero => ("icp_refill.maximum_refill_e8s", "must be positive"),
        IcpMaximumBelowPerCallMaximum => (
            "icp_refill.maximum_refill_e8s",
            "must be at least max_refill_e8s_per_call",
        ),
        IcpMinimumBalanceZero => ("icp_refill.minimum_icp_balance_e8s", "must be positive"),
        IcpMinimumRateZero => (
            "icp_refill.min_xdr_permyriad_per_icp",
            "must be positive when present",
        ),
        IcpLedgerPrincipalReserved => (
            "icp_refill.ledger_canister_id",
            "must name a non-reserved Canister principal",
        ),
        IcpCmcPrincipalReserved => (
            "icp_refill.cmc_canister_id",
            "must name a non-reserved Canister principal",
        ),
        IcpOverrideUnsafe => {
            return FleetInstallInputError::UnsafeIcpRefillOverride {
                field: if authority
                    .icp_refill
                    .as_ref()
                    .is_some_and(|policy| policy.ledger_canister_id.is_some())
                {
                    "fleet_subnet_roots.icp_refill.ledger_canister_id"
                } else {
                    "fleet_subnet_roots.icp_refill.cmc_canister_id"
                },
            };
        }
        AutomaticEmergencyThresholdZero => (
            "icp_refill.automatic.emergency_threshold",
            "must be positive",
        ),
        AutomaticEmergencyThresholdBelowFloor => (
            "icp_refill.automatic.emergency_threshold",
            "is below the measured automatic-refill execution and recovery floor",
        ),
        AutomaticEmergencyNotBelowRequestThreshold => (
            "icp_refill.automatic.emergency_threshold",
            "must be less than root_funding.request_threshold",
        ),
        AutomaticTargetBalanceZero => ("icp_refill.automatic.target_balance", "must be positive"),
        AutomaticTargetNotAboveRequestThreshold => (
            "icp_refill.automatic.target_balance",
            "must be greater than root_funding.request_threshold",
        ),
        AutomaticTargetAboveRootTargetBalance => (
            "icp_refill.automatic.target_balance",
            "must not exceed root_funding.target_balance",
        ),
        _ => (
            "root_funding",
            "contains invalid protected policy authority",
        ),
    };
    invalid_policy(format!("{prefix}.{field}"), reason)
}

fn invalid_policy(field: impl Into<String>, reason: &'static str) -> FleetInstallInputError {
    FleetInstallInputError::InvalidFundingPolicy {
        field: field.into(),
        reason,
    }
}

fn resolve_coordinator_subnet(
    selector: &CoordinatorSubnetSelector,
    build_network: BuildNetwork,
    catalog: Option<&ValidatedSubnetCatalog>,
) -> Result<(SubnetId, bool), FleetInstallInputError> {
    match selector {
        CoordinatorSubnetSelector::Explicit {
            subnet,
            acknowledge_fiduciary_cost,
        } => {
            let subnet = parse_subnet("coordinator.subnet", subnet)?;
            if build_network == BuildNetwork::Ic {
                let info = trusted_subnet(catalog, subnet)?;
                validate_eligible_subnet(info)?;
            }
            Ok((subnet, *acknowledge_fiduciary_cost))
        }
        CoordinatorSubnetSelector::Profile {
            profile,
            acknowledge_fiduciary_cost,
        } => {
            validate_profile(profile)?;
            require_public_catalog(build_network, catalog, &format!("profile {profile:?}"))?;
            let subnet = select_unique_subnet(
                catalog.expect("public catalog required"),
                &format!("profile {profile:?}"),
                |info| info.subnet_kind == SubnetKind::Application && info.subnet_label == *profile,
            )?;
            Ok((subnet, *acknowledge_fiduciary_cost))
        }
    }
}

fn resolve_placement_cost_evidence(
    owner: &str,
    subnet: SubnetId,
    acknowledge_fiduciary_cost: bool,
    creation_funding: &PlannedCanisterCreationFunding,
    maximum_automatic_cycles: u128,
    build_network: BuildNetwork,
    catalog: Option<&ValidatedSubnetCatalog>,
) -> Result<PlannedSubnetPlacementCostEvidence, FleetInstallInputError> {
    if build_network != BuildNetwork::Ic {
        if acknowledge_fiduciary_cost {
            return Err(
                FleetInstallInputError::UnexpectedFiduciaryCostAcknowledgement {
                    owner: owner.to_string(),
                    subnet,
                },
            );
        }
        return Ok(PlannedSubnetPlacementCostEvidence {
            subnet,
            catalog_sha256: None,
            subnet_specialization: "not_required".to_string(),
            node_count: STANDARD_SUBNET_NODE_COUNT,
            cost_multiplier_numerator: 1,
            cost_multiplier_denominator: 1,
            acknowledge_fiduciary_cost: false,
            warning: None,
        });
    }

    let catalog = catalog.ok_or_else(|| FleetInstallInputError::TrustedMetadataRequired {
        selector: format!("placement cost for {owner}"),
    })?;
    let info = trusted_subnet(Some(catalog), subnet)?;
    validate_eligible_subnet(info)?;
    let node_count = u64::from(
        info.node_count
            .filter(|count| *count > 0)
            .ok_or(FleetInstallInputError::MissingSubnetNodeCount { subnet })?,
    );
    let fiduciary = info.subnet_specialization == SubnetSpecialization::Fiduciary;
    if fiduciary && !acknowledge_fiduciary_cost {
        return Err(
            FleetInstallInputError::FiduciaryCostAcknowledgementRequired {
                owner: owner.to_string(),
                subnet,
            },
        );
    }
    if !fiduciary && acknowledge_fiduciary_cost {
        return Err(
            FleetInstallInputError::UnexpectedFiduciaryCostAcknowledgement {
                owner: owner.to_string(),
                subnet,
            },
        );
    }
    let multiplier_numerator = node_count.max(STANDARD_SUBNET_NODE_COUNT);
    let multiplier_denominator = STANDARD_SUBNET_NODE_COUNT;
    let warning = fiduciary.then(|| {
        format!(
            "WARNING: {owner} uses Fiduciary Subnet {subnet}; node_count={node_count}; cost_multiplier={multiplier_numerator}/{multiplier_denominator}; creation_funding={}; maximum_automatic_exposure={maximum_automatic_cycles} cycles",
            display_creation_funding(creation_funding),
        )
    });
    Ok(PlannedSubnetPlacementCostEvidence {
        subnet,
        catalog_sha256: Some(canic_core::cdk::utils::hash::hex_bytes(
            catalog.catalog_digest(),
        )),
        subnet_specialization: info.subnet_specialization.as_str().to_string(),
        node_count,
        cost_multiplier_numerator: multiplier_numerator,
        cost_multiplier_denominator: multiplier_denominator,
        acknowledge_fiduciary_cost,
        warning,
    })
}

fn display_creation_funding(funding: &PlannedCanisterCreationFunding) -> String {
    match funding {
        PlannedCanisterCreationFunding::Cycles { cycles } => format!("{cycles} cycles"),
        PlannedCanisterCreationFunding::Icp { e8s } => format!("{e8s} e8s"),
    }
}

fn resolved_funding_profile(
    coordinator_subnet: SubnetId,
    roots: &[PlannedFleetSubnetRootInput],
) -> FleetFundingProfile {
    if roots
        .iter()
        .all(|root| root.placement_subnet == coordinator_subnet)
    {
        FleetFundingProfile::SingleSubnet
    } else {
        FleetFundingProfile::MultiSubnet
    }
}

const fn funding_profile_matches_topology(
    configured: FleetFundingProfile,
    resolved: FleetFundingProfile,
) -> bool {
    matches!(
        (configured, resolved),
        (
            FleetFundingProfile::SingleSubnet,
            FleetFundingProfile::SingleSubnet
        ) | (
            FleetFundingProfile::PreviewMultiSubnet | FleetFundingProfile::MultiSubnet,
            FleetFundingProfile::MultiSubnet
        )
    )
}

fn validate_funding_profile_baselines(
    profile: FleetFundingProfile,
    coordinator: &PlannedFleetCoordinator,
    roots: &[PlannedFleetSubnetRootInput],
) -> Result<(), FleetInstallInputError> {
    let coordinator_nodes = coordinator.placement_cost.node_count;
    let coordinator_reserve_base = match profile {
        FleetFundingProfile::SingleSubnet => 30 * TRILLION_CYCLES,
        FleetFundingProfile::PreviewMultiSubnet => 80 * TRILLION_CYCLES,
        FleetFundingProfile::MultiSubnet => 2_000 * TRILLION_CYCLES,
    };
    let coordinator_reserve = scale_profile_cycles(
        coordinator_reserve_base,
        coordinator_nodes,
        "coordinator minimum reserve",
    )?;
    let root_target_base = match profile {
        FleetFundingProfile::SingleSubnet | FleetFundingProfile::PreviewMultiSubnet => {
            30 * TRILLION_CYCLES
        }
        FleetFundingProfile::MultiSubnet => 1_000 * TRILLION_CYCLES,
    };
    let root_targets = roots
        .iter()
        .map(|root| {
            scale_profile_cycles(
                root_target_base,
                root.placement_cost.node_count,
                "root target balance",
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let fleet_window_minimum = match profile {
        FleetFundingProfile::SingleSubnet => root_targets.first().copied().unwrap_or(0),
        FleetFundingProfile::PreviewMultiSubnet | FleetFundingProfile::MultiSubnet => {
            checked_sum(&root_targets, "Fleet window maximum")?
        }
    };
    let coordinator_creation_minimum = match profile {
        FleetFundingProfile::SingleSubnet => scale_profile_cycles(
            100 * TRILLION_CYCLES,
            coordinator_nodes,
            "Coordinator creation funding",
        )?,
        FleetFundingProfile::PreviewMultiSubnet => coordinator_reserve
            .checked_add(
                coordinator
                    .root_funding
                    .as_ref()
                    .map_or(0, |policy| policy.maximum_automatic_cycles.to_u128()),
            )
            .ok_or(FleetInstallInputError::FundingProfileOverflow {
                field: "Coordinator creation funding",
            })?,
        FleetFundingProfile::MultiSubnet => coordinator_reserve
            .checked_add(
                checked_sum(&root_targets, "Coordinator creation funding")?
                    .checked_mul(2)
                    .ok_or(FleetInstallInputError::FundingProfileOverflow {
                        field: "Coordinator creation funding",
                    })?,
            )
            .ok_or(FleetInstallInputError::FundingProfileOverflow {
                field: "Coordinator creation funding",
            })?,
    };
    validate_creation_funding_minimum(
        "Fleet Coordinator creation funding",
        &coordinator.creation_funding,
        coordinator_creation_minimum,
    )?;

    if let Some(policy) = coordinator.root_funding.as_ref() {
        validate_cycles_minimum(
            "Coordinator minimum reserve",
            policy.minimum_reserve_cycles.to_u128(),
            coordinator_reserve,
        )?;
        validate_cycles_minimum(
            "Fleet window maximum",
            policy.budget.maximum_cycles.to_u128(),
            fleet_window_minimum,
        )?;
    }

    for (root, target_minimum) in roots.iter().zip(root_targets) {
        validate_root_funding_profile_baseline(profile, root, target_minimum)?;
    }
    Ok(())
}

fn validate_root_funding_profile_baseline(
    profile: FleetFundingProfile,
    root: &PlannedFleetSubnetRootInput,
    target_minimum: u128,
) -> Result<(), FleetInstallInputError> {
    let owner = format!("Fleet Subnet Root {}", root.placement_subnet);
    let nodes = root.placement_cost.node_count;
    let (threshold_base, root_creation_base, store_creation_base) = match profile {
        FleetFundingProfile::SingleSubnet | FleetFundingProfile::PreviewMultiSubnet => (
            10 * TRILLION_CYCLES,
            30 * TRILLION_CYCLES,
            10 * TRILLION_CYCLES,
        ),
        FleetFundingProfile::MultiSubnet => (
            250 * TRILLION_CYCLES,
            1_000 * TRILLION_CYCLES,
            200 * TRILLION_CYCLES,
        ),
    };
    let threshold_minimum = scale_profile_cycles(threshold_base, nodes, "root request threshold")?;
    validate_creation_funding_minimum(
        &format!("{owner} creation funding"),
        &root.root_creation_funding,
        scale_profile_cycles(root_creation_base, nodes, "Root creation funding")?,
    )?;
    validate_creation_funding_minimum(
        &format!("Wasm Store for {owner} creation funding"),
        &root.wasm_store_creation_funding,
        scale_profile_cycles(store_creation_base, nodes, "Wasm Store creation funding")?,
    )?;
    validate_cycles_minimum(
        &format!("{owner} request threshold"),
        root.funding.root_funding.request_threshold.to_u128(),
        threshold_minimum,
    )?;
    validate_cycles_minimum(
        &format!("{owner} target balance"),
        root.funding.root_funding.target_balance.to_u128(),
        target_minimum,
    )?;
    let actual_gap = root
        .funding
        .root_funding
        .target_balance
        .to_u128()
        .checked_sub(root.funding.root_funding.request_threshold.to_u128())
        .ok_or(FleetInstallInputError::FundingProfileOverflow {
            field: "Root target/threshold gap",
        })?;
    validate_cycles_minimum(
        &format!("{owner} target/threshold gap"),
        actual_gap,
        target_minimum - threshold_minimum,
    )?;
    validate_cycles_minimum(
        &format!("{owner} window maximum"),
        root.funding.root_funding.budget.maximum_cycles.to_u128(),
        target_minimum,
    )
}

fn scale_profile_cycles(
    standard_cycles: u128,
    node_count: u64,
    field: &'static str,
) -> Result<u128, FleetInstallInputError> {
    let numerator = u128::from(node_count.max(STANDARD_SUBNET_NODE_COUNT));
    let scaled_numerator = standard_cycles
        .checked_mul(numerator)
        .ok_or(FleetInstallInputError::FundingProfileOverflow { field })?;
    let denominator = u128::from(STANDARD_SUBNET_NODE_COUNT);
    let scaled = scaled_numerator / denominator + u128::from(scaled_numerator % denominator != 0);
    let rounded =
        scaled / PROFILE_ROUNDING_CYCLES + u128::from(scaled % PROFILE_ROUNDING_CYCLES != 0);
    rounded
        .checked_mul(PROFILE_ROUNDING_CYCLES)
        .ok_or(FleetInstallInputError::FundingProfileOverflow { field })
}

fn checked_sum(values: &[u128], field: &'static str) -> Result<u128, FleetInstallInputError> {
    values.iter().try_fold(0_u128, |total, value| {
        total
            .checked_add(*value)
            .ok_or(FleetInstallInputError::FundingProfileOverflow { field })
    })
}

fn validate_creation_funding_minimum(
    owner: &str,
    funding: &PlannedCanisterCreationFunding,
    minimum: u128,
) -> Result<(), FleetInstallInputError> {
    if let PlannedCanisterCreationFunding::Cycles { cycles } = funding {
        validate_cycles_minimum(owner, *cycles, minimum)?;
    }
    Ok(())
}

fn validate_cycles_minimum(
    owner: &str,
    actual: u128,
    minimum: u128,
) -> Result<(), FleetInstallInputError> {
    if actual < minimum {
        Err(FleetInstallInputError::FundingProfileMinimum {
            owner: owner.to_string(),
            actual,
            minimum,
        })
    } else {
        Ok(())
    }
}

fn resolve_funding(
    owner: &str,
    subnet: SubnetId,
    funding: &CreationFundingDocument,
    build_network: BuildNetwork,
    catalog: Option<&ValidatedSubnetCatalog>,
) -> Result<PlannedCanisterCreationFunding, FleetInstallInputError> {
    let planned = planned_funding(owner, funding)?;
    if build_network != BuildNetwork::Ic {
        return match planned {
            PlannedCanisterCreationFunding::Cycles { .. } => Ok(planned),
            PlannedCanisterCreationFunding::Icp { .. } => {
                Err(FleetInstallInputError::NonPublicFunding {
                    owner: owner.to_string(),
                })
            }
        };
    }

    let info = trusted_subnet(catalog, subnet)?;
    validate_eligible_subnet(info)?;
    let matches = matches!(
        (&planned, info.subnet_kind),
        (
            PlannedCanisterCreationFunding::Cycles { .. },
            SubnetKind::Application
        ) | (
            PlannedCanisterCreationFunding::Icp { .. },
            SubnetKind::System
        )
    );
    if matches {
        return Ok(planned);
    }
    Err(FleetInstallInputError::FundingMismatch {
        owner: owner.to_string(),
        subnet,
        kind: info.subnet_kind.as_str().to_string(),
        expected: match info.subnet_kind {
            SubnetKind::Application => "cycles",
            SubnetKind::System => "icp",
            SubnetKind::CloudEngine | SubnetKind::Unknown => {
                unreachable!("ineligible Subnets reject before funding validation")
            }
        },
    })
}

fn planned_funding(
    owner: &str,
    funding: &CreationFundingDocument,
) -> Result<PlannedCanisterCreationFunding, FleetInstallInputError> {
    match funding {
        CreationFundingDocument::Cycles { cycles } if cycles.to_u128() > 0 => {
            Ok(PlannedCanisterCreationFunding::Cycles {
                cycles: cycles.to_u128(),
            })
        }
        CreationFundingDocument::Icp { e8s } if *e8s > 0 => {
            Ok(PlannedCanisterCreationFunding::Icp { e8s: *e8s })
        }
        CreationFundingDocument::Cycles { .. } | CreationFundingDocument::Icp { .. } => {
            Err(FleetInstallInputError::NonPositiveCreationFunding {
                owner: owner.to_string(),
            })
        }
    }
}

fn parse_subnet(field: &str, value: &str) -> Result<SubnetId, FleetInstallInputError> {
    let principal =
        Principal::from_text(value).map_err(|error| FleetInstallInputError::InvalidSubnet {
            field: field.to_string(),
            value: value.to_string(),
            reason: error.to_string(),
        })?;
    if principal == Principal::anonymous() || principal == Principal::management_canister() {
        return Err(FleetInstallInputError::InvalidSubnet {
            field: field.to_string(),
            value: value.to_string(),
            reason: "anonymous principal is not a physical Subnet".to_string(),
        });
    }
    Ok(SubnetId::from_principal(principal))
}

fn parse_canister(field: &str, value: &str) -> Result<Principal, FleetInstallInputError> {
    let principal =
        Principal::from_text(value).map_err(|error| FleetInstallInputError::InvalidCanister {
            field: field.to_string(),
            value: value.to_string(),
            reason: error.to_string(),
        })?;
    if principal == Principal::anonymous() || principal == Principal::management_canister() {
        return Err(FleetInstallInputError::InvalidCanister {
            field: field.to_string(),
            value: value.to_string(),
            reason: "reserved principal is not a Canister".to_string(),
        });
    }
    Ok(principal)
}

fn validate_canister_pool(
    root: &FleetSubnetRootInputDocument,
    imports: &[Principal],
) -> Result<(), FleetInstallInputError> {
    let pool = &root.canister_pool;
    if pool.minimum_size == 0 {
        return Err(FleetInstallInputError::InvalidCanisterPool {
            reason: "minimum_size must be greater than zero for every Fleet Subnet Root"
                .to_string(),
        });
    }
    if pool.maximum_size < pool.minimum_size {
        return Err(FleetInstallInputError::InvalidCanisterPool {
            reason: format!(
                "maximum_size {} is smaller than minimum_size {}",
                pool.maximum_size, pool.minimum_size
            ),
        });
    }
    if pool.canister_cycles.to_u128() == 0 {
        return Err(FleetInstallInputError::InvalidCanisterPool {
            reason: "canister_cycles must be greater than zero".to_string(),
        });
    }
    if imports.len() > pool.maximum_size as usize {
        return Err(FleetInstallInputError::InvalidCanisterPool {
            reason: format!(
                "{} imported Canisters exceed maximum_size {}",
                imports.len(),
                pool.maximum_size
            ),
        });
    }
    let unique = imports.iter().collect::<std::collections::BTreeSet<_>>();
    if unique.len() != imports.len() {
        return Err(FleetInstallInputError::InvalidCanisterPool {
            reason: "imported Canister principals must be unique within one root".to_string(),
        });
    }
    Ok(())
}

fn validate_imported_canister_placements(
    expected_subnet: SubnetId,
    imports: &[Principal],
    build_network: BuildNetwork,
    catalog: Option<&ValidatedSubnetCatalog>,
) -> Result<(), FleetInstallInputError> {
    if build_network != BuildNetwork::Ic || imports.is_empty() {
        return Ok(());
    }
    let catalog = catalog.ok_or_else(|| FleetInstallInputError::TrustedMetadataRequired {
        selector: format!("Canister pool imports for Subnet {expected_subnet}"),
    })?;
    for canister in imports {
        let resolved = catalog
            .resolve_canister_route(&canister.to_text())
            .map_err(|error| FleetInstallInputError::ImportedCanisterRoute {
                canister: *canister,
                reason: error.to_string(),
            })?;
        let actual = SubnetId::from_principal(resolved.subnet);
        if actual != expected_subnet {
            return Err(FleetInstallInputError::ImportedCanisterSubnetMismatch {
                canister: *canister,
                expected: expected_subnet,
                actual,
            });
        }
    }
    Ok(())
}

fn validate_profile(profile: &str) -> Result<(), FleetInstallInputError> {
    if !profile.is_empty()
        && profile.len() <= MAX_SUBNET_PROFILE_BYTES
        && profile.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
    {
        Ok(())
    } else {
        Err(FleetInstallInputError::InvalidSubnetProfile {
            profile: profile.to_string(),
        })
    }
}

fn require_public_catalog(
    build_network: BuildNetwork,
    catalog: Option<&ValidatedSubnetCatalog>,
    selector: &str,
) -> Result<(), FleetInstallInputError> {
    if build_network == BuildNetwork::Ic && catalog.is_some() {
        Ok(())
    } else {
        Err(FleetInstallInputError::TrustedMetadataRequired {
            selector: selector.to_string(),
        })
    }
}

fn trusted_subnet(
    catalog: Option<&ValidatedSubnetCatalog>,
    subnet: SubnetId,
) -> Result<&SubnetInfo, FleetInstallInputError> {
    let catalog = catalog.ok_or_else(|| FleetInstallInputError::TrustedMetadataRequired {
        selector: format!("explicit Subnet {subnet}"),
    })?;
    catalog
        .subnets()
        .iter()
        .find(|info| info.subnet_principal == subnet.to_string())
        .ok_or_else(|| FleetInstallInputError::SubnetNotFound {
            selector: format!("explicit Subnet {subnet}"),
        })
}

fn validate_eligible_subnet(info: &SubnetInfo) -> Result<(), FleetInstallInputError> {
    if matches!(
        info.subnet_kind,
        SubnetKind::Application | SubnetKind::System
    ) {
        return Ok(());
    }
    let subnet = parse_subnet("trusted subnet catalog", &info.subnet_principal)?;
    Err(FleetInstallInputError::IneligibleSubnet {
        subnet,
        kind: info.subnet_kind.as_str().to_string(),
    })
}

fn select_unique_subnet(
    catalog: &ValidatedSubnetCatalog,
    selector: &str,
    matches: impl Fn(&SubnetInfo) -> bool,
) -> Result<SubnetId, FleetInstallInputError> {
    let candidates = catalog
        .subnets()
        .iter()
        .filter(|info| matches(info))
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [info] => parse_subnet("trusted subnet catalog", &info.subnet_principal),
        [] => Err(FleetInstallInputError::SubnetNotFound {
            selector: selector.to_string(),
        }),
        _ => Err(FleetInstallInputError::AmbiguousSubnetSelector {
            selector: selector.to_string(),
            matches: candidates.len(),
        }),
    }
}

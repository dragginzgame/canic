//! Module: fleet_install_input
//!
//! Responsibility: load and resolve strict operator placement, admission, limit, and funding input.
//! Does not own: immutable plan publication, Canister creation, installation journals, or runtime.
//! Boundary: IC mainnet selectors and funding are admitted only through trusted Subnet metadata.

#[cfg(test)]
mod tests;

use crate::{
    component_topology::RootComponentAdmissionInput,
    durable_io::{RegularFileReadError, read_optional_regular_bytes},
    fleet_install_plan::{
        PlannedCanisterCreationFunding, PlannedFleetCoordinator, PlannedFleetSubnetRootInput,
    },
    icp_config::{IcpConfigError, resolve_icp_build_network_from_root},
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
        BuildNetwork, ComponentSpecId, CyclesFundingBudget, FleetSubnetCanisterPoolConfig,
        FleetSubnetRootLimits, SubnetId,
    },
};
use ic_query::subnet_catalog::{
    DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT, SubnetCatalog, SubnetCatalogCacheRequest, SubnetInfo,
    SubnetKind, SubnetSpecialization, load_or_refresh_subnet_catalog,
};
use serde::Deserialize;
use thiserror::Error as ThisError;

const FLEET_INSTALL_INPUT_SCHEMA_VERSION: u32 = 1;
const MAX_FLEET_INSTALL_INPUT_BYTES: usize = 1_024 * 1_024;
const MAX_SUBNET_PROFILE_BYTES: usize = 64;

///
/// ResolvedFleetInstallInput
///
/// Exact pre-effect input accepted by the immutable Fleet install planner.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedFleetInstallInput {
    pub coordinator: PlannedFleetCoordinator,
    pub fleet_subnet_roots: Vec<PlannedFleetSubnetRootInput>,
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

    #[error("trusted Subnet catalog resolution failed: {0}")]
    SubnetCatalog(#[from] ic_query::subnet_catalog::SubnetCatalogHostError),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct FleetInstallInputDocument {
    schema_version: u32,
    coordinator: CoordinatorInputDocument,
    fleet_subnet_roots: Vec<FleetSubnetRootInputDocument>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct CoordinatorInputDocument {
    subnet: CoordinatorSubnetSelector,
    creation_funding: CreationFundingDocument,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "kind")]
enum CoordinatorSubnetSelector {
    Recommended,
    Profile { profile: String },
    Explicit { subnet: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct FleetSubnetRootInputDocument {
    placement_subnet: String,
    component_admissions: BTreeMap<ComponentSpecId, u32>,
    limits: FleetSubnetRootLimitsDocument,
    canister_pool: CanisterPoolInputDocument,
    creation_funding: CreationFundingDocument,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct CanisterPoolInputDocument {
    minimum_size: u32,
    maximum_size: u32,
    #[serde(deserialize_with = "Cycles::from_config")]
    canister_cycles: Cycles,
    #[serde(default)]
    imports: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct FleetSubnetRootLimitsDocument {
    maximum_component_instances: u32,
    maximum_managed_canisters: u32,
    maximum_registry_bytes: u64,
    maximum_wasm_store_bytes: u64,
    cycles_funding: CyclesFundingBudgetDocument,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
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
    let document = load_document(path)?;
    let build_network = resolve_icp_build_network_from_root(icp_root, environment)?;
    if build_network == BuildNetwork::Ic {
        let now_unix_secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let cached = load_or_refresh_subnet_catalog(
            &SubnetCatalogCacheRequest::new(icp_root, "ic"),
            DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT,
            now_unix_secs,
        )?;
        return resolve_document(&document, build_network, Some(&cached.catalog));
    }

    resolve_document(&document, build_network, None)
}

fn load_document(path: &Path) -> Result<FleetInstallInputDocument, FleetInstallInputError> {
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
    Ok(document)
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

fn resolve_document(
    document: &FleetInstallInputDocument,
    build_network: BuildNetwork,
    catalog: Option<&SubnetCatalog>,
) -> Result<ResolvedFleetInstallInput, FleetInstallInputError> {
    validate_schema_version(document)?;
    let coordinator_subnet =
        resolve_coordinator_subnet(&document.coordinator.subnet, build_network, catalog)?;
    let coordinator_funding = resolve_funding(
        "Fleet Coordinator",
        coordinator_subnet,
        &document.coordinator.creation_funding,
        build_network,
        catalog,
    )?;
    let coordinator = PlannedFleetCoordinator {
        coordinator_subnet,
        creation_funding: coordinator_funding,
    };

    let mut fleet_subnet_roots = Vec::with_capacity(document.fleet_subnet_roots.len());
    let mut imported_canisters = BTreeSet::new();
    for root in &document.fleet_subnet_roots {
        let placement_subnet = parse_subnet(
            "fleet_subnet_roots.placement_subnet",
            &root.placement_subnet,
        )?;
        let owner = format!("Fleet Subnet Root {placement_subnet}");
        let creation_funding = resolve_funding(
            &owner,
            placement_subnet,
            &root.creation_funding,
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
        fleet_subnet_roots.push(PlannedFleetSubnetRootInput {
            placement_subnet,
            component_admissions,
            limits: FleetSubnetRootLimits {
                maximum_component_instances: root.limits.maximum_component_instances,
                maximum_managed_canisters: root.limits.maximum_managed_canisters,
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
            },
            canister_pool_imports,
            creation_funding,
        });
    }

    Ok(ResolvedFleetInstallInput {
        coordinator,
        fleet_subnet_roots,
    })
}

fn resolve_coordinator_subnet(
    selector: &CoordinatorSubnetSelector,
    build_network: BuildNetwork,
    catalog: Option<&SubnetCatalog>,
) -> Result<SubnetId, FleetInstallInputError> {
    match selector {
        CoordinatorSubnetSelector::Explicit { subnet } => {
            let subnet = parse_subnet("coordinator.subnet", subnet)?;
            if build_network == BuildNetwork::Ic {
                let info = trusted_subnet(catalog, subnet)?;
                validate_eligible_subnet(info)?;
            }
            Ok(subnet)
        }
        CoordinatorSubnetSelector::Recommended => {
            require_public_catalog(build_network, catalog, "recommended")?;
            select_unique_subnet(
                catalog.expect("public catalog required"),
                "recommended",
                |info| {
                    info.subnet_kind == SubnetKind::Application
                        && info.subnet_specialization == SubnetSpecialization::Fiduciary
                },
            )
        }
        CoordinatorSubnetSelector::Profile { profile } => {
            validate_profile(profile)?;
            require_public_catalog(build_network, catalog, &format!("profile {profile:?}"))?;
            select_unique_subnet(
                catalog.expect("public catalog required"),
                &format!("profile {profile:?}"),
                |info| info.subnet_kind == SubnetKind::Application && info.subnet_label == *profile,
            )
        }
    }
}

fn resolve_funding(
    owner: &str,
    subnet: SubnetId,
    funding: &CreationFundingDocument,
    build_network: BuildNetwork,
    catalog: Option<&SubnetCatalog>,
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
    if pool.maximum_size > root.limits.maximum_managed_canisters {
        return Err(FleetInstallInputError::InvalidCanisterPool {
            reason: format!(
                "maximum_size {} exceeds maximum_managed_canisters {}",
                pool.maximum_size, root.limits.maximum_managed_canisters
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
    catalog: Option<&SubnetCatalog>,
) -> Result<(), FleetInstallInputError> {
    if build_network != BuildNetwork::Ic || imports.is_empty() {
        return Ok(());
    }
    let catalog = catalog.ok_or_else(|| FleetInstallInputError::TrustedMetadataRequired {
        selector: format!("Canister pool imports for Subnet {expected_subnet}"),
    })?;
    for canister in imports {
        let resolved = catalog
            .resolve_canister(&canister.to_text())
            .map_err(|error| FleetInstallInputError::ImportedCanisterRoute {
                canister: *canister,
                reason: error.to_string(),
            })?;
        let actual = parse_subnet(
            "trusted Canister routing catalog",
            &resolved.subnet.subnet_principal,
        )?;
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
    catalog: Option<&SubnetCatalog>,
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
    catalog: Option<&SubnetCatalog>,
    subnet: SubnetId,
) -> Result<&SubnetInfo, FleetInstallInputError> {
    let catalog = catalog.ok_or_else(|| FleetInstallInputError::TrustedMetadataRequired {
        selector: format!("explicit Subnet {subnet}"),
    })?;
    catalog
        .subnets
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
    catalog: &SubnetCatalog,
    selector: &str,
    matches: impl Fn(&SubnetInfo) -> bool,
) -> Result<SubnetId, FleetInstallInputError> {
    let candidates = catalog
        .subnets
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

//! Module: canic_cli::info_subnets::model
//!
//! Responsibility: validate and aggregate live Fleet Subnet inventory evidence.
//! Does not own: network queries, catalog persistence, or output rendering.
//! Boundary: accepts exact catalog/Coordinator/root observations and produces a report only when
//! the complete authority graph agrees.

use std::collections::{BTreeMap, BTreeSet};

use candid::{Principal, types::principal::PrincipalError};
use canic_core::{
    dto::{
        fleet_registry::{
            FleetRegistry, FleetRegistryManifest, FleetRegistryVersion, FleetSubnetRootEntry,
            FleetSubnetRootStatus,
        },
        fleet_subnet_root::FleetSubnetRootCanisterSummary,
    },
    ids::SubnetId,
};
use canic_host::fleet_catalog::FleetCatalogEntryV1;
use serde::Serialize;
use thiserror::Error as ThisError;

const SUBNET_INVENTORY_SCHEMA_VERSION: u32 = 1;
const ROOT_LOCAL_INFRASTRUCTURE_CANISTERS: u32 = 2;

///
/// FleetSubnetInventoryReportV1
///
/// Schema-versioned complete live inventory returned by `canic info subnets`.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct FleetSubnetInventoryReportV1 {
    pub schema_version: u32,
    pub canonical_network_id: String,
    pub fleet_id: String,
    pub fleet: String,
    pub app: String,
    pub coordinator_principal: String,
    pub registry_revision: u64,
    pub total_canisters: u32,
    pub subnets: Vec<FleetSubnetInventoryRowV1>,
}

///
/// FleetSubnetInventoryRowV1
///
/// One physical Subnet occupied by the selected Fleet.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct FleetSubnetInventoryRowV1 {
    pub subnet: String,
    pub coordinator_canisters: u32,
    pub root: Option<String>,
    pub status: Option<String>,
    pub root_infrastructure_canisters: u32,
    pub component_canisters: u32,
    pub pooled_canisters: u32,
    pub total_canisters: u32,
}

///
/// SubnetInventoryError
///
/// Typed rejection of incomplete or contradictory live inventory evidence.
///

#[derive(Debug, ThisError)]
pub enum SubnetInventoryError {
    #[error("Fleet catalog Coordinator principal is invalid: {0}")]
    CatalogCoordinator(#[source] PrincipalError),

    #[error("Fleet Subnet inventory count overflow")]
    CountOverflow,

    #[error("Fleet Registry authority disagrees with catalog field {field}")]
    CatalogAuthorityMismatch { field: &'static str },

    #[error("Fleet Registry contains duplicate root principal {root}")]
    DuplicateRegistryRoot { root: Principal },

    #[error("Fleet Subnet Root summary repeats root {root}")]
    DuplicateSummary { root: Principal },

    #[error("Fleet Subnet Root summary is missing for root {root}")]
    MissingSummary { root: Principal },

    #[error("Fleet Registry root order is not strictly ascending by physical Subnet")]
    NonCanonicalRootOrder,

    #[error("Fleet Registry evidence is inconsistent: {field}")]
    RegistryEvidenceMismatch { field: &'static str },

    #[error("Fleet Registry contains invalid root authority for {root}")]
    RegistryRootAuthority { root: Principal },

    #[error("Fleet Subnet Root summary disagrees with Coordinator authority for root {root}")]
    SummaryMismatch { root: Principal },

    #[error("Fleet Subnet Root summary names unknown or removed root {root}")]
    UnexpectedSummary { root: Principal },
}

///
/// SubnetInventoryPlan
///
/// Validated Coordinator evidence that identifies the exact root summary query set.
///

pub(super) struct SubnetInventoryPlan {
    catalog: FleetCatalogEntryV1,
    registry: FleetRegistry,
    version: FleetRegistryVersion,
    roots: Vec<FleetSubnetRootEntry>,
}

impl SubnetInventoryPlan {
    /// Validate Coordinator evidence before any Fleet Subnet Root is queried.
    pub(super) fn compile(
        catalog: FleetCatalogEntryV1,
        registry: FleetRegistry,
        manifest: FleetRegistryManifest,
        version: FleetRegistryVersion,
    ) -> Result<Self, SubnetInventoryError> {
        let coordinator = Principal::from_text(&catalog.coordinator_principal)
            .map_err(SubnetInventoryError::CatalogCoordinator)?;
        validate_catalog_authority(&catalog, &registry, coordinator)?;
        validate_registry_evidence(&registry, &manifest, &version)?;
        validate_registry_roots(&registry, coordinator)?;
        let roots = registry
            .fleet_subnet_roots
            .iter()
            .filter(|root| root.status != FleetSubnetRootStatus::Removed)
            .cloned()
            .collect();

        Ok(Self {
            catalog,
            registry,
            version,
            roots,
        })
    }

    /// Return the exact non-removed root principals that must supply summaries.
    pub(super) fn root_principals(&self) -> Vec<Principal> {
        self.roots
            .iter()
            .map(|root| root.fleet_subnet_root)
            .collect()
    }

    /// Validate every expected summary and aggregate one canonical report.
    pub(super) fn complete(
        self,
        summaries: Vec<FleetSubnetRootCanisterSummary>,
    ) -> Result<FleetSubnetInventoryReportV1, SubnetInventoryError> {
        let mut summaries = index_summaries(summaries)?;
        let mut rows = BTreeMap::<SubnetId, FleetSubnetInventoryRowV1>::new();
        let coordinator_subnet = self.registry.authority.binding.coordinator_subnet;
        rows.insert(
            coordinator_subnet,
            FleetSubnetInventoryRowV1 {
                subnet: coordinator_subnet.to_string(),
                coordinator_canisters: 1,
                root: None,
                status: None,
                root_infrastructure_canisters: 0,
                component_canisters: 0,
                pooled_canisters: 0,
                total_canisters: 1,
            },
        );

        for root in &self.roots {
            let summary = summaries.remove(&root.fleet_subnet_root).ok_or(
                SubnetInventoryError::MissingSummary {
                    root: root.fleet_subnet_root,
                },
            )?;
            validate_summary(root, &self.version, &summary)?;
            add_root_row(&mut rows, root, &summary)?;
        }
        if let Some(root) = summaries.into_keys().next() {
            return Err(SubnetInventoryError::UnexpectedSummary { root });
        }

        let total_canisters = rows.values().try_fold(0_u32, |total, row| {
            total
                .checked_add(row.total_canisters)
                .ok_or(SubnetInventoryError::CountOverflow)
        })?;
        Ok(FleetSubnetInventoryReportV1 {
            schema_version: SUBNET_INVENTORY_SCHEMA_VERSION,
            canonical_network_id: self.catalog.canonical_network_id.to_string(),
            fleet_id: self.catalog.fleet_id.to_string(),
            fleet: self.catalog.fleet_name.to_string(),
            app: self.catalog.app.to_string(),
            coordinator_principal: self.catalog.coordinator_principal,
            registry_revision: self.version.revision,
            total_canisters,
            subnets: rows.into_values().collect(),
        })
    }
}

fn validate_catalog_authority(
    catalog: &FleetCatalogEntryV1,
    registry: &FleetRegistry,
    coordinator: Principal,
) -> Result<(), SubnetInventoryError> {
    let binding = &registry.authority.binding;
    if binding.fleet.fleet.canonical_network_id != catalog.canonical_network_id {
        return Err(SubnetInventoryError::CatalogAuthorityMismatch {
            field: "canonical_network_id",
        });
    }
    if binding.fleet.fleet.fleet_id != catalog.fleet_id {
        return Err(SubnetInventoryError::CatalogAuthorityMismatch { field: "fleet_id" });
    }
    if binding.fleet.app != catalog.app {
        return Err(SubnetInventoryError::CatalogAuthorityMismatch { field: "app" });
    }
    if binding.coordinator != coordinator {
        return Err(SubnetInventoryError::CatalogAuthorityMismatch {
            field: "coordinator_principal",
        });
    }
    if is_reserved_principal(coordinator) {
        return Err(SubnetInventoryError::CatalogAuthorityMismatch {
            field: "coordinator_principal",
        });
    }
    if is_reserved_principal(binding.coordinator_subnet.into_principal()) {
        return Err(SubnetInventoryError::CatalogAuthorityMismatch {
            field: "coordinator_subnet",
        });
    }
    Ok(())
}

fn validate_registry_evidence(
    registry: &FleetRegistry,
    manifest: &FleetRegistryManifest,
    version: &FleetRegistryVersion,
) -> Result<(), SubnetInventoryError> {
    if registry.authority.epoch == 0 {
        return Err(SubnetInventoryError::RegistryEvidenceMismatch {
            field: "authority epoch",
        });
    }
    if registry.revision == 0 {
        return Err(SubnetInventoryError::RegistryEvidenceMismatch {
            field: "Registry revision",
        });
    }
    if registry.authority != manifest.authority || registry.authority != version.authority {
        return Err(SubnetInventoryError::RegistryEvidenceMismatch { field: "authority" });
    }
    if registry.revision != manifest.revision || registry.revision != version.revision {
        return Err(SubnetInventoryError::RegistryEvidenceMismatch { field: "revision" });
    }
    if manifest.byte_length == 0 {
        return Err(SubnetInventoryError::RegistryEvidenceMismatch {
            field: "manifest byte length",
        });
    }
    if manifest.content_hash != version.content_hash {
        return Err(SubnetInventoryError::RegistryEvidenceMismatch {
            field: "content hash",
        });
    }
    Ok(())
}

fn validate_registry_roots(
    registry: &FleetRegistry,
    coordinator: Principal,
) -> Result<(), SubnetInventoryError> {
    if registry
        .fleet_subnet_roots
        .windows(2)
        .any(|roots| roots[0].placement_subnet >= roots[1].placement_subnet)
    {
        return Err(SubnetInventoryError::NonCanonicalRootOrder);
    }

    let mut principals = BTreeSet::new();
    for root in &registry.fleet_subnet_roots {
        let principal = root.fleet_subnet_root;
        if !principals.insert(principal) {
            return Err(SubnetInventoryError::DuplicateRegistryRoot { root: principal });
        }
        if principal == coordinator
            || is_reserved_principal(principal)
            || is_reserved_principal(root.placement_subnet.into_principal())
        {
            return Err(SubnetInventoryError::RegistryRootAuthority { root: principal });
        }
    }
    Ok(())
}

fn index_summaries(
    summaries: Vec<FleetSubnetRootCanisterSummary>,
) -> Result<BTreeMap<Principal, FleetSubnetRootCanisterSummary>, SubnetInventoryError> {
    let mut indexed = BTreeMap::new();
    for summary in summaries {
        let root = summary.fleet_subnet_root;
        if indexed.insert(root, summary).is_some() {
            return Err(SubnetInventoryError::DuplicateSummary { root });
        }
    }
    Ok(indexed)
}

fn validate_summary(
    root: &FleetSubnetRootEntry,
    version: &FleetRegistryVersion,
    summary: &FleetSubnetRootCanisterSummary,
) -> Result<(), SubnetInventoryError> {
    let expected_total = summary
        .infrastructure_canisters
        .checked_add(summary.component_canisters)
        .and_then(|count| count.checked_add(summary.pooled_canisters))
        .ok_or(SubnetInventoryError::CountOverflow)?;
    if &summary.fleet_registry != version
        || summary.placement_subnet != root.placement_subnet
        || summary.fleet_subnet_root != root.fleet_subnet_root
        || summary.status != root.status
        || summary.infrastructure_canisters != ROOT_LOCAL_INFRASTRUCTURE_CANISTERS
        || summary.total_canisters != expected_total
    {
        return Err(SubnetInventoryError::SummaryMismatch {
            root: root.fleet_subnet_root,
        });
    }
    Ok(())
}

fn add_root_row(
    rows: &mut BTreeMap<SubnetId, FleetSubnetInventoryRowV1>,
    root: &FleetSubnetRootEntry,
    summary: &FleetSubnetRootCanisterSummary,
) -> Result<(), SubnetInventoryError> {
    let row = rows
        .entry(root.placement_subnet)
        .or_insert_with(|| FleetSubnetInventoryRowV1 {
            subnet: root.placement_subnet.to_string(),
            coordinator_canisters: 0,
            root: None,
            status: None,
            root_infrastructure_canisters: 0,
            component_canisters: 0,
            pooled_canisters: 0,
            total_canisters: 0,
        });
    if row.root.is_some() {
        return Err(SubnetInventoryError::NonCanonicalRootOrder);
    }
    row.root = Some(root.fleet_subnet_root.to_text());
    row.status = Some(status_label(root.status).to_string());
    row.root_infrastructure_canisters = summary.infrastructure_canisters;
    row.component_canisters = summary.component_canisters;
    row.pooled_canisters = summary.pooled_canisters;
    row.total_canisters = row
        .total_canisters
        .checked_add(summary.total_canisters)
        .ok_or(SubnetInventoryError::CountOverflow)?;
    Ok(())
}

const fn status_label(status: FleetSubnetRootStatus) -> &'static str {
    match status {
        FleetSubnetRootStatus::Joining => "joining",
        FleetSubnetRootStatus::Active => "active",
        FleetSubnetRootStatus::Draining => "draining",
        FleetSubnetRootStatus::Removed => "removed",
    }
}

fn is_reserved_principal(principal: Principal) -> bool {
    principal == Principal::anonymous() || principal == Principal::management_canister()
}

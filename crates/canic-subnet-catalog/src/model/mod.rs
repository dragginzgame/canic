use crate::{
    CATALOG_SCHEMA_VERSION, CatalogError, parse_principal, principal_bytes,
    resolver::routing_range_sorts_after,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

///
/// SubnetCatalog
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetCatalog {
    pub catalog_schema_version: u32,
    pub network: String,
    pub registry_canister_id: String,
    pub registry_version: u64,
    pub fetched_at: String,
    pub fetched_by: String,
    pub source_endpoint: String,
    pub resolver_backend: String,
    pub subnets: Vec<SubnetInfo>,
    pub routing_ranges: Vec<RoutingRange>,
}

///
/// SubnetInfo
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubnetInfo {
    pub subnet_principal: String,
    pub subnet_kind: SubnetKind,
    pub subnet_kind_source: ClassificationSource,
    pub subnet_specialization: SubnetSpecialization,
    pub subnet_specialization_source: ClassificationSource,
    pub geographic_scope: GeographicScope,
    pub geographic_scope_source: ClassificationSource,
    pub subnet_label: String,
    pub subnet_label_source: ClassificationSource,
    pub node_count: Option<u32>,
    pub charges_apply_by_default: bool,
}

///
/// RoutingRange
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoutingRange {
    pub start_canister_id: String,
    pub end_canister_id: String,
    pub subnet_principal: String,
}

///
/// SubnetKind
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubnetKind {
    Application,
    System,
    Unknown,
}

impl SubnetKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::System => "system",
            Self::Unknown => "unknown",
        }
    }
}

///
/// SubnetSpecialization
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubnetSpecialization {
    None,
    Fiduciary,
    European,
    Unknown,
}

impl SubnetSpecialization {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Fiduciary => "fiduciary",
            Self::European => "european",
            Self::Unknown => "unknown",
        }
    }
}

///
/// GeographicScope
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GeographicScope {
    Global,
    Europe,
    Unknown,
}

impl GeographicScope {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Europe => "europe",
            Self::Unknown => "unknown",
        }
    }
}

///
/// ClassificationSource
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationSource {
    Registry,
    Curated,
    Computed,
    Unknown,
}

impl ClassificationSource {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Registry => "registry",
            Self::Curated => "curated",
            Self::Computed => "computed",
            Self::Unknown => "unknown",
        }
    }
}

impl SubnetCatalog {
    /// Validate schema, principal syntax, and routing references.
    pub fn validate(&self) -> Result<(), CatalogError> {
        if self.catalog_schema_version != CATALOG_SCHEMA_VERSION {
            return Err(CatalogError::UnsupportedSchemaVersion {
                found: self.catalog_schema_version,
                supported: CATALOG_SCHEMA_VERSION,
            });
        }
        if self.subnets.is_empty() {
            return Err(CatalogError::EmptySubnets);
        }
        if self.routing_ranges.is_empty() {
            return Err(CatalogError::EmptyRoutingRanges);
        }
        parse_principal(&self.registry_canister_id, "registry_canister_id")?;

        let mut subnet_principals = BTreeSet::new();
        for subnet in &self.subnets {
            parse_principal(&subnet.subnet_principal, "subnet_principal")?;
            if !subnet_principals.insert(subnet.subnet_principal.clone()) {
                return Err(CatalogError::DuplicateSubnet {
                    subnet_principal: subnet.subnet_principal.clone(),
                });
            }
        }

        for range in &self.routing_ranges {
            if !subnet_principals.contains(&range.subnet_principal) {
                return Err(CatalogError::UnknownRoutingSubnet {
                    subnet_principal: range.subnet_principal.clone(),
                });
            }
            let start = principal_bytes(&range.start_canister_id, "start_canister_id")?;
            let end = principal_bytes(&range.end_canister_id, "end_canister_id")?;
            parse_principal(&range.subnet_principal, "routing_range.subnet_principal")?;
            if routing_range_sorts_after(&start, &end) {
                return Err(CatalogError::InvalidRoutingRange {
                    subnet_principal: range.subnet_principal.clone(),
                    start_canister_id: range.start_canister_id.clone(),
                    end_canister_id: range.end_canister_id.clone(),
                });
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn subnet_by_principal(&self, subnet_principal: &str) -> Option<&SubnetInfo> {
        self.subnets
            .iter()
            .find(|subnet| subnet.subnet_principal == subnet_principal)
    }

    #[must_use]
    pub fn routing_ranges_for_subnet(&self, subnet_principal: &str) -> Vec<&RoutingRange> {
        self.routing_ranges
            .iter()
            .filter(|range| range.subnet_principal == subnet_principal)
            .collect()
    }
}

//! Pure cached IC subnet catalog model and resolver for Canic host tools.

pub mod model;
pub mod resolver;

use candid::Principal;
pub use model::{
    ClassificationSource, GeographicScope, RoutingRange, SubnetCatalog, SubnetInfo, SubnetKind,
    SubnetSpecialization,
};
pub use resolver::{ResolveAs, ResolvedSubnet, ResolvedSubnetSubject};
use thiserror::Error as ThisError;

pub const CATALOG_SCHEMA_VERSION: u32 = 1;
pub const MAINNET_NETWORK: &str = "ic";
pub const MAINNET_REGISTRY_CANISTER_ID: &str = "rwlgt-iiaaa-aaaaa-aaaaa-cai";

///
/// CatalogError
///
#[derive(Debug, ThisError)]
pub enum CatalogError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("unsupported subnet catalog schema version {found}; supported version is {supported}")]
    UnsupportedSchemaVersion { found: u32, supported: u32 },

    #[error("subnet catalog must contain at least one subnet")]
    EmptySubnets,

    #[error("subnet catalog must contain at least one routing range")]
    EmptyRoutingRanges,

    #[error("invalid principal in {field}: {value}: {reason}")]
    InvalidPrincipal {
        field: &'static str,
        value: String,
        reason: String,
    },

    #[error("duplicate subnet principal in catalog: {subnet_principal}")]
    DuplicateSubnet { subnet_principal: String },

    #[error("routing range references unknown subnet: {subnet_principal}")]
    UnknownRoutingSubnet { subnet_principal: String },

    #[error(
        "invalid routing range for {subnet_principal}: start {start_canister_id} sorts after end {end_canister_id}"
    )]
    InvalidRoutingRange {
        subnet_principal: String,
        start_canister_id: String,
        end_canister_id: String,
    },

    #[error("subnet principal {subnet_principal} was not found in the cached catalog")]
    UnknownSubnet { subnet_principal: String },

    #[error(
        "canister principal {canister_principal} was not covered by cached routing ranges at registry_version={registry_version}, catalog_schema_version={catalog_schema_version}"
    )]
    RouteNotFound {
        canister_principal: String,
        registry_version: u64,
        catalog_schema_version: u32,
    },
}

/// Decode and validate one subnet catalog JSON payload.
pub fn parse_catalog_json(data: &str) -> Result<SubnetCatalog, CatalogError> {
    let catalog = serde_json::from_str::<SubnetCatalog>(data)?;
    catalog.validate()?;
    Ok(catalog)
}

/// Render one subnet catalog JSON payload with stable pretty formatting.
pub fn catalog_to_pretty_json(catalog: &SubnetCatalog) -> Result<String, CatalogError> {
    Ok(serde_json::to_string_pretty(catalog)?)
}

/// Parse a textual IC principal into canonical text.
pub fn canonical_principal_text(value: &str) -> Result<String, CatalogError> {
    Ok(parse_principal(value, "principal")?.to_text())
}

pub(crate) fn parse_principal(value: &str, field: &'static str) -> Result<Principal, CatalogError> {
    Principal::from_text(value).map_err(|err| CatalogError::InvalidPrincipal {
        field,
        value: value.to_string(),
        reason: err.to_string(),
    })
}

pub(crate) fn principal_bytes(value: &str, field: &'static str) -> Result<Vec<u8>, CatalogError> {
    Ok(parse_principal(value, field)?.as_slice().to_vec())
}

#[cfg(test)]
mod tests;

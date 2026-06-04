use super::*;
use canic_host::subnet_catalog::{
    DEFAULT_STALE_AFTER_SECONDS, SubnetCatalogCacheRequest, catalog_stale_status,
    load_cached_subnet_catalog, parse_stale_after_duration,
};
use canic_subnet_catalog::{MAINNET_NETWORK, ResolvedSubnet, RoutingRange, SubnetKind};
use std::{
    env,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) const ESTIMATE_SCHEMA_VERSION: u8 = 1;
pub(super) const FORMULA_VERSION: &str = "canic-0.59-ic-cycle-costs-v1";
const CATALOG_FORMULA_VERSION: &str = "base_13_node_linear_v1";
const ESTIMATE_KIND_PER_INSTRUCTION_COMPONENT: &str = "per_instruction_component_only";
const CHARGE_MODEL_UPDATE_EXECUTION_COMPONENT: &str = "hypothetical_update_execution_component";
const SUBNET_SOURCE_FLAG: &str = "flag";
const SUBNET_SOURCE_EXPLICIT_RATE: &str = "explicit-rate";
const SUBNET_SOURCE_NNS_REGISTRY_CACHE: &str = "nns-registry-cache";
const SOURCE_MEANING_OPERATOR_SUPPLIED: &str = "operator_supplied_pricing_assumption";
const SOURCE_MEANING_NNS_REGISTRY_CACHE: &str = "resolved_from_nns_registry_cache";
const RATE_SOURCE_OFFICIAL_DOCS: &str = "official-ic-cycle-costs-docs";
const RATE_SOURCE_OPERATOR_EXPLICIT: &str = "operator-explicit-rate";
const RATE_SOURCE_NNS_REGISTRY_CACHE: &str = "nns-registry-cache";
const BILLION: u128 = 1_000_000_000;
const THIRTEEN_NODE_CYCLES_PER_BILLION: u128 = 1_000_000_000;
const THIRTY_FOUR_NODE_CYCLES_PER_BILLION: u128 = 2_615_384_615;
const OMITTED_COSTS: &[&str] = &[
    "update_message_execution_base_fee",
    "message_transmission_base_fee",
    "payload_bytes",
    "garbage_collection",
    "callee_instructions",
    "management_call_fees",
    "storage_and_reservations",
];
const ENV_ESTIMATE_EXECUTION_CYCLES: &str = "CANIC_INSTRUCTION_AUDIT_ESTIMATE_EXECUTION_CYCLES";
const ENV_ESTIMATE_NODE_COUNT: &str = "CANIC_INSTRUCTION_AUDIT_ESTIMATE_NODE_COUNT";
const ENV_CYCLES_PER_BILLION_INSTRUCTIONS: &str =
    "CANIC_INSTRUCTION_AUDIT_CYCLES_PER_BILLION_INSTRUCTIONS";
const ENV_ESTIMATE_CANISTER_PRINCIPAL: &str = "CANIC_INSTRUCTION_AUDIT_ESTIMATE_CANISTER_PRINCIPAL";
const ENV_ALLOW_STALE_SUBNET_CATALOG: &str = "CANIC_INSTRUCTION_AUDIT_ALLOW_STALE_SUBNET_CATALOG";
const ENV_SUBNET_CATALOG_STALE_AFTER: &str = "CANIC_INSTRUCTION_AUDIT_SUBNET_CATALOG_STALE_AFTER";

///
/// EstimateError
///

#[derive(Debug, Eq, PartialEq)]
pub(super) enum EstimateError {
    MissingEstimateSource,
    EstimateSourceWithoutEstimateFlag,
    CatalogStaleControlWithoutCatalogSource,
    UnsupportedNodeCount(u16),
    InvalidBooleanFlag { field: &'static str, value: String },
    InvalidPositiveInteger { field: &'static str, value: String },
    InvalidDuration { field: &'static str, value: String },
    InvalidText { field: &'static str, value: String },
    Overflow,
    Clock(String),
}

impl std::fmt::Display for EstimateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEstimateSource => formatter.write_str(
                "--estimate-execution-cycles requires --estimate-node-count, --cycles-per-billion-instructions, or --estimate-canister-principal",
            ),
            Self::EstimateSourceWithoutEstimateFlag => formatter.write_str(
                "--estimate-node-count, --cycles-per-billion-instructions, and --estimate-canister-principal require --estimate-execution-cycles",
            ),
            Self::CatalogStaleControlWithoutCatalogSource => {
                formatter.write_str("catalog stale options require --estimate-canister-principal")
            }
            Self::UnsupportedNodeCount(node_count) => write!(
                formatter,
                "unsupported --estimate-node-count {node_count}; use 13, 34, or provide --cycles-per-billion-instructions",
            ),
            Self::InvalidBooleanFlag { field, value } => {
                write!(formatter, "{field} must be 0, 1, true, or false, got {value:?}")
            }
            Self::InvalidPositiveInteger { field, value } => {
                write!(formatter, "{field} must be a positive integer, got {value:?}")
            }
            Self::InvalidDuration { field, value } => write!(
                formatter,
                "{field} must be positive seconds or a value ending in s, m, h, or d, got {value:?}"
            ),
            Self::InvalidText { field, value } => {
                write!(formatter, "{field} must be valid UTF-8 text, got {value:?}")
            }
            Self::Overflow => formatter.write_str("instruction cycle estimate overflowed u128"),
            Self::Clock(reason) => write!(formatter, "failed to read system clock: {reason}"),
        }
    }
}

impl std::error::Error for EstimateError {}

///
/// EstimateOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EstimateOptions {
    pub enabled: bool,
    pub node_count: Option<u16>,
    pub explicit_cycles_per_billion_instructions: Option<u128>,
    pub catalog_canister_principal: Option<String>,
    pub allow_stale_subnet_catalog: bool,
    pub subnet_catalog_stale_after_seconds: u64,
    pub subnet_catalog_stale_after_configured: bool,
    pub catalog_root: PathBuf,
    pub now_unix_secs: u64,
}

///
/// RateSelection
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct RateSelection {
    cycles_per_billion_instructions: u128,
    node_count_table_rate: Option<u128>,
    overrode_node_count_table_rate: bool,
    subnet_source: &'static str,
    source_meaning: &'static str,
    formula_version: &'static str,
    rate_source: &'static str,
    subnet_node_count: Option<u32>,
    catalog: Option<CatalogEstimateProvenance>,
}

///
/// CatalogEstimateProvenance
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct CatalogEstimateProvenance {
    registry_canister_id: String,
    registry_version: u64,
    subnet_principal: String,
    subnet_kind: String,
    subnet_kind_source: String,
    subnet_specialization: String,
    subnet_specialization_source: String,
    geographic_scope: String,
    geographic_scope_source: String,
    catalog_schema_version: u32,
    catalog_stale: bool,
    resolver_backend: String,
    matched_canister_principal: Option<String>,
    matched_routing_range: Option<RoutingRange>,
}

///
/// ExecutionCycleEstimate
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct ExecutionCycleEstimate {
    pub estimate_schema_version: u8,
    pub kind: &'static str,
    pub charge_model: &'static str,
    pub local_instructions: u64,
    pub counter_id: u8,
    pub sample_origin: String,
    pub estimated_instruction_cycles: String,
    pub cycles_per_billion_instructions: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_node_count: Option<u32>,
    pub subnet_source: &'static str,
    pub source_meaning: &'static str,
    pub formula_version: &'static str,
    pub rate_source: &'static str,
    pub overrode_node_count_table_rate: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_count_table_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_canister_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_version: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_principal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_kind_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_specialization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subnet_specialization_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geographic_scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geographic_scope_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_schema_version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog_stale: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolver_backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_canister_principal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_routing_range: Option<RoutingRange>,
    pub omitted_costs: &'static [&'static str],
}

impl EstimateOptions {
    pub(super) const fn disabled() -> Self {
        Self {
            enabled: false,
            node_count: None,
            explicit_cycles_per_billion_instructions: None,
            catalog_canister_principal: None,
            allow_stale_subnet_catalog: false,
            subnet_catalog_stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
            subnet_catalog_stale_after_configured: false,
            catalog_root: PathBuf::new(),
            now_unix_secs: 0,
        }
    }

    fn validate(self) -> Result<Self, EstimateError> {
        if !self.enabled {
            if self.node_count.is_some()
                || self.explicit_cycles_per_billion_instructions.is_some()
                || self.catalog_canister_principal.is_some()
                || self.allow_stale_subnet_catalog
                || self.subnet_catalog_stale_after_configured
            {
                return Err(EstimateError::EstimateSourceWithoutEstimateFlag);
            }

            return Ok(Self::disabled());
        }

        if self.node_count.is_none()
            && self.explicit_cycles_per_billion_instructions.is_none()
            && self.catalog_canister_principal.is_none()
        {
            return Err(EstimateError::MissingEstimateSource);
        }
        if self.catalog_canister_principal.is_none()
            && (self.allow_stale_subnet_catalog || self.subnet_catalog_stale_after_configured)
        {
            return Err(EstimateError::CatalogStaleControlWithoutCatalogSource);
        }

        if self.explicit_cycles_per_billion_instructions.is_none()
            && let Some(node_count) = self.node_count
            && node_count_table_rate(node_count).is_none()
        {
            return Err(EstimateError::UnsupportedNodeCount(node_count));
        }
        Ok(self)
    }
}

pub(super) fn estimate_options_from_env(
    catalog_root: &Path,
) -> Result<EstimateOptions, EstimateError> {
    let enabled = env_flag(ENV_ESTIMATE_EXECUTION_CYCLES)?;
    let node_count = optional_positive_u16_env(ENV_ESTIMATE_NODE_COUNT)?;
    let explicit_cycles_per_billion_instructions =
        optional_positive_u128_env(ENV_CYCLES_PER_BILLION_INSTRUCTIONS)?;
    let catalog_canister_principal = optional_string_env(ENV_ESTIMATE_CANISTER_PRINCIPAL)?;
    let allow_stale_subnet_catalog = env_flag(ENV_ALLOW_STALE_SUBNET_CATALOG)?;
    let subnet_catalog_stale_after = optional_duration_env(ENV_SUBNET_CATALOG_STALE_AFTER)?;
    let subnet_catalog_stale_after_configured = subnet_catalog_stale_after.is_some();
    let subnet_catalog_stale_after_seconds =
        subnet_catalog_stale_after.unwrap_or(DEFAULT_STALE_AFTER_SECONDS);

    EstimateOptions {
        enabled,
        node_count,
        explicit_cycles_per_billion_instructions,
        catalog_canister_principal,
        allow_stale_subnet_catalog,
        subnet_catalog_stale_after_seconds,
        subnet_catalog_stale_after_configured,
        catalog_root: catalog_root.to_path_buf(),
        now_unix_secs: now_unix_secs()?,
    }
    .validate()
}

pub(super) fn apply_execution_cycle_estimates(
    results: &mut [ScenarioResult],
    options: EstimateOptions,
) -> Result<(), EstimateError> {
    if !options.enabled {
        return Ok(());
    }

    let Some(selection) = select_rate(&options)? else {
        return Ok(());
    };

    for result in results {
        if result.row.sample_origin != "update" {
            continue;
        }

        result.row.execution_cycle_estimate = Some(execution_cycle_estimate(
            result.row.total_local_instructions,
            &result.row.sample_origin,
            &selection,
        )?);
    }

    Ok(())
}

fn execution_cycle_estimate(
    local_instructions: u64,
    sample_origin: &str,
    selection: &RateSelection,
) -> Result<ExecutionCycleEstimate, EstimateError> {
    let estimated_instruction_cycles = estimate_instruction_cycles(
        u128::from(local_instructions),
        selection.cycles_per_billion_instructions,
    )?;
    let catalog = selection.catalog.clone();

    Ok(ExecutionCycleEstimate {
        estimate_schema_version: ESTIMATE_SCHEMA_VERSION,
        kind: ESTIMATE_KIND_PER_INSTRUCTION_COMPONENT,
        charge_model: CHARGE_MODEL_UPDATE_EXECUTION_COMPONENT,
        local_instructions,
        counter_id: PERF_COUNTER_ID,
        sample_origin: sample_origin.to_string(),
        estimated_instruction_cycles: estimated_instruction_cycles.to_string(),
        cycles_per_billion_instructions: selection.cycles_per_billion_instructions.to_string(),
        subnet_node_count: selection.subnet_node_count,
        subnet_source: selection.subnet_source,
        source_meaning: selection.source_meaning,
        formula_version: selection.formula_version,
        rate_source: selection.rate_source,
        overrode_node_count_table_rate: selection.overrode_node_count_table_rate,
        node_count_table_rate: selection.node_count_table_rate.map(|rate| rate.to_string()),
        registry_canister_id: catalog
            .as_ref()
            .map(|provenance| provenance.registry_canister_id.clone()),
        registry_version: catalog
            .as_ref()
            .map(|provenance| provenance.registry_version),
        subnet_principal: catalog
            .as_ref()
            .map(|provenance| provenance.subnet_principal.clone()),
        subnet_kind: catalog
            .as_ref()
            .map(|provenance| provenance.subnet_kind.clone()),
        subnet_kind_source: catalog
            .as_ref()
            .map(|provenance| provenance.subnet_kind_source.clone()),
        subnet_specialization: catalog
            .as_ref()
            .map(|provenance| provenance.subnet_specialization.clone()),
        subnet_specialization_source: catalog
            .as_ref()
            .map(|provenance| provenance.subnet_specialization_source.clone()),
        geographic_scope: catalog
            .as_ref()
            .map(|provenance| provenance.geographic_scope.clone()),
        geographic_scope_source: catalog
            .as_ref()
            .map(|provenance| provenance.geographic_scope_source.clone()),
        catalog_schema_version: catalog
            .as_ref()
            .map(|provenance| provenance.catalog_schema_version),
        catalog_stale: catalog.as_ref().map(|provenance| provenance.catalog_stale),
        resolver_backend: catalog
            .as_ref()
            .map(|provenance| provenance.resolver_backend.clone()),
        matched_canister_principal: catalog
            .as_ref()
            .and_then(|provenance| provenance.matched_canister_principal.clone()),
        matched_routing_range: catalog
            .as_ref()
            .and_then(|provenance| provenance.matched_routing_range.clone()),
        omitted_costs: OMITTED_COSTS,
    })
}

fn select_rate(options: &EstimateOptions) -> Result<Option<RateSelection>, EstimateError> {
    let node_count_table_rate = options.node_count.and_then(node_count_table_rate);

    if let Some(explicit_rate) = options.explicit_cycles_per_billion_instructions {
        return Ok(Some(RateSelection {
            cycles_per_billion_instructions: explicit_rate,
            node_count_table_rate,
            overrode_node_count_table_rate: node_count_table_rate.is_some(),
            subnet_source: SUBNET_SOURCE_EXPLICIT_RATE,
            source_meaning: SOURCE_MEANING_OPERATOR_SUPPLIED,
            formula_version: FORMULA_VERSION,
            rate_source: RATE_SOURCE_OPERATOR_EXPLICIT,
            subnet_node_count: options.node_count.map(u32::from),
            catalog: None,
        }));
    }

    if let Some(node_count) = options.node_count {
        let Some(rate) = node_count_table_rate else {
            return Err(EstimateError::UnsupportedNodeCount(node_count));
        };

        return Ok(Some(RateSelection {
            cycles_per_billion_instructions: rate,
            node_count_table_rate: Some(rate),
            overrode_node_count_table_rate: false,
            subnet_source: SUBNET_SOURCE_FLAG,
            source_meaning: SOURCE_MEANING_OPERATOR_SUPPLIED,
            formula_version: FORMULA_VERSION,
            rate_source: RATE_SOURCE_OFFICIAL_DOCS,
            subnet_node_count: Some(u32::from(node_count)),
            catalog: None,
        }));
    }

    let Some(canister_principal) = options.catalog_canister_principal.as_deref() else {
        return Err(EstimateError::MissingEstimateSource);
    };

    catalog_rate_selection(options, canister_principal)
}

fn catalog_rate_selection(
    options: &EstimateOptions,
    canister_principal: &str,
) -> Result<Option<RateSelection>, EstimateError> {
    let Ok(cached) = load_cached_subnet_catalog(&SubnetCatalogCacheRequest {
        icp_root: options.catalog_root.clone(),
        network: MAINNET_NETWORK.to_string(),
    }) else {
        return Ok(None);
    };
    let stale = catalog_stale_status(
        &cached.catalog,
        options.now_unix_secs,
        options.subnet_catalog_stale_after_seconds,
    );
    if stale.catalog_stale && !options.allow_stale_subnet_catalog {
        return Ok(None);
    }

    let Ok(resolved) = cached.catalog.resolve_canister(canister_principal) else {
        return Ok(None);
    };
    if resolved.subnet.subnet_kind != SubnetKind::Application {
        return Ok(None);
    }

    let Some(node_count) = resolved
        .subnet
        .node_count
        .filter(|node_count| *node_count > 0)
    else {
        return Ok(None);
    };
    let Some(cycles_per_billion_instructions) = catalog_cycles_per_billion(node_count) else {
        return Err(EstimateError::Overflow);
    };

    Ok(Some(RateSelection {
        cycles_per_billion_instructions,
        node_count_table_rate: None,
        overrode_node_count_table_rate: false,
        subnet_source: SUBNET_SOURCE_NNS_REGISTRY_CACHE,
        source_meaning: SOURCE_MEANING_NNS_REGISTRY_CACHE,
        formula_version: CATALOG_FORMULA_VERSION,
        rate_source: RATE_SOURCE_NNS_REGISTRY_CACHE,
        subnet_node_count: Some(node_count),
        catalog: Some(catalog_provenance(
            &cached.catalog,
            &resolved,
            stale.catalog_stale,
        )),
    }))
}

fn catalog_cycles_per_billion(node_count: u32) -> Option<u128> {
    THIRTEEN_NODE_CYCLES_PER_BILLION
        .checked_mul(u128::from(node_count))
        .map(|numerator| ceil_div(numerator, 13))
}

fn catalog_provenance(
    catalog: &canic_subnet_catalog::SubnetCatalog,
    resolved: &ResolvedSubnet,
    catalog_stale: bool,
) -> CatalogEstimateProvenance {
    CatalogEstimateProvenance {
        registry_canister_id: catalog.registry_canister_id.clone(),
        registry_version: catalog.registry_version,
        subnet_principal: resolved.subnet.subnet_principal.clone(),
        subnet_kind: resolved.subnet.subnet_kind.as_str().to_string(),
        subnet_kind_source: resolved.subnet.subnet_kind_source.as_str().to_string(),
        subnet_specialization: resolved.subnet.subnet_specialization.as_str().to_string(),
        subnet_specialization_source: resolved
            .subnet
            .subnet_specialization_source
            .as_str()
            .to_string(),
        geographic_scope: resolved.subnet.geographic_scope.as_str().to_string(),
        geographic_scope_source: resolved.subnet.geographic_scope_source.as_str().to_string(),
        catalog_schema_version: catalog.catalog_schema_version,
        catalog_stale,
        resolver_backend: catalog.resolver_backend.clone(),
        matched_canister_principal: resolved.matched_canister_principal.clone(),
        matched_routing_range: resolved.matched_routing_range.clone(),
    }
}

const fn node_count_table_rate(node_count: u16) -> Option<u128> {
    match node_count {
        13 => Some(THIRTEEN_NODE_CYCLES_PER_BILLION),
        34 => Some(THIRTY_FOUR_NODE_CYCLES_PER_BILLION),
        _ => None,
    }
}

fn estimate_instruction_cycles(
    local_instructions: u128,
    cycles_per_billion_instructions: u128,
) -> Result<u128, EstimateError> {
    let numerator = local_instructions
        .checked_mul(cycles_per_billion_instructions)
        .ok_or(EstimateError::Overflow)?;

    Ok(ceil_div(numerator, BILLION))
}

const fn ceil_div(numerator: u128, denominator: u128) -> u128 {
    if numerator == 0 {
        0
    } else {
        ((numerator - 1) / denominator) + 1
    }
}

fn env_flag(name: &'static str) -> Result<bool, EstimateError> {
    match env::var(name) {
        Ok(value) => parse_env_flag(name, &value),
        Err(env::VarError::NotPresent) => Ok(false),
        Err(env::VarError::NotUnicode(value)) => Err(EstimateError::InvalidBooleanFlag {
            field: name,
            value: value.to_string_lossy().into_owned(),
        }),
    }
}

fn parse_env_flag(field: &'static str, value: &str) -> Result<bool, EstimateError> {
    match value {
        "" | "0" | "false" | "False" | "FALSE" => Ok(false),
        "1" | "true" | "True" | "TRUE" => Ok(true),
        _ => Err(EstimateError::InvalidBooleanFlag {
            field,
            value: value.to_string(),
        }),
    }
}

fn optional_positive_u16_env(name: &'static str) -> Result<Option<u16>, EstimateError> {
    optional_positive_u128_env(name).and_then(|value| {
        value.map_or(Ok(None), |value| {
            u16::try_from(value)
                .ok()
                .filter(|value| *value > 0)
                .map(Some)
                .ok_or_else(|| EstimateError::InvalidPositiveInteger {
                    field: name,
                    value: value.to_string(),
                })
        })
    })
}

fn optional_positive_u128_env(name: &'static str) -> Result<Option<u128>, EstimateError> {
    match env::var(name) {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => parse_positive_u128(name, &value).map(Some),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(value)) => Err(EstimateError::InvalidPositiveInteger {
            field: name,
            value: value.to_string_lossy().into_owned(),
        }),
    }
}

fn optional_string_env(name: &'static str) -> Result<Option<String>, EstimateError> {
    match env::var(name) {
        Ok(value) => {
            let value = value.trim();
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(value.to_string()))
            }
        }
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(value)) => Err(EstimateError::InvalidText {
            field: name,
            value: value.to_string_lossy().into_owned(),
        }),
    }
}

fn optional_duration_env(name: &'static str) -> Result<Option<u64>, EstimateError> {
    match env::var(name) {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => parse_stale_after_duration(&value).map(Some).map_err(|_| {
            EstimateError::InvalidDuration {
                field: name,
                value: value.clone(),
            }
        }),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(value)) => Err(EstimateError::InvalidDuration {
            field: name,
            value: value.to_string_lossy().into_owned(),
        }),
    }
}

fn now_unix_secs() -> Result<u64, EstimateError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|err| EstimateError::Clock(err.to_string()))
}

fn parse_positive_u128(field: &'static str, value: &str) -> Result<u128, EstimateError> {
    let compact = value.trim().replace('_', "");
    compact
        .parse::<u128>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| EstimateError::InvalidPositiveInteger {
            field,
            value: value.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_host::subnet_catalog::subnet_catalog_path;
    use canic_subnet_catalog::{
        CATALOG_SCHEMA_VERSION, ClassificationSource, GeographicScope,
        MAINNET_REGISTRY_CANISTER_ID, SubnetCatalog, SubnetInfo, SubnetSpecialization,
    };
    use serde_json::Value;
    use std::{fs, process};

    const SUBNET_A: &str = "rwlgt-iiaaa-aaaaa-aaaaa-cai";
    const CANISTER_A: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";
    const FRESH_FETCHED_AT: &str = "2026-06-04T00:00:00Z";
    const FRESH_NOW_UNIX_SECS: u64 = 1_780_531_300;

    fn options_with_node_count(node_count: u16) -> EstimateOptions {
        EstimateOptions {
            enabled: true,
            node_count: Some(node_count),
            explicit_cycles_per_billion_instructions: None,
            catalog_canister_principal: None,
            allow_stale_subnet_catalog: false,
            subnet_catalog_stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
            subnet_catalog_stale_after_configured: false,
            catalog_root: PathBuf::new(),
            now_unix_secs: FRESH_NOW_UNIX_SECS,
        }
    }

    fn enabled_without_source() -> EstimateOptions {
        EstimateOptions {
            enabled: true,
            node_count: None,
            explicit_cycles_per_billion_instructions: None,
            catalog_canister_principal: None,
            allow_stale_subnet_catalog: false,
            subnet_catalog_stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
            subnet_catalog_stale_after_configured: false,
            catalog_root: PathBuf::new(),
            now_unix_secs: FRESH_NOW_UNIX_SECS,
        }
    }

    fn catalog_options(root: &Path) -> EstimateOptions {
        EstimateOptions {
            catalog_canister_principal: Some(CANISTER_A.to_string()),
            catalog_root: root.to_path_buf(),
            ..enabled_without_source()
        }
    }

    fn select_required_rate(options: &EstimateOptions) -> RateSelection {
        select_rate(options).expect("rate selection").expect("rate")
    }

    fn estimate_with_options(options: &EstimateOptions) -> ExecutionCycleEstimate {
        let selection = select_required_rate(options);
        execution_cycle_estimate(1_000_000, "update", &selection).expect("estimate")
    }

    fn scenario(transport_mode: &'static str) -> AuditScenario {
        AuditScenario {
            key: "test:test:minimal-valid",
            canister: "test",
            endpoint_or_flow: "test",
            transport_mode,
            subject_kind: "endpoint",
            subject_label: "test",
            arg_class: "minimal-valid",
            caller_class: "anonymous",
            auth_state: "local-test-only",
            replay_state: "n/a",
            cache_state: "n/a",
            topology_state: "standalone-test-ready",
            freshness_model: "fresh-topology-per-scenario",
            notes: "test row",
        }
    }

    fn result(transport_mode: &'static str, sample_origin: &str) -> ScenarioResult {
        ScenarioResult {
            scenario: scenario(transport_mode),
            row: CanonicalPerfRow {
                subject_kind: "endpoint".to_string(),
                subject_label: "test".to_string(),
                count: 1,
                total_local_instructions: 1_000_000,
                avg_local_instructions: 1_000_000,
                scenario_key: "test:test:minimal-valid".to_string(),
                scenario_labels: vec![format!("transport_mode={transport_mode}")],
                principal_scope: Some("anonymous".to_string()),
                sample_origin: sample_origin.to_string(),
                execution_cycle_estimate: None,
            },
            checkpoint_rows: Vec::new(),
        }
    }

    #[test]
    fn estimate_flag_requires_explicit_source() {
        let err = enabled_without_source()
            .validate()
            .expect_err("missing source");

        assert!(matches!(err, EstimateError::MissingEstimateSource));
    }

    #[test]
    fn estimate_flag_parser_accepts_boolean_values() {
        assert!(!parse_env_flag("FLAG", "").expect("empty"));
        assert!(!parse_env_flag("FLAG", "0").expect("zero"));
        assert!(!parse_env_flag("FLAG", "false").expect("false"));
        assert!(parse_env_flag("FLAG", "1").expect("one"));
        assert!(parse_env_flag("FLAG", "true").expect("true"));
    }

    #[test]
    fn estimate_flag_parser_rejects_non_boolean_values() {
        let err = parse_env_flag("FLAG", "yes").expect_err("non-boolean flag");

        assert!(matches!(err, EstimateError::InvalidBooleanFlag { .. }));
        assert_eq!(
            err.to_string(),
            "FLAG must be 0, 1, true, or false, got \"yes\""
        );
    }

    #[test]
    fn estimate_sources_require_estimate_flag() {
        let node_err = EstimateOptions {
            enabled: false,
            node_count: Some(13),
            explicit_cycles_per_billion_instructions: None,
            catalog_canister_principal: None,
            allow_stale_subnet_catalog: false,
            subnet_catalog_stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
            subnet_catalog_stale_after_configured: false,
            catalog_root: PathBuf::new(),
            now_unix_secs: FRESH_NOW_UNIX_SECS,
        }
        .validate()
        .expect_err("node source without estimate flag");

        assert!(matches!(
            node_err,
            EstimateError::EstimateSourceWithoutEstimateFlag
        ));

        let rate_err = EstimateOptions {
            enabled: false,
            node_count: None,
            explicit_cycles_per_billion_instructions: Some(1_000_000_000),
            catalog_canister_principal: None,
            allow_stale_subnet_catalog: false,
            subnet_catalog_stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
            subnet_catalog_stale_after_configured: false,
            catalog_root: PathBuf::new(),
            now_unix_secs: FRESH_NOW_UNIX_SECS,
        }
        .validate()
        .expect_err("rate source without estimate flag");

        assert!(matches!(
            rate_err,
            EstimateError::EstimateSourceWithoutEstimateFlag
        ));

        let catalog_err = EstimateOptions {
            enabled: false,
            node_count: None,
            explicit_cycles_per_billion_instructions: None,
            catalog_canister_principal: Some(CANISTER_A.to_string()),
            allow_stale_subnet_catalog: false,
            subnet_catalog_stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
            subnet_catalog_stale_after_configured: false,
            catalog_root: PathBuf::new(),
            now_unix_secs: FRESH_NOW_UNIX_SECS,
        }
        .validate()
        .expect_err("catalog source without estimate flag");

        assert!(matches!(
            catalog_err,
            EstimateError::EstimateSourceWithoutEstimateFlag
        ));
    }

    #[test]
    fn catalog_stale_controls_require_catalog_source() {
        let mut allow_stale = enabled_without_source();
        allow_stale.explicit_cycles_per_billion_instructions = Some(1_000_000_000);
        allow_stale.allow_stale_subnet_catalog = true;

        let allow_err = allow_stale
            .validate()
            .expect_err("allow-stale without catalog source");

        assert!(matches!(
            allow_err,
            EstimateError::CatalogStaleControlWithoutCatalogSource
        ));

        let mut stale_after = enabled_without_source();
        stale_after.explicit_cycles_per_billion_instructions = Some(1_000_000_000);
        stale_after.subnet_catalog_stale_after_seconds = 60;
        stale_after.subnet_catalog_stale_after_configured = true;

        let stale_after_err = stale_after
            .validate()
            .expect_err("stale-after without catalog source");

        assert!(matches!(
            stale_after_err,
            EstimateError::CatalogStaleControlWithoutCatalogSource
        ));
    }

    #[test]
    fn disabled_estimates_leave_rows_instruction_only() {
        let mut results = vec![result("update", "update")];

        apply_execution_cycle_estimates(&mut results, EstimateOptions::disabled())
            .expect("disabled estimates");

        assert!(results[0].row.execution_cycle_estimate.is_none());
    }

    #[test]
    fn thirteen_node_table_rate_is_pinned() {
        let options = options_with_node_count(13);
        let selection = select_required_rate(&options);

        assert_eq!(
            selection.cycles_per_billion_instructions,
            THIRTEEN_NODE_CYCLES_PER_BILLION
        );
        assert_eq!(selection.subnet_source, "flag");
        assert_eq!(selection.rate_source, "official-ic-cycle-costs-docs");
    }

    #[test]
    fn thirty_four_node_table_rate_is_pinned() {
        let options = options_with_node_count(34);
        let selection = select_required_rate(&options);

        assert_eq!(
            selection.cycles_per_billion_instructions,
            THIRTY_FOUR_NODE_CYCLES_PER_BILLION
        );
        assert_eq!(selection.subnet_source, "flag");
        assert_eq!(selection.rate_source, "official-ic-cycle-costs-docs");
    }

    #[test]
    fn unsupported_node_count_requires_explicit_rate() {
        let err = options_with_node_count(28)
            .validate()
            .expect_err("unsupported table node count");

        assert!(matches!(err, EstimateError::UnsupportedNodeCount(28)));
    }

    #[test]
    fn explicit_rate_wins_over_node_count_table_rate() {
        let options = EstimateOptions {
            enabled: true,
            node_count: Some(34),
            explicit_cycles_per_billion_instructions: Some(123_456_789),
            catalog_canister_principal: None,
            allow_stale_subnet_catalog: false,
            subnet_catalog_stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
            subnet_catalog_stale_after_configured: false,
            catalog_root: PathBuf::new(),
            now_unix_secs: FRESH_NOW_UNIX_SECS,
        };
        let selection = select_required_rate(&options);

        assert_eq!(selection.cycles_per_billion_instructions, 123_456_789);
        assert_eq!(
            selection.node_count_table_rate,
            Some(THIRTY_FOUR_NODE_CYCLES_PER_BILLION)
        );
        assert!(selection.overrode_node_count_table_rate);
        assert_eq!(selection.subnet_source, "explicit-rate");
        assert_eq!(selection.rate_source, "operator-explicit-rate");
    }

    #[test]
    fn estimates_decorate_update_rows_only() {
        let mut results = vec![result("query", "query"), result("update", "update")];

        apply_execution_cycle_estimates(&mut results, options_with_node_count(13))
            .expect("apply estimates");

        assert!(results[0].row.execution_cycle_estimate.is_none());
        assert!(results[1].row.execution_cycle_estimate.is_some());
    }

    #[test]
    fn explicit_rate_allows_unsupported_node_count_metadata() {
        let options = EstimateOptions {
            enabled: true,
            node_count: Some(28),
            explicit_cycles_per_billion_instructions: Some(123),
            catalog_canister_principal: None,
            allow_stale_subnet_catalog: false,
            subnet_catalog_stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
            subnet_catalog_stale_after_configured: false,
            catalog_root: PathBuf::new(),
            now_unix_secs: FRESH_NOW_UNIX_SECS,
        };
        let selection = select_required_rate(&options);

        assert_eq!(selection.cycles_per_billion_instructions, 123);
        assert_eq!(selection.node_count_table_rate, None);
        assert!(!selection.overrode_node_count_table_rate);
    }

    #[test]
    fn estimate_math_uses_ceil_integer_arithmetic() {
        assert_eq!(
            estimate_instruction_cycles(1_000_000, THIRTEEN_NODE_CYCLES_PER_BILLION)
                .expect("estimate"),
            1_000_000
        );
        assert_eq!(estimate_instruction_cycles(1, 1).expect("ceil"), 1);
        assert_eq!(estimate_instruction_cycles(0, 1).expect("zero"), 0);
    }

    #[test]
    fn estimate_math_reports_overflow() {
        let err = estimate_instruction_cycles(u128::MAX, 2).expect_err("overflow");

        assert!(matches!(err, EstimateError::Overflow));
    }

    #[test]
    fn estimate_object_serializes_required_shape_without_catalog_fields() {
        let options = options_with_node_count(13);
        let estimate = estimate_with_options(&options);
        let value = serde_json::to_value(estimate).expect("serialize estimate");

        assert_eq!(value["estimate_schema_version"], ESTIMATE_SCHEMA_VERSION);
        assert_eq!(
            value["counter_id"].as_u64(),
            Some(u64::from(PERF_COUNTER_ID))
        );
        assert_eq!(value["sample_origin"], "update");
        assert_eq!(value["kind"], "per_instruction_component_only");
        assert_eq!(
            value["charge_model"],
            "hypothetical_update_execution_component"
        );
        assert_eq!(value["estimated_instruction_cycles"], "1000000");
        assert_eq!(
            value["cycles_per_billion_instructions"],
            THIRTEEN_NODE_CYCLES_PER_BILLION.to_string()
        );
        assert_eq!(value["subnet_node_count"].as_u64(), Some(13));
        assert_eq!(value["subnet_source"], "flag");
        assert_eq!(
            value["source_meaning"],
            "operator_supplied_pricing_assumption"
        );
        assert_eq!(value["formula_version"], FORMULA_VERSION);
        assert_eq!(value["rate_source"], "official-ic-cycle-costs-docs");
        assert_eq!(
            value["overrode_node_count_table_rate"].as_bool(),
            Some(false)
        );
        assert_eq!(
            value["node_count_table_rate"],
            THIRTEEN_NODE_CYCLES_PER_BILLION.to_string()
        );
        assert_no_catalog_fields(&value);
    }

    #[test]
    fn estimate_object_serializes_explicit_rate_override_labels() {
        let options = EstimateOptions {
            enabled: true,
            node_count: Some(34),
            explicit_cycles_per_billion_instructions: Some(123_456_789),
            catalog_canister_principal: None,
            allow_stale_subnet_catalog: false,
            subnet_catalog_stale_after_seconds: DEFAULT_STALE_AFTER_SECONDS,
            subnet_catalog_stale_after_configured: false,
            catalog_root: PathBuf::new(),
            now_unix_secs: FRESH_NOW_UNIX_SECS,
        };
        let estimate = estimate_with_options(&options);
        let value = serde_json::to_value(estimate).expect("serialize estimate");

        assert_eq!(value["subnet_node_count"].as_u64(), Some(34));
        assert_eq!(value["subnet_source"], "explicit-rate");
        assert_eq!(value["rate_source"], "operator-explicit-rate");
        assert_eq!(value["cycles_per_billion_instructions"], "123456789");
        assert_eq!(
            value["overrode_node_count_table_rate"].as_bool(),
            Some(true)
        );
        assert_eq!(
            value["node_count_table_rate"],
            THIRTY_FOUR_NODE_CYCLES_PER_BILLION.to_string()
        );
        assert_no_catalog_fields(&value);
    }

    #[test]
    fn estimate_object_serializes_pinned_omitted_costs() {
        let options = options_with_node_count(13);
        let estimate = estimate_with_options(&options);
        let value = serde_json::to_value(estimate).expect("serialize estimate");
        let omitted = value["omitted_costs"]
            .as_array()
            .expect("omitted costs array");

        assert_eq!(omitted.len(), OMITTED_COSTS.len());
        for expected in OMITTED_COSTS {
            assert!(
                omitted
                    .iter()
                    .any(|actual| actual.as_str() == Some(*expected)),
                "omitted cost should be serialized: {expected}"
            );
        }
    }

    #[test]
    fn catalog_estimate_uses_any_positive_application_node_count() {
        let root = temp_root("catalog-31");
        write_catalog(
            &root,
            fixture_catalog(Some(31), SubnetKind::Application, FRESH_FETCHED_AT),
        );
        let options = catalog_options(&root);

        let selection = select_required_rate(&options);

        assert_eq!(
            selection.cycles_per_billion_instructions,
            catalog_cycles_per_billion(31).expect("31 node rate")
        );
        assert_eq!(selection.subnet_node_count, Some(31));
        assert_eq!(selection.subnet_source, "nns-registry-cache");
        assert_eq!(selection.source_meaning, "resolved_from_nns_registry_cache");
        assert_eq!(selection.rate_source, "nns-registry-cache");
        assert_eq!(selection.formula_version, "base_13_node_linear_v1");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn catalog_thirty_four_node_rate_uses_ceil_formula() {
        let root = temp_root("catalog-34");
        write_catalog(
            &root,
            fixture_catalog(Some(34), SubnetKind::Application, FRESH_FETCHED_AT),
        );
        let options = catalog_options(&root);

        let selection = select_required_rate(&options);

        assert_eq!(THIRTY_FOUR_NODE_CYCLES_PER_BILLION, 2_615_384_615);
        assert_eq!(selection.cycles_per_billion_instructions, 2_615_384_616);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn catalog_estimate_serializes_registry_provenance() {
        let root = temp_root("catalog-provenance");
        write_catalog(
            &root,
            fixture_catalog(Some(31), SubnetKind::Application, FRESH_FETCHED_AT),
        );
        let options = catalog_options(&root);
        let estimate = estimate_with_options(&options);
        let value = serde_json::to_value(estimate).expect("serialize estimate");

        assert_eq!(value["subnet_node_count"].as_u64(), Some(31));
        assert_eq!(value["subnet_source"], "nns-registry-cache");
        assert_eq!(value["source_meaning"], "resolved_from_nns_registry_cache");
        assert_eq!(value["formula_version"], "base_13_node_linear_v1");
        assert_eq!(value["rate_source"], "nns-registry-cache");
        assert_eq!(value["registry_canister_id"], MAINNET_REGISTRY_CANISTER_ID);
        assert_eq!(value["registry_version"].as_u64(), Some(123_456));
        assert_eq!(value["subnet_principal"], SUBNET_A);
        assert_eq!(value["subnet_kind"], "application");
        assert_eq!(value["subnet_kind_source"], "registry");
        assert_eq!(value["subnet_specialization"], "fiduciary");
        assert_eq!(value["subnet_specialization_source"], "curated");
        assert_eq!(value["geographic_scope"], "global");
        assert_eq!(value["geographic_scope_source"], "curated");
        assert_eq!(
            value["catalog_schema_version"].as_u64(),
            Some(u64::from(CATALOG_SCHEMA_VERSION))
        );
        assert_eq!(value["catalog_stale"].as_bool(), Some(false));
        assert_eq!(value["resolver_backend"], "local-nns-subnet-catalog");
        assert_eq!(value["matched_canister_principal"], CANISTER_A);
        assert_eq!(
            value["matched_routing_range"]["start_canister_id"],
            CANISTER_A
        );
        assert_eq!(
            value["matched_routing_range"]["end_canister_id"],
            CANISTER_A
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stale_catalog_omits_estimates_by_default() {
        let root = temp_root("catalog-stale-default");
        write_catalog(
            &root,
            fixture_catalog(Some(13), SubnetKind::Application, FRESH_FETCHED_AT),
        );
        let mut options = catalog_options(&root);
        options.now_unix_secs = FRESH_NOW_UNIX_SECS + DEFAULT_STALE_AFTER_SECONDS + 1;
        let mut results = vec![result("update", "update")];

        apply_execution_cycle_estimates(&mut results, options).expect("stale catalog");

        assert!(results[0].row.execution_cycle_estimate.is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stale_catalog_can_be_used_when_allowed() {
        let root = temp_root("catalog-stale-allowed");
        write_catalog(
            &root,
            fixture_catalog(Some(13), SubnetKind::Application, FRESH_FETCHED_AT),
        );
        let mut options = catalog_options(&root);
        options.now_unix_secs = FRESH_NOW_UNIX_SECS + DEFAULT_STALE_AFTER_SECONDS + 1;
        options.allow_stale_subnet_catalog = true;
        let mut results = vec![result("update", "update")];

        apply_execution_cycle_estimates(&mut results, options).expect("stale catalog allowed");

        let estimate = results[0]
            .row
            .execution_cycle_estimate
            .as_ref()
            .expect("estimate");
        assert_eq!(estimate.catalog_stale, Some(true));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn system_subnet_omits_catalog_estimate() {
        let root = temp_root("catalog-system");
        write_catalog(
            &root,
            fixture_catalog(Some(13), SubnetKind::System, FRESH_FETCHED_AT),
        );
        let mut results = vec![result("update", "update")];

        apply_execution_cycle_estimates(&mut results, catalog_options(&root))
            .expect("system catalog");

        assert!(results[0].row.execution_cycle_estimate.is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_catalog_node_count_omits_catalog_estimate() {
        let root = temp_root("catalog-missing-node-count");
        write_catalog(
            &root,
            fixture_catalog(None, SubnetKind::Application, FRESH_FETCHED_AT),
        );
        let mut results = vec![result("update", "update")];

        apply_execution_cycle_estimates(&mut results, catalog_options(&root))
            .expect("missing node count");

        assert!(results[0].row.execution_cycle_estimate.is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_catalog_cache_omits_catalog_estimate() {
        let root = temp_root("catalog-missing");
        let mut results = vec![result("update", "update")];

        apply_execution_cycle_estimates(&mut results, catalog_options(&root))
            .expect("missing catalog");

        assert!(results[0].row.execution_cycle_estimate.is_none());
    }

    #[test]
    fn explicit_rate_wins_over_catalog_without_reading_cache() {
        let root = temp_root("catalog-explicit-rate");
        let mut options = catalog_options(&root);
        options.explicit_cycles_per_billion_instructions = Some(123_456_789);
        let estimate = estimate_with_options(&options);
        let value = serde_json::to_value(estimate).expect("serialize estimate");

        assert_eq!(value["cycles_per_billion_instructions"], "123456789");
        assert_eq!(value["subnet_source"], "explicit-rate");
        assert_no_catalog_fields(&value);
    }

    #[test]
    fn explicit_node_count_wins_over_catalog_without_reading_cache() {
        let root = temp_root("catalog-explicit-node-count");
        let mut options = catalog_options(&root);
        options.node_count = Some(13);
        let estimate = estimate_with_options(&options);
        let value = serde_json::to_value(estimate).expect("serialize estimate");

        assert_eq!(
            value["cycles_per_billion_instructions"],
            THIRTEEN_NODE_CYCLES_PER_BILLION.to_string()
        );
        assert_eq!(value["subnet_source"], "flag");
        assert_no_catalog_fields(&value);
    }

    fn assert_no_catalog_fields(value: &Value) {
        let text = serde_json::to_string(value).expect("serialize");
        for forbidden in [
            "subnet_principal",
            "registry_version",
            "catalog_schema_version",
            "resolver_backend",
            "routing_range",
            "geographic_scope",
        ] {
            assert!(
                !text.contains(forbidden),
                "non-catalog estimate artifacts must not contain {forbidden}"
            );
        }
    }

    fn temp_root(name: &str) -> PathBuf {
        let mut root = env::temp_dir();
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        root.push(format!(
            "canic-instruction-estimates-{name}-{}-{nonce}",
            process::id()
        ));
        root
    }

    fn write_catalog(root: &Path, catalog: SubnetCatalog) {
        let path = subnet_catalog_path(root, MAINNET_NETWORK);
        fs::create_dir_all(path.parent().expect("catalog path parent"))
            .expect("create catalog dir");
        let json = serde_json::to_string_pretty(&catalog).expect("serialize catalog");
        fs::write(path, json).expect("write catalog");
    }

    fn fixture_catalog(
        node_count: Option<u32>,
        subnet_kind: SubnetKind,
        fetched_at: &str,
    ) -> SubnetCatalog {
        SubnetCatalog {
            catalog_schema_version: CATALOG_SCHEMA_VERSION,
            network: MAINNET_NETWORK.to_string(),
            registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
            registry_version: 123_456,
            fetched_at: fetched_at.to_string(),
            fetched_by: "fixture".to_string(),
            source_endpoint: "https://icp-api.io".to_string(),
            resolver_backend: "local-nns-subnet-catalog".to_string(),
            subnets: vec![SubnetInfo {
                subnet_principal: SUBNET_A.to_string(),
                subnet_kind,
                subnet_kind_source: ClassificationSource::Registry,
                subnet_specialization: SubnetSpecialization::Fiduciary,
                subnet_specialization_source: ClassificationSource::Curated,
                geographic_scope: GeographicScope::Global,
                geographic_scope_source: ClassificationSource::Curated,
                subnet_label: "fiduciary".to_string(),
                subnet_label_source: ClassificationSource::Curated,
                node_count,
                charges_apply_by_default: subnet_kind == SubnetKind::Application,
            }],
            routing_ranges: vec![RoutingRange {
                start_canister_id: CANISTER_A.to_string(),
                end_canister_id: CANISTER_A.to_string(),
                subnet_principal: SUBNET_A.to_string(),
            }],
        }
    }
}

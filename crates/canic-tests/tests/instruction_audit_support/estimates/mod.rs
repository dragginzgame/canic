use super::*;
use std::env;

pub(super) const ESTIMATE_SCHEMA_VERSION: u8 = 1;
pub(super) const FORMULA_VERSION: &str = "canic-0.59-ic-cycle-costs-v1";
const ESTIMATE_KIND_PER_INSTRUCTION_COMPONENT: &str = "per_instruction_component_only";
const CHARGE_MODEL_UPDATE_EXECUTION_COMPONENT: &str = "hypothetical_update_execution_component";
const SUBNET_SOURCE_FLAG: &str = "flag";
const SUBNET_SOURCE_EXPLICIT_RATE: &str = "explicit-rate";
const SOURCE_MEANING_OPERATOR_SUPPLIED: &str = "operator_supplied_pricing_assumption";
const RATE_SOURCE_OFFICIAL_DOCS: &str = "official-ic-cycle-costs-docs";
const RATE_SOURCE_OPERATOR_EXPLICIT: &str = "operator-explicit-rate";
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

///
/// EstimateError
///

#[derive(Debug, Eq, PartialEq)]
pub(super) enum EstimateError {
    MissingEstimateSource,
    EstimateSourceWithoutEstimateFlag,
    UnsupportedNodeCount(u16),
    InvalidBooleanFlag { field: &'static str, value: String },
    InvalidPositiveInteger { field: &'static str, value: String },
    Overflow,
}

impl std::fmt::Display for EstimateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEstimateSource => formatter.write_str(
                "--estimate-execution-cycles requires --estimate-node-count or --cycles-per-billion-instructions",
            ),
            Self::EstimateSourceWithoutEstimateFlag => formatter.write_str(
                "--estimate-node-count and --cycles-per-billion-instructions require --estimate-execution-cycles",
            ),
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
            Self::Overflow => formatter.write_str("instruction cycle estimate overflowed u128"),
        }
    }
}

impl std::error::Error for EstimateError {}

///
/// EstimateOptions
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct EstimateOptions {
    pub enabled: bool,
    pub node_count: Option<u16>,
    pub explicit_cycles_per_billion_instructions: Option<u128>,
}

///
/// RateSelection
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RateSelection {
    cycles_per_billion_instructions: u128,
    node_count_table_rate: Option<u128>,
    overrode_node_count_table_rate: bool,
    subnet_source: &'static str,
    rate_source: &'static str,
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
    pub subnet_node_count: Option<u16>,
    pub subnet_source: &'static str,
    pub source_meaning: &'static str,
    pub formula_version: &'static str,
    pub rate_source: &'static str,
    pub overrode_node_count_table_rate: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_count_table_rate: Option<String>,
    pub omitted_costs: &'static [&'static str],
}

impl EstimateOptions {
    pub(super) const fn disabled() -> Self {
        Self {
            enabled: false,
            node_count: None,
            explicit_cycles_per_billion_instructions: None,
        }
    }

    fn validate(self) -> Result<Self, EstimateError> {
        if !self.enabled {
            if self.node_count.is_some() || self.explicit_cycles_per_billion_instructions.is_some()
            {
                return Err(EstimateError::EstimateSourceWithoutEstimateFlag);
            }

            return Ok(Self::disabled());
        }

        if self.node_count.is_none() && self.explicit_cycles_per_billion_instructions.is_none() {
            return Err(EstimateError::MissingEstimateSource);
        }

        let _selection = select_rate(self)?;
        Ok(self)
    }
}

pub(super) fn estimate_options_from_env() -> Result<EstimateOptions, EstimateError> {
    let enabled = env_flag(ENV_ESTIMATE_EXECUTION_CYCLES)?;
    let node_count = optional_positive_u16_env(ENV_ESTIMATE_NODE_COUNT)?;
    let explicit_cycles_per_billion_instructions =
        optional_positive_u128_env(ENV_CYCLES_PER_BILLION_INSTRUCTIONS)?;

    EstimateOptions {
        enabled,
        node_count,
        explicit_cycles_per_billion_instructions,
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

    for result in results {
        if result.row.sample_origin != "update" {
            continue;
        }

        result.row.execution_cycle_estimate = Some(execution_cycle_estimate(
            result.row.total_local_instructions,
            &result.row.sample_origin,
            options,
        )?);
    }

    Ok(())
}

fn execution_cycle_estimate(
    local_instructions: u64,
    sample_origin: &str,
    options: EstimateOptions,
) -> Result<ExecutionCycleEstimate, EstimateError> {
    let selection = select_rate(options)?;
    let estimated_instruction_cycles = estimate_instruction_cycles(
        u128::from(local_instructions),
        selection.cycles_per_billion_instructions,
    )?;

    Ok(ExecutionCycleEstimate {
        estimate_schema_version: ESTIMATE_SCHEMA_VERSION,
        kind: ESTIMATE_KIND_PER_INSTRUCTION_COMPONENT,
        charge_model: CHARGE_MODEL_UPDATE_EXECUTION_COMPONENT,
        local_instructions,
        counter_id: PERF_COUNTER_ID,
        sample_origin: sample_origin.to_string(),
        estimated_instruction_cycles: estimated_instruction_cycles.to_string(),
        cycles_per_billion_instructions: selection.cycles_per_billion_instructions.to_string(),
        subnet_node_count: options.node_count,
        subnet_source: selection.subnet_source,
        source_meaning: SOURCE_MEANING_OPERATOR_SUPPLIED,
        formula_version: FORMULA_VERSION,
        rate_source: selection.rate_source,
        overrode_node_count_table_rate: selection.overrode_node_count_table_rate,
        node_count_table_rate: selection.node_count_table_rate.map(|rate| rate.to_string()),
        omitted_costs: OMITTED_COSTS,
    })
}

fn select_rate(options: EstimateOptions) -> Result<RateSelection, EstimateError> {
    let node_count_table_rate = options.node_count.and_then(node_count_table_rate);

    if let Some(explicit_rate) = options.explicit_cycles_per_billion_instructions {
        return Ok(RateSelection {
            cycles_per_billion_instructions: explicit_rate,
            node_count_table_rate,
            overrode_node_count_table_rate: node_count_table_rate.is_some(),
            subnet_source: SUBNET_SOURCE_EXPLICIT_RATE,
            rate_source: RATE_SOURCE_OPERATOR_EXPLICIT,
        });
    }

    let Some(node_count) = options.node_count else {
        return Err(EstimateError::MissingEstimateSource);
    };
    let Some(rate) = node_count_table_rate else {
        return Err(EstimateError::UnsupportedNodeCount(node_count));
    };

    Ok(RateSelection {
        cycles_per_billion_instructions: rate,
        node_count_table_rate: Some(rate),
        overrode_node_count_table_rate: false,
        subnet_source: SUBNET_SOURCE_FLAG,
        rate_source: RATE_SOURCE_OFFICIAL_DOCS,
    })
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
    use serde_json::Value;

    fn options_with_node_count(node_count: u16) -> EstimateOptions {
        EstimateOptions {
            enabled: true,
            node_count: Some(node_count),
            explicit_cycles_per_billion_instructions: None,
        }
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
        let err = EstimateOptions {
            enabled: true,
            node_count: None,
            explicit_cycles_per_billion_instructions: None,
        }
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
        }
        .validate()
        .expect_err("rate source without estimate flag");

        assert!(matches!(
            rate_err,
            EstimateError::EstimateSourceWithoutEstimateFlag
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
        let selection = select_rate(options_with_node_count(13)).expect("13 node rate");

        assert_eq!(
            selection.cycles_per_billion_instructions,
            THIRTEEN_NODE_CYCLES_PER_BILLION
        );
        assert_eq!(selection.subnet_source, "flag");
        assert_eq!(selection.rate_source, "official-ic-cycle-costs-docs");
    }

    #[test]
    fn thirty_four_node_table_rate_is_pinned() {
        let selection = select_rate(options_with_node_count(34)).expect("34 node rate");

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
        };
        let selection = select_rate(options).expect("explicit rate");

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
        };
        let selection = select_rate(options).expect("explicit rate");

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
        let estimate = execution_cycle_estimate(1_000_000, "update", options_with_node_count(13))
            .expect("estimate");
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
        };
        let estimate = execution_cycle_estimate(1_000_000, "update", options).expect("estimate");
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
        let estimate = execution_cycle_estimate(1_000_000, "update", options_with_node_count(13))
            .expect("estimate");
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
                "0.59 estimate artifacts must not contain {forbidden}"
            );
        }
    }
}

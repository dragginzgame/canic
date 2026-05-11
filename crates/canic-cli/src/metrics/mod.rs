use crate::{
    args::{
        default_icp, flag_arg, internal_icp_arg, internal_network_arg, local_network,
        parse_matches, path_option, print_help_or_version, string_option, value_arg,
    },
    output,
    response_parse::{
        RECORD_MARKER, candid_record_blocks, find_field, parse_json_u64, parse_json_u128,
        parse_u64_digits, parse_u128_digits, quoted_strings, text_after,
    },
    version_text,
};
use canic_backup::discovery::{DiscoveryError, RegistryEntry, parse_registry_entries};
use canic_host::{
    icp::{IcpCli, IcpCommandError},
    install_root::read_named_fleet_install_state,
    replica_query,
    table::{ColumnAlign, render_table},
};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{ffi::OsString, path::PathBuf, sync::Arc, thread};
use thiserror::Error as ThisError;

const DEFAULT_LIMIT: u64 = 1_000;
pub const CANIC_METRICS_METHOD: &str = "canic_metrics";

///
/// MetricsCommandError
///

#[derive(Debug, ThisError)]
pub enum MetricsCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(
        "fleet {fleet} is not installed on network {network}; run `canic install {fleet}` before querying metrics"
    )]
    NoInstalledFleet { network: String, fleet: String },

    #[error("failed to read canic fleet state: {0}")]
    InstallState(String),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error(
        "invalid metrics kind {0}; use core, placement, platform, runtime, security, or storage"
    )]
    InvalidKind(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
}

///
/// MetricsKind
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricsKind {
    Core,
    Placement,
    Platform,
    Runtime,
    Security,
    Storage,
}

impl MetricsKind {
    fn parse(value: &str) -> Result<Self, MetricsCommandError> {
        match value {
            "core" => Ok(Self::Core),
            "placement" => Ok(Self::Placement),
            "platform" => Ok(Self::Platform),
            "runtime" => Ok(Self::Runtime),
            "security" => Ok(Self::Security),
            "storage" => Ok(Self::Storage),
            _ => Err(MetricsCommandError::InvalidKind(value.to_string())),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Placement => "placement",
            Self::Platform => "platform",
            Self::Runtime => "runtime",
            Self::Security => "security",
            Self::Storage => "storage",
        }
    }

    const fn candid_variant(self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Placement => "Placement",
            Self::Platform => "Platform",
            Self::Runtime => "Runtime",
            Self::Security => "Security",
            Self::Storage => "Storage",
        }
    }
}

///
/// MetricsOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetricsOptions {
    pub fleet: String,
    pub kind: MetricsKind,
    pub role: Option<String>,
    pub canister: Option<String>,
    pub nonzero: bool,
    pub limit: u64,
    pub json: bool,
    pub out: Option<PathBuf>,
    pub network: String,
    pub icp: String,
}

///
/// MetricsReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MetricsReport {
    pub fleet: String,
    pub network: String,
    pub kind: MetricsKind,
    pub canisters: Vec<MetricsCanisterReport>,
}

///
/// MetricsCanisterReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MetricsCanisterReport {
    pub role: String,
    pub canister_id: String,
    pub status: String,
    pub entries: Vec<MetricEntry>,
    pub error: Option<String>,
}

///
/// MetricEntry
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MetricEntry {
    pub labels: Vec<String>,
    pub principal: Option<String>,
    pub value: MetricValue,
}

///
/// MetricValue
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MetricValue {
    Count { count: u64 },
    CountAndU64 { count: u64, value_u64: u64 },
    U128 { value: u128 },
}

impl MetricValue {
    const fn is_zero(&self) -> bool {
        match self {
            Self::Count { count } => *count == 0,
            Self::CountAndU64 { count, value_u64 } => *count == 0 && *value_u64 == 0,
            Self::U128 { value } => *value == 0,
        }
    }
}

pub fn run<I>(args: I) -> Result<(), MetricsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = MetricsOptions::parse(args)?;
    let report = metrics_report(&options)?;
    write_metrics_report(&options, &report)
}

impl MetricsOptions {
    pub fn parse<I>(args: I) -> Result<Self, MetricsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(metrics_command(), args)
            .map_err(|_| MetricsCommandError::Usage(usage()))?;
        let kind = string_option(&matches, "kind")
            .map(|value| MetricsKind::parse(&value))
            .transpose()?
            .unwrap_or(MetricsKind::Core);
        let limit = string_option(&matches, "limit")
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|limit| *limit > 0)
            .unwrap_or(DEFAULT_LIMIT);

        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
            kind,
            role: string_option(&matches, "role"),
            canister: string_option(&matches, "canister"),
            nonzero: matches.get_flag("nonzero"),
            limit,
            json: matches.get_flag("json"),
            out: path_option(&matches, "out"),
            network: string_option(&matches, "network").unwrap_or_else(local_network),
            icp: string_option(&matches, "icp").unwrap_or_else(default_icp),
        })
    }
}

pub fn metrics_report(options: &MetricsOptions) -> Result<MetricsReport, MetricsCommandError> {
    let registry = load_registry(options)?;
    let canisters = collect_metrics_reports(options, &registry);

    Ok(MetricsReport {
        fleet: options.fleet.clone(),
        network: options.network.clone(),
        kind: options.kind,
        canisters,
    })
}

fn load_registry(options: &MetricsOptions) -> Result<Vec<RegistryEntry>, MetricsCommandError> {
    let state = read_named_fleet_install_state(&options.network, &options.fleet)
        .map_err(|err| MetricsCommandError::InstallState(err.to_string()))?
        .ok_or_else(|| MetricsCommandError::NoInstalledFleet {
            network: options.network.clone(),
            fleet: options.fleet.clone(),
        })?;
    let registry_json = call_subnet_registry(options, &state.root_canister_id)?;
    let mut registry = parse_registry_entries(&registry_json)?;
    registry.retain(|entry| matches_metrics_filter(options, entry));
    Ok(registry)
}

fn matches_metrics_filter(options: &MetricsOptions, entry: &RegistryEntry) -> bool {
    if let Some(role) = &options.role
        && entry.role.as_deref() != Some(role.as_str())
    {
        return false;
    }
    if let Some(canister) = &options.canister
        && entry.pid != *canister
    {
        return false;
    }
    true
}

fn collect_metrics_reports(
    options: &MetricsOptions,
    registry: &[RegistryEntry],
) -> Vec<MetricsCanisterReport> {
    let query = Arc::new(options.clone());
    let mut handles = Vec::new();
    for entry in registry {
        let entry = entry.clone();
        let query = Arc::clone(&query);
        handles.push(thread::spawn(move || {
            metrics_canister_report(&query, &entry)
        }));
    }

    handles
        .into_iter()
        .filter_map(|handle| handle.join().ok())
        .collect()
}

fn metrics_canister_report(
    options: &MetricsOptions,
    entry: &RegistryEntry,
) -> MetricsCanisterReport {
    match query_metrics(options, &entry.pid) {
        Ok(mut entries) => {
            if options.nonzero {
                entries.retain(|entry| !entry.value.is_zero());
            }
            MetricsCanisterReport {
                role: entry.role.clone().unwrap_or_else(|| "-".to_string()),
                canister_id: entry.pid.clone(),
                status: "ok".to_string(),
                entries,
                error: None,
            }
        }
        Err(error) => metrics_error_report(entry, &error),
    }
}

fn metrics_error_report(entry: &RegistryEntry, error: &str) -> MetricsCanisterReport {
    let (status, error) = if error.contains("has no query method 'canic_metrics'") {
        ("unavailable", "canic_metrics unavailable")
    } else {
        ("error", error.lines().next().unwrap_or(error))
    };

    MetricsCanisterReport {
        role: entry.role.clone().unwrap_or_else(|| "-".to_string()),
        canister_id: entry.pid.clone(),
        status: status.to_string(),
        entries: Vec::new(),
        error: Some(error.to_string()),
    }
}

fn query_metrics(options: &MetricsOptions, canister_id: &str) -> Result<Vec<MetricEntry>, String> {
    let arg = format!(
        "(variant {{ {} }}, record {{ offset = 0 : nat64; limit = {} : nat64 }})",
        options.kind.candid_variant(),
        options.limit
    );
    let output = IcpCli::new(&options.icp, None, Some(options.network.clone()))
        .canister_query_arg_output(canister_id, CANIC_METRICS_METHOD, &arg, Some("json"))
        .map_err(|err| err.to_string())?;

    parse_metrics_page(&output).ok_or_else(|| "could not parse canic_metrics response".to_string())
}

pub fn parse_metrics_page(output: &str) -> Option<Vec<MetricEntry>> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    if let Some(entries) = parse_metrics_page_json(&value) {
        return Some(entries);
    }
    find_field(&value, "response_candid")
        .and_then(serde_json::Value::as_str)
        .and_then(parse_metrics_page_text)
}

fn parse_metrics_page_json(value: &serde_json::Value) -> Option<Vec<MetricEntry>> {
    Some(
        find_field(value, "entries")?
            .as_array()?
            .iter()
            .filter_map(parse_metric_entry_json)
            .collect::<Vec<_>>(),
    )
}

fn parse_metric_entry_json(value: &serde_json::Value) -> Option<MetricEntry> {
    Some(MetricEntry {
        labels: find_field(value, "labels")?
            .as_array()?
            .iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect(),
        principal: find_field(value, "principal").and_then(parse_principal_json),
        value: find_field(value, "value").and_then(parse_metric_value_json)?,
    })
}

fn parse_metrics_page_text(output: &str) -> Option<Vec<MetricEntry>> {
    let mut entries = Vec::new();
    for chunk in candid_record_blocks(output) {
        if !(chunk[RECORD_MARKER.len()..]
            .trim_start()
            .starts_with("\"principal\" =")
            && chunk.contains("labels = vec")
            && chunk.contains("value = variant"))
        {
            continue;
        }
        entries.push(MetricEntry {
            labels: parse_candid_labels(chunk)?,
            principal: parse_candid_principal(chunk),
            value: parse_candid_metric_value(chunk)?,
        });
    }
    Some(entries)
}

fn parse_candid_labels(chunk: &str) -> Option<Vec<String>> {
    let (_, after_field) = chunk.split_once("labels = vec")?;
    let (_, after_open) = after_field.split_once('{')?;
    let (labels, _) = after_open.split_once("};")?;
    Some(quoted_strings(labels))
}

fn parse_candid_principal(chunk: &str) -> Option<String> {
    let (_, after_field) = chunk.split_once("\"principal\" =")?;
    let value = after_field.trim_start();
    if value.starts_with("null") {
        return None;
    }
    quoted_strings(value).into_iter().next()
}

fn parse_candid_metric_value(chunk: &str) -> Option<MetricValue> {
    if let Some(value) = text_after(chunk, "Count =").and_then(parse_u64_digits) {
        return Some(MetricValue::Count { count: value });
    }
    if let Some(value) = text_after(chunk, "U128 =").and_then(parse_u128_digits) {
        return Some(MetricValue::U128 { value });
    }
    if chunk.contains("CountAndU64") {
        let count = text_after(chunk, "count =").and_then(parse_u64_digits)?;
        let value_u64 = text_after(chunk, "value_u64 =").and_then(parse_u64_digits)?;
        return Some(MetricValue::CountAndU64 { count, value_u64 });
    }
    None
}

fn parse_principal_json(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Object(map) => map.values().find_map(parse_principal_json),
        serde_json::Value::Array(values) => values.iter().find_map(parse_principal_json),
        _ => None,
    }
}

fn parse_metric_value_json(value: &serde_json::Value) -> Option<MetricValue> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(value) = map.get("Count").and_then(parse_json_u64) {
                return Some(MetricValue::Count { count: value });
            }
            if let Some(value) = map.get("U128").and_then(parse_json_u128) {
                return Some(MetricValue::U128 { value });
            }
            if let Some(value) = map.get("CountAndU64") {
                let count = find_field(value, "count").and_then(parse_json_u64)?;
                let value_u64 = find_field(value, "value_u64").and_then(parse_json_u64)?;
                return Some(MetricValue::CountAndU64 { count, value_u64 });
            }
            if let (Some(count), Some(value_u64)) = (
                map.get("count").and_then(parse_json_u64),
                map.get("value_u64").and_then(parse_json_u64),
            ) {
                return Some(MetricValue::CountAndU64 { count, value_u64 });
            }
            map.values().find_map(parse_metric_value_json)
        }
        serde_json::Value::Array(values) => values.iter().find_map(parse_metric_value_json),
        _ => None,
    }
}

fn write_metrics_report(
    options: &MetricsOptions,
    report: &MetricsReport,
) -> Result<(), MetricsCommandError> {
    if options.json {
        return output::write_pretty_json::<_, MetricsCommandError>(options.out.as_ref(), report);
    }

    output::write_text::<MetricsCommandError>(options.out.as_ref(), &render_metrics_report(report))
}

fn render_metrics_report(report: &MetricsReport) -> String {
    let mut rows = Vec::new();
    for canister in &report.canisters {
        if canister.entries.is_empty() {
            rows.push([
                canister.role.clone(),
                canister.canister_id.clone(),
                report.kind.as_str().to_string(),
                canister.status.clone(),
                canister.error.clone().unwrap_or_else(|| "-".to_string()),
                "-".to_string(),
                "-".to_string(),
            ]);
            continue;
        }

        for entry in &canister.entries {
            rows.push([
                canister.role.clone(),
                canister.canister_id.clone(),
                report.kind.as_str().to_string(),
                canister.status.clone(),
                entry.labels.join("/"),
                entry.principal.clone().unwrap_or_else(|| "-".to_string()),
                metric_value_label(&entry.value),
            ]);
        }
    }

    [
        format!(
            "Fleet: {} (network {}, metrics {})",
            report.fleet,
            report.network,
            report.kind.as_str()
        ),
        String::new(),
        render_table(
            &[
                "ROLE",
                "CANISTER_ID",
                "KIND",
                "STATUS",
                "LABELS",
                "PRINCIPAL",
                "VALUE",
            ],
            &rows,
            &[
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Right,
            ],
        ),
    ]
    .join("\n")
}

fn metric_value_label(value: &MetricValue) -> String {
    match value {
        MetricValue::Count { count } => count.to_string(),
        MetricValue::CountAndU64 { count, value_u64 } => format!("{count}/{value_u64}"),
        MetricValue::U128 { value } => value.to_string(),
    }
}

fn call_subnet_registry(
    options: &MetricsOptions,
    root: &str,
) -> Result<String, MetricsCommandError> {
    if replica_query::should_use_local_replica_query(Some(&options.network)) {
        return replica_query::query_subnet_registry_json(Some(&options.network), root)
            .map_err(|err| MetricsCommandError::ReplicaQuery(err.to_string()));
    }

    IcpCli::new(&options.icp, None, Some(options.network.clone()))
        .canister_call_output(root, "canic_subnet_registry", Some("json"))
        .map_err(metrics_icp_error)
}

fn metrics_icp_error(error: IcpCommandError) -> MetricsCommandError {
    match error {
        IcpCommandError::Io(err) => MetricsCommandError::Io(err),
        IcpCommandError::Failed { command, stderr } => {
            MetricsCommandError::IcpFailed { command, stderr }
        }
        IcpCommandError::SnapshotIdUnavailable { output } => MetricsCommandError::IcpFailed {
            command: "icp canister snapshot create".to_string(),
            stderr: output,
        },
    }
}

fn usage() -> String {
    let mut command = metrics_command();
    command.render_help().to_string()
}

fn metrics_command() -> ClapCommand {
    ClapCommand::new("metrics")
        .bin_name("canic metrics")
        .about("Query Canic runtime telemetry")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Installed fleet name to inspect"),
        )
        .arg(
            value_arg("kind")
                .long("kind")
                .value_name("kind")
                .help("Metrics tier to query; defaults to core"),
        )
        .arg(
            value_arg("role")
                .long("role")
                .value_name("role")
                .help("Only query one registry role"),
        )
        .arg(
            value_arg("canister")
                .long("canister")
                .value_name("id")
                .help("Only query one canister principal"),
        )
        .arg(
            value_arg("limit")
                .long("limit")
                .value_name("entries")
                .help("Maximum metric rows to fetch per canister; defaults to 1000"),
        )
        .arg(flag_arg("nonzero").long("nonzero"))
        .arg(flag_arg("json").long("json"))
        .arg(value_arg("out").long("out").value_name("file"))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure the public kind selector maps to Candid variant names.
    #[test]
    fn parses_metric_kind_selectors() {
        assert_eq!(MetricsKind::parse("core").expect("core"), MetricsKind::Core);
        assert_eq!(
            MetricsKind::parse("security")
                .expect("security")
                .candid_variant(),
            "Security"
        );
        assert!(matches!(
            MetricsKind::parse("cycles"),
            Err(MetricsCommandError::InvalidKind(_))
        ));
    }

    // Ensure named JSON metric pages parse into the CLI row shape.
    #[test]
    fn parses_metrics_json_page() {
        let entries = parse_metrics_page(
            r#"{"Ok":{"entries":[{"labels":["lifecycle","init","started"],"principal":null,"value":{"Count":2}},{"labels":["cycles_funding","minted"],"principal":"aaaaa-aa","value":{"U128":"1000"}},{"labels":["timer","tick"],"principal":null,"value":{"CountAndU64":{"count":3,"value_u64":12}}}],"total":3}}"#,
        )
        .expect("parse metrics page");

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].labels, ["lifecycle", "init", "started"]);
        assert_eq!(entries[0].value, MetricValue::Count { count: 2 });
        assert_eq!(entries[1].principal.as_deref(), Some("aaaaa-aa"));
        assert_eq!(entries[1].value, MetricValue::U128 { value: 1_000 });
        assert_eq!(
            entries[2].value,
            MetricValue::CountAndU64 {
                count: 3,
                value_u64: 12
            }
        );
    }

    // Ensure ICP CLI response wrappers without did metadata still parse.
    #[test]
    fn parses_metrics_response_candid_text() {
        let entries = parse_metrics_page(
            r#"{"response_candid":"(\n  variant {\n    Ok = record {\n      total = 2 : nat64;\n      entries = vec {\n        record {\n          \"principal\" = null;\n          value = variant { Count = 1 : nat64 };\n          labels = vec { \"canister_ops\"; \"create\"; \"app\"; \"completed\"; \"ok\" };\n        };\n        record {\n          \"principal\" = opt principal \"aaaaa-aa\";\n          value = variant { CountAndU64 = record { count = 3 : nat64; value_u64 = 12 : nat64 } };\n          labels = vec { \"timer\"; \"tick\" };\n        };\n      };\n    }\n  },\n)"}"#,
        )
        .expect("parse response_candid metrics page");

        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].labels,
            ["canister_ops", "create", "app", "completed", "ok"]
        );
        assert_eq!(entries[0].value, MetricValue::Count { count: 1 });
        assert_eq!(entries[1].principal.as_deref(), Some("aaaaa-aa"));
        assert_eq!(
            entries[1].value,
            MetricValue::CountAndU64 {
                count: 3,
                value_u64: 12
            }
        );
    }

    // Ensure zero filtering treats every payload shape consistently.
    #[test]
    fn detects_zero_metric_values() {
        assert!(MetricValue::Count { count: 0 }.is_zero());
        assert!(
            MetricValue::CountAndU64 {
                count: 0,
                value_u64: 0
            }
            .is_zero()
        );
        assert!(!MetricValue::U128 { value: 1 }.is_zero());
    }

    // Ensure method-missing responses do not stretch the table with raw ICP output.
    #[test]
    fn shortens_metrics_unavailable_errors() {
        let entry = RegistryEntry {
            pid: "aaaaa-aa".to_string(),
            role: Some("wasm_store".to_string()),
            kind: Some("wasm_store".to_string()),
            parent_pid: None,
            module_hash: None,
        };
        let report = metrics_error_report(
            &entry,
            "icp command failed\nCanister has no query method 'canic_metrics'.",
        );

        assert_eq!(report.status, "unavailable");
        assert_eq!(report.error.as_deref(), Some("canic_metrics unavailable"));
    }
}

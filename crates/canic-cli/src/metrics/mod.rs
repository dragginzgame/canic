use crate::{
    args::{
        default_icp, flag_arg, internal_icp_arg, internal_network_arg, local_network,
        parse_matches, path_option, print_help_or_version, string_option, value_arg,
    },
    output, version_text,
};
use canic_backup::discovery::{DiscoveryError, RegistryEntry, parse_registry_entries};
use canic_host::{
    format::cycles_tc,
    icp::{IcpCli, IcpCommandError},
    install_root::read_named_fleet_install_state,
    replica_query,
    table::{ColumnAlign, render_table},
};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    sync::Arc,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const DEFAULT_SINCE_SECONDS: u64 = 24 * 60 * 60;
const DEFAULT_LIMIT: u64 = 1_000;

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

    #[error("invalid duration {0}; use values like 1h, 6h, 24h, 7d, or 30m")]
    InvalidDuration(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
}

///
/// MetricsOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetricsOptions {
    pub fleet: String,
    pub since_seconds: u64,
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
    pub since_seconds: u64,
    pub generated_at_secs: u64,
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
    pub sample_count: usize,
    pub total_samples: u64,
    pub requested_since_secs: u64,
    pub coverage_seconds: Option<u64>,
    pub coverage_status: String,
    pub latest_timestamp_secs: Option<u64>,
    pub latest_cycles: Option<u128>,
    pub baseline_timestamp_secs: Option<u64>,
    pub baseline_cycles: Option<u128>,
    pub delta_cycles: Option<i128>,
    pub rate_cycles_per_hour: Option<i128>,
    pub error: Option<String>,
}

///
/// CycleTrackerPage
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct CycleTrackerPage {
    entries: Vec<CycleTrackerSample>,
    total: u64,
}

///
/// CycleTrackerSample
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct CycleTrackerSample {
    timestamp_secs: u64,
    cycles: u128,
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
        let since_seconds = string_option(&matches, "since")
            .map(|value| parse_duration(&value))
            .transpose()?
            .unwrap_or(DEFAULT_SINCE_SECONDS);
        let limit = string_option(&matches, "limit")
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|limit| *limit > 0)
            .unwrap_or(DEFAULT_LIMIT);

        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
            since_seconds,
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
    let generated_at_secs = current_unix_seconds();
    let requested_since_secs = generated_at_secs.saturating_sub(options.since_seconds);
    let canisters = collect_cycle_tracker_reports(options, &registry, requested_since_secs);

    Ok(MetricsReport {
        fleet: options.fleet.clone(),
        network: options.network.clone(),
        since_seconds: options.since_seconds,
        generated_at_secs,
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
    Ok(parse_registry_entries(&registry_json)?)
}

fn collect_cycle_tracker_reports(
    options: &MetricsOptions,
    registry: &[RegistryEntry],
    requested_since_secs: u64,
) -> Vec<MetricsCanisterReport> {
    let query = Arc::new(options.clone());
    let mut handles = Vec::new();
    for entry in registry {
        let entry = entry.clone();
        let query = Arc::clone(&query);
        handles.push(thread::spawn(move || {
            cycle_tracker_report(&query, &entry, requested_since_secs)
        }));
    }

    handles
        .into_iter()
        .filter_map(|handle| handle.join().ok())
        .collect()
}

fn cycle_tracker_report(
    options: &MetricsOptions,
    entry: &RegistryEntry,
    requested_since_secs: u64,
) -> MetricsCanisterReport {
    let result = query_cycle_tracker(options, &entry.pid);
    match result {
        Ok(page) => summarize_cycle_tracker(entry, page, requested_since_secs),
        Err(error) => MetricsCanisterReport {
            role: entry.role.clone().unwrap_or_else(|| "-".to_string()),
            canister_id: entry.pid.clone(),
            status: "error".to_string(),
            sample_count: 0,
            total_samples: 0,
            requested_since_secs,
            coverage_seconds: None,
            coverage_status: "none".to_string(),
            latest_timestamp_secs: None,
            latest_cycles: None,
            baseline_timestamp_secs: None,
            baseline_cycles: None,
            delta_cycles: None,
            rate_cycles_per_hour: None,
            error: Some(error),
        },
    }
}

fn summarize_cycle_tracker(
    entry: &RegistryEntry,
    mut page: CycleTrackerPage,
    requested_since_secs: u64,
) -> MetricsCanisterReport {
    page.entries.sort_by_key(|entry| entry.timestamp_secs);
    let latest = page.entries.last().cloned();
    let baseline = latest.as_ref().and_then(|_| {
        page.entries
            .iter()
            .rev()
            .find(|sample| sample.timestamp_secs <= requested_since_secs)
            .or_else(|| page.entries.first())
            .cloned()
    });
    let delta = latest
        .as_ref()
        .zip(baseline.as_ref())
        .map(|(latest, baseline)| signed_delta(latest.cycles, baseline.cycles));
    let coverage_seconds = latest
        .as_ref()
        .zip(baseline.as_ref())
        .map(|(latest, baseline)| {
            latest
                .timestamp_secs
                .saturating_sub(baseline.timestamp_secs)
        });
    let rate_cycles_per_hour = delta
        .zip(coverage_seconds)
        .and_then(|(delta, coverage)| hourly_rate(delta, coverage));
    let coverage_status = coverage_status(baseline.as_ref(), requested_since_secs);
    let status = if latest.is_some() { "ok" } else { "empty" };

    MetricsCanisterReport {
        role: entry.role.clone().unwrap_or_else(|| "-".to_string()),
        canister_id: entry.pid.clone(),
        status: status.to_string(),
        sample_count: page.entries.len(),
        total_samples: page.total,
        requested_since_secs,
        coverage_seconds,
        coverage_status,
        latest_timestamp_secs: latest.as_ref().map(|sample| sample.timestamp_secs),
        latest_cycles: latest.as_ref().map(|sample| sample.cycles),
        baseline_timestamp_secs: baseline.as_ref().map(|sample| sample.timestamp_secs),
        baseline_cycles: baseline.as_ref().map(|sample| sample.cycles),
        delta_cycles: delta,
        rate_cycles_per_hour,
        error: None,
    }
}

fn query_cycle_tracker(
    options: &MetricsOptions,
    canister_id: &str,
) -> Result<CycleTrackerPage, String> {
    let mut page = query_cycle_tracker_page(options, canister_id, 0, options.limit)?;
    if page.total > options.limit {
        let offset = page.total.saturating_sub(options.limit);
        page = query_cycle_tracker_page(options, canister_id, offset, options.limit)?;
    }
    Ok(page)
}

fn query_cycle_tracker_page(
    options: &MetricsOptions,
    canister_id: &str,
    offset: u64,
    limit: u64,
) -> Result<CycleTrackerPage, String> {
    let arg = format!("(record {{ offset = {offset} : nat64; limit = {limit} : nat64 }})");
    let output = IcpCli::new(&options.icp, None, Some(options.network.clone()))
        .canister_query_arg_output(
            canister_id,
            canic_core::protocol::CANIC_CYCLE_TRACKER,
            &arg,
            Some("json"),
        )
        .map_err(|err| err.to_string())?;

    parse_cycle_tracker_page(&output)
        .or_else(|| parse_cycle_tracker_page_text(&output))
        .ok_or_else(|| "could not parse canic_cycle_tracker response".to_string())
}

fn parse_cycle_tracker_page(output: &str) -> Option<CycleTrackerPage> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    let entries_value = find_field(&value, "entries")?;
    let entries = entries_value
        .as_array()?
        .iter()
        .filter_map(parse_cycle_tracker_sample_json)
        .collect::<Vec<_>>();
    let total = find_field(&value, "total")
        .and_then(parse_json_u64)
        .unwrap_or(entries.len() as u64);

    Some(CycleTrackerPage { entries, total })
}

fn parse_cycle_tracker_sample_json(value: &serde_json::Value) -> Option<CycleTrackerSample> {
    Some(CycleTrackerSample {
        timestamp_secs: find_field(value, "timestamp_secs").and_then(parse_json_u64)?,
        cycles: find_field(value, "cycles").and_then(parse_json_u128)?,
    })
}

fn find_field<'a>(value: &'a serde_json::Value, field: &str) -> Option<&'a serde_json::Value> {
    match value {
        serde_json::Value::Object(map) => map
            .get(field)
            .or_else(|| map.values().find_map(|value| find_field(value, field))),
        serde_json::Value::Array(values) => {
            values.iter().find_map(|value| find_field(value, field))
        }
        _ => None,
    }
}

fn parse_json_u64(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(parse_u64_digits))
}

fn parse_json_u128(value: &serde_json::Value) -> Option<u128> {
    value
        .as_u64()
        .map(u128::from)
        .or_else(|| value.as_str().and_then(parse_u128_digits))
}

fn parse_cycle_tracker_page_text(output: &str) -> Option<CycleTrackerPage> {
    let mut entries = Vec::new();
    for chunk in output.split("record") {
        if !(chunk.contains("timestamp_secs") && chunk.contains("cycles")) {
            continue;
        }
        let timestamp_secs =
            field_number_after(chunk, "timestamp_secs").and_then(parse_u64_digits)?;
        let cycles = field_number_after(chunk, "cycles").and_then(parse_u128_digits)?;
        entries.push(CycleTrackerSample {
            timestamp_secs,
            cycles,
        });
    }
    let total = field_number_after(output, "total")
        .and_then(parse_u64_digits)
        .unwrap_or(entries.len() as u64);
    Some(CycleTrackerPage { entries, total })
}

fn field_number_after<'a>(text: &'a str, field: &str) -> Option<&'a str> {
    let (_, after_field) = text.split_once(field)?;
    let (_, after_eq) = after_field.split_once('=')?;
    Some(after_eq.trim_start())
}

fn parse_u64_digits(text: &str) -> Option<u64> {
    number_digits(text).parse().ok()
}

fn parse_u128_digits(text: &str) -> Option<u128> {
    number_digits(text).parse().ok()
}

fn number_digits(text: &str) -> String {
    text.chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit() || *ch == '_' || *ch == ',')
        .filter(char::is_ascii_digit)
        .collect()
}

fn signed_delta(latest: u128, baseline: u128) -> i128 {
    if latest >= baseline {
        i128::try_from(latest - baseline).unwrap_or(i128::MAX)
    } else {
        -i128::try_from(baseline - latest).unwrap_or(i128::MAX)
    }
}

fn hourly_rate(delta: i128, coverage_seconds: u64) -> Option<i128> {
    if coverage_seconds == 0 {
        return None;
    }
    Some(delta.saturating_mul(3_600) / i128::from(coverage_seconds))
}

fn coverage_status(baseline: Option<&CycleTrackerSample>, requested_since_secs: u64) -> String {
    match baseline {
        Some(sample) if sample.timestamp_secs <= requested_since_secs => "covered".to_string(),
        Some(_) => "partial".to_string(),
        None => "none".to_string(),
    }
}

fn write_metrics_report(
    options: &MetricsOptions,
    report: &MetricsReport,
) -> Result<(), MetricsCommandError> {
    if options.json {
        return output::write_pretty_json::<_, MetricsCommandError>(options.out.as_ref(), report);
    }

    let text = render_metrics_report(report);
    if let Some(path) = &options.out {
        fs::write(path, text)?;
    } else {
        println!("{text}");
    }
    Ok(())
}

fn render_metrics_report(report: &MetricsReport) -> String {
    let rows = report
        .canisters
        .iter()
        .map(|row| {
            [
                row.role.clone(),
                row.canister_id.clone(),
                row.status.clone(),
                row.sample_count.to_string(),
                row.coverage_seconds
                    .map_or_else(|| "-".to_string(), format_duration),
                row.coverage_status.clone(),
                row.latest_cycles.map_or_else(|| "-".to_string(), cycles_tc),
                row.delta_cycles
                    .map_or_else(|| "-".to_string(), format_signed_cycles),
                row.rate_cycles_per_hour
                    .map_or_else(|| "-".to_string(), format_signed_cycles),
            ]
        })
        .collect::<Vec<_>>();

    [
        format!(
            "Fleet: {} (network {}, since {})",
            report.fleet,
            report.network,
            format_duration(report.since_seconds)
        ),
        String::new(),
        render_table(
            &[
                "ROLE",
                "CANISTER_ID",
                "STATUS",
                "SAMPLES",
                "COVERAGE",
                "WINDOW",
                "LATEST",
                "DELTA",
                "PER_H",
            ],
            &rows,
            &[
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Right,
                ColumnAlign::Right,
                ColumnAlign::Left,
                ColumnAlign::Right,
                ColumnAlign::Right,
                ColumnAlign::Right,
            ],
        ),
    ]
    .join("\n")
}

fn format_signed_cycles(value: i128) -> String {
    if value < 0 {
        format!("-{}", cycles_tc(value.unsigned_abs()))
    } else {
        format!("+{}", cycles_tc(value.cast_unsigned()))
    }
}

fn format_duration(seconds: u64) -> String {
    if seconds.is_multiple_of(24 * 60 * 60) {
        format!("{}d", seconds / (24 * 60 * 60))
    } else if seconds.is_multiple_of(60 * 60) {
        format!("{}h", seconds / (60 * 60))
    } else if seconds.is_multiple_of(60) {
        format!("{}m", seconds / 60)
    } else {
        format!("{seconds}s")
    }
}

fn parse_duration(value: &str) -> Result<u64, MetricsCommandError> {
    let value = value.trim();
    let digits = value
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    let suffix = value[digits.len()..].trim();
    let amount = digits
        .parse::<u64>()
        .map_err(|_| MetricsCommandError::InvalidDuration(value.to_string()))?;
    let multiplier = match suffix {
        "s" | "" => 1,
        "m" => 60,
        "h" => 60 * 60,
        "d" => 24 * 60 * 60,
        _ => return Err(MetricsCommandError::InvalidDuration(value.to_string())),
    };
    amount
        .checked_mul(multiplier)
        .filter(|seconds| *seconds > 0)
        .ok_or_else(|| MetricsCommandError::InvalidDuration(value.to_string()))
}

fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
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
        .about("Summarize fleet cycle tracker history")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Installed fleet name to inspect"),
        )
        .arg(
            value_arg("since")
                .long("since")
                .value_name("duration")
                .help("Cycle history window; defaults to 24h"),
        )
        .arg(
            value_arg("limit")
                .long("limit")
                .value_name("entries")
                .help("Maximum tracker samples to fetch per canister; defaults to 1000"),
        )
        .arg(flag_arg("json").long("json"))
        .arg(value_arg("out").long("out").value_name("file"))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure common duration selectors parse into seconds.
    #[test]
    fn parses_duration_selectors() {
        assert_eq!(parse_duration("30m").expect("30m"), 1_800);
        assert_eq!(parse_duration("6h").expect("6h"), 21_600);
        assert_eq!(parse_duration("7d").expect("7d"), 604_800);
        assert!(matches!(
            parse_duration("0h"),
            Err(MetricsCommandError::InvalidDuration(_))
        ));
    }

    // Ensure cycle tracker JSON output can be parsed from wrapped result shapes.
    #[test]
    fn parses_cycle_tracker_json() {
        let page = parse_cycle_tracker_page(
            r#"{"Ok":{"entries":[{"timestamp_secs":10,"cycles":"1000"},{"timestamp_secs":"20","cycles":750}],"total":2}}"#,
        )
        .expect("parse page");

        assert_eq!(page.total, 2);
        assert_eq!(page.entries[0].timestamp_secs, 10);
        assert_eq!(page.entries[1].cycles, 750);
    }

    // Ensure Candid text output remains usable when JSON formatting is unavailable.
    #[test]
    fn parses_cycle_tracker_candid_text() {
        let page = parse_cycle_tracker_page_text(
            "(variant { 17_724 = record { entries = vec { record { cycles = 1_000 : nat; timestamp_secs = 10 : nat64 }; record { cycles = 750 : nat; timestamp_secs = 20 : nat64 } }; total = 2 : nat64 } })",
        )
        .expect("parse candid page");

        assert_eq!(page.total, 2);
        assert_eq!(page.entries.len(), 2);
        assert_eq!(page.entries[0].cycles, 1_000);
    }

    // Ensure summaries report partial windows when no sample exists before the cutoff.
    #[test]
    fn summarizes_partial_cycle_window() {
        let entry = RegistryEntry {
            pid: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            kind: Some("root".to_string()),
            parent_pid: None,
            module_hash: None,
        };
        let report = summarize_cycle_tracker(
            &entry,
            CycleTrackerPage {
                total: 2,
                entries: vec![
                    CycleTrackerSample {
                        timestamp_secs: 100,
                        cycles: 1_000,
                    },
                    CycleTrackerSample {
                        timestamp_secs: 200,
                        cycles: 700,
                    },
                ],
            },
            50,
        );

        assert_eq!(report.coverage_status, "partial");
        assert_eq!(report.delta_cycles, Some(-300));
        assert_eq!(report.rate_cycles_per_hour, Some(-10_800));
    }
}

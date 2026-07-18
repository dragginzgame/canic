//! Module: cycles::transport
//!
//! Responsibility: collect cycle history and supplemental observations for deployment canisters.
//! Does not own: cycle accounting, endpoint DTOs, or report rendering.
//! Boundary: preserves query causes until projecting per-canister report diagnostics.

use crate::{
    cycles::{
        CyclesCommandError,
        model::{
            CycleTopupEventSample, CycleTopupStatus, CycleTrackerPage, CycleTrackerSample,
            CyclesCanisterReport, CyclesCanisterStatus, CyclesCoverageStatus, CyclesReport,
            CyclesTopupSummary,
        },
        options::CyclesOptions,
        parse::{parse_cycle_tracker_page, parse_topup_event_page},
    },
    support::{
        candid::registry_entry_candid_path,
        registry_tree::{RegistryRow, visible_rows},
    },
};
use canic_host::{
    cycle_balance::{CycleBalanceQueryError, query_cycle_balance},
    icp::{IcpCli, IcpCommandError, IcpJsonResponseError},
    icp_config::resolve_current_canic_icp_root,
    installed_deployment::{
        InstalledDeploymentRequest, InstalledDeploymentResolution,
        resolve_installed_deployment_from_root,
    },
    registry::RegistryEntry,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const TOPUP_EVENTS_LIMIT: u64 = 1_000;
const ICP_JSON_OUTPUT: &str = "json";
const CYCLES_WORKER_PANIC: &str = "cycles query worker panicked";

///
/// CycleQueryTarget
///

struct CycleQueryTarget {
    icp: IcpCli,
    canister_id: String,
    environment: String,
    icp_root: Option<PathBuf>,
    candid_path: Option<PathBuf>,
}

#[derive(Debug, ThisError)]
enum CycleObservationError {
    #[error(transparent)]
    Balance(#[from] CycleBalanceQueryError),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error("invalid {method} response: {source}")]
    Response {
        method: &'static str,
        #[source]
        source: IcpJsonResponseError,
    },
}

pub(super) fn cycles_report(options: &CyclesOptions) -> Result<CyclesReport, CyclesCommandError> {
    let registry = load_registry(options)?;
    let generated_at_secs = current_unix_seconds();
    let requested_since_secs = generated_at_secs.saturating_sub(options.since_seconds);
    let canisters =
        collect_cycle_tracker_reports(options, &registry, requested_since_secs, generated_at_secs)?;

    Ok(CyclesReport {
        deployment: options.deployment.clone(),
        environment: options.environment.clone(),
        since_seconds: options.since_seconds,
        generated_at_secs,
        canisters,
    })
}

fn load_registry(options: &CyclesOptions) -> Result<Vec<RegistryEntry>, CyclesCommandError> {
    Ok(resolve_cycles_deployment(options)?.registry.entries)
}

fn collect_cycle_tracker_reports(
    options: &CyclesOptions,
    registry: &[RegistryEntry],
    requested_since_secs: u64,
    generated_at_secs: u64,
) -> Result<Vec<CyclesCanisterReport>, CyclesCommandError> {
    let query = Arc::new(options.clone());
    let mut handles = Vec::new();
    let rows = visible_rows(registry, options.subtree.as_deref())?;
    for row in rows {
        let RegistryRow { entry, tree_prefix } = row;
        let entry = entry.clone();
        let worker_entry = entry.clone();
        let worker_tree_prefix = tree_prefix.clone();
        let query = Arc::clone(&query);
        handles.push((
            worker_entry,
            worker_tree_prefix,
            thread::spawn(move || {
                cycle_tracker_report(
                    &query,
                    &entry,
                    tree_prefix,
                    requested_since_secs,
                    generated_at_secs,
                )
            }),
        ));
    }

    Ok(collect_cycle_worker_reports(handles, requested_since_secs))
}

fn collect_cycle_worker_reports(
    handles: Vec<(
        RegistryEntry,
        String,
        thread::JoinHandle<CyclesCanisterReport>,
    )>,
    requested_since_secs: u64,
) -> Vec<CyclesCanisterReport> {
    handles
        .into_iter()
        .map(|(entry, tree_prefix, handle)| {
            handle.join().unwrap_or_else(|_| {
                cycles_error_report(
                    &entry,
                    tree_prefix,
                    requested_since_secs,
                    None,
                    CYCLES_WORKER_PANIC.to_string(),
                )
            })
        })
        .collect()
}

fn cycle_tracker_report(
    options: &CyclesOptions,
    entry: &RegistryEntry,
    tree_prefix: String,
    requested_since_secs: u64,
    generated_at_secs: u64,
) -> CyclesCanisterReport {
    let target = cycle_query_target(options, entry);
    let live_cycles = query_live_cycle_balance(&target);
    let result = query_cycle_tracker(&target, options.limit);
    match result {
        Ok(page) => {
            let topup_events = query_topup_events(&target);
            let live_cycles_error = live_cycles.as_ref().err().map(ToString::to_string);
            let topup_events_error = topup_events.as_ref().err().map(ToString::to_string);
            let observation_error = cycle_observation_error(
                live_cycles_error.as_deref(),
                topup_events_error.as_deref(),
            );
            let mut report = summarize_cycle_tracker(
                entry,
                page,
                tree_prefix,
                requested_since_secs,
                generated_at_secs,
                live_cycles.as_ref().ok().copied(),
                topup_events.ok(),
            );
            if let Some(error) = observation_error {
                report.status = CyclesCanisterStatus::Error;
                report.error = Some(error);
            }
            report
        }
        Err(error) => cycles_error_report(
            entry,
            tree_prefix,
            requested_since_secs,
            live_cycles.ok().map(|cycles| (generated_at_secs, cycles)),
            error.to_string(),
        ),
    }
}

fn cycles_error_report(
    entry: &RegistryEntry,
    tree_prefix: String,
    requested_since_secs: u64,
    live_cycles: Option<(u64, u128)>,
    error: String,
) -> CyclesCanisterReport {
    CyclesCanisterReport {
        role: entry.role.clone().unwrap_or_else(|| "-".to_string()),
        tree_prefix,
        canister_id: entry.pid.clone(),
        status: CyclesCanisterStatus::Error,
        sample_count: 0,
        total_samples: 0,
        requested_since_secs,
        coverage_seconds: None,
        coverage_status: CyclesCoverageStatus::None,
        latest_timestamp_secs: live_cycles.map(|(timestamp, _)| timestamp),
        latest_cycles: live_cycles.map(|(_, cycles)| cycles),
        baseline_timestamp_secs: None,
        baseline_cycles: None,
        delta_cycles: None,
        rate_cycles_per_hour: None,
        burn_cycles: None,
        burn_cycles_per_hour: None,
        topup_cycles_per_hour: None,
        topups: None,
        error: Some(error),
    }
}

fn first_line(error: &str) -> &str {
    error.lines().next().unwrap_or(error)
}

fn cycle_observation_error(
    live_cycles_error: Option<&str>,
    topup_events_error: Option<&str>,
) -> Option<String> {
    let mut errors = Vec::new();
    if let Some(error) = live_cycles_error {
        errors.push(format!("live cycle balance: {}", first_line(error)));
    }
    if let Some(error) = topup_events_error {
        errors.push(format!("top-up events: {}", first_line(error)));
    }
    (!errors.is_empty()).then(|| errors.join("; "))
}

pub(super) fn summarize_cycle_tracker(
    entry: &RegistryEntry,
    mut page: CycleTrackerPage,
    tree_prefix: String,
    requested_since_secs: u64,
    generated_at_secs: u64,
    live_cycles: Option<u128>,
    topup_events: Option<Vec<CycleTopupEventSample>>,
) -> CyclesCanisterReport {
    page.entries.sort_by_key(|entry| entry.timestamp_secs);
    let tracker_latest = page.entries.last().cloned();
    let latest = live_cycles
        .map(|cycles| CycleTrackerSample {
            timestamp_secs: generated_at_secs,
            cycles,
        })
        .or(tracker_latest);
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
    let topup_summary = topup_events
        .as_deref()
        .zip(baseline.as_ref())
        .zip(latest.as_ref())
        .map(|((events, baseline), latest)| {
            topup_summary_from_events(events, baseline.timestamp_secs, latest.timestamp_secs)
        });
    let topup_cycles = topup_summary
        .as_ref()
        .map_or(0, |summary| summary.transferred_cycles);
    let topup_cycles_per_hour = topup_summary
        .as_ref()
        .zip(coverage_seconds)
        .and_then(|(_, coverage)| unsigned_hourly_rate(topup_cycles, coverage));
    let burn_cycles = topup_summary
        .as_ref()
        .zip(delta)
        .and_then(|(_, delta)| inferred_burn_cycles(topup_cycles, delta));
    let burn_cycles_per_hour = topup_summary
        .as_ref()
        .zip(burn_cycles)
        .zip(coverage_seconds)
        .and_then(|((_, burn), coverage)| unsigned_hourly_rate(burn, coverage));
    let visible_topups = topup_summary.filter(|summary| !topup_summary_is_empty(summary));
    let coverage_status = coverage_status(baseline.as_ref(), requested_since_secs);
    let status = if latest.is_some() {
        CyclesCanisterStatus::Ok
    } else {
        CyclesCanisterStatus::Empty
    };

    CyclesCanisterReport {
        role: entry.role.clone().unwrap_or_else(|| "-".to_string()),
        tree_prefix,
        canister_id: entry.pid.clone(),
        status,
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
        burn_cycles,
        burn_cycles_per_hour,
        topup_cycles_per_hour,
        topups: visible_topups,
        error: None,
    }
}

fn query_live_cycle_balance(target: &CycleQueryTarget) -> Result<u128, CycleObservationError> {
    query_cycle_balance(
        &target.icp,
        &target.canister_id,
        &target.environment,
        target.icp_root.as_deref(),
        target.candid_path.as_deref(),
    )
    .map_err(Into::into)
}

fn query_topup_events(
    target: &CycleQueryTarget,
) -> Result<Vec<CycleTopupEventSample>, CycleObservationError> {
    let mut page = query_topup_event_page(target, 0, TOPUP_EVENTS_LIMIT)?;
    if page.total > TOPUP_EVENTS_LIMIT {
        let offset = page.total.saturating_sub(TOPUP_EVENTS_LIMIT);
        page = query_topup_event_page(target, offset, TOPUP_EVENTS_LIMIT)?;
    }
    Ok(page.entries)
}

fn topup_summary_from_events(
    entries: &[CycleTopupEventSample],
    start_secs: u64,
    end_secs: u64,
) -> CyclesTopupSummary {
    let mut summary = CyclesTopupSummary::default();
    for entry in entries {
        if entry.timestamp_secs < start_secs || entry.timestamp_secs > end_secs {
            continue;
        }
        match entry.status {
            CycleTopupStatus::RequestScheduled => {
                summary.request_scheduled = summary.request_scheduled.saturating_add(1);
            }
            CycleTopupStatus::RequestOk => {
                summary.request_ok = summary.request_ok.saturating_add(1);
                summary.transferred_cycles = summary
                    .transferred_cycles
                    .saturating_add(entry.transferred_cycles.unwrap_or_default());
            }
            CycleTopupStatus::RequestErr => {
                summary.request_err = summary.request_err.saturating_add(1);
            }
        }
    }
    summary
}

const fn topup_summary_is_empty(summary: &CyclesTopupSummary) -> bool {
    summary.request_scheduled == 0 && summary.request_ok == 0 && summary.request_err == 0
}

fn query_topup_event_page(
    target: &CycleQueryTarget,
    offset: u64,
    limit: u64,
) -> Result<crate::cycles::model::CycleTopupEventPage, CycleObservationError> {
    let arg = page_request_arg(offset, limit);
    let output = target.icp.canister_query_arg_output_with_candid(
        &target.canister_id,
        canic_core::protocol::CANIC_CYCLE_TOPUPS,
        &arg,
        Some(ICP_JSON_OUTPUT),
        target.candid_path.as_deref(),
    )?;

    parse_topup_event_page(&output).map_err(|source| CycleObservationError::Response {
        method: canic_core::protocol::CANIC_CYCLE_TOPUPS,
        source,
    })
}

fn query_cycle_tracker(
    target: &CycleQueryTarget,
    limit: u64,
) -> Result<CycleTrackerPage, CycleObservationError> {
    let mut page = query_cycle_tracker_page(target, 0, limit)?;
    if page.total > limit {
        let offset = page.total.saturating_sub(limit);
        page = query_cycle_tracker_page(target, offset, limit)?;
    }
    Ok(page)
}

fn query_cycle_tracker_page(
    target: &CycleQueryTarget,
    offset: u64,
    limit: u64,
) -> Result<CycleTrackerPage, CycleObservationError> {
    let arg = page_request_arg(offset, limit);
    let output = target.icp.canister_query_arg_output_with_candid(
        &target.canister_id,
        canic_core::protocol::CANIC_CYCLE_TRACKER,
        &arg,
        Some(ICP_JSON_OUTPUT),
        target.candid_path.as_deref(),
    )?;

    parse_cycle_tracker_page(&output).map_err(|source| CycleObservationError::Response {
        method: canic_core::protocol::CANIC_CYCLE_TRACKER,
        source,
    })
}

fn cycle_query_target(options: &CyclesOptions, entry: &RegistryEntry) -> CycleQueryTarget {
    let root = resolve_cycles_icp_root();
    CycleQueryTarget {
        icp: cycles_icp(options, root.as_deref()),
        canister_id: entry.pid.clone(),
        environment: options.environment.clone(),
        icp_root: root.clone(),
        candid_path: registry_entry_candid_path(root.as_deref(), &options.environment, entry),
    }
}

fn cycles_icp(options: &CyclesOptions, root: Option<&Path>) -> IcpCli {
    let icp = IcpCli::new(&options.icp, Some(options.environment.clone()));
    if let Some(root) = root {
        return icp.with_cwd(root);
    }
    icp
}

fn page_request_arg(offset: u64, limit: u64) -> String {
    format!("(record {{ offset = {offset} : nat64; limit = {limit} : nat64 }})")
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

fn unsigned_hourly_rate(value: u128, coverage_seconds: u64) -> Option<u128> {
    if coverage_seconds == 0 {
        return None;
    }
    Some(value.saturating_mul(3_600) / u128::from(coverage_seconds))
}

const fn inferred_burn_cycles(topup_cycles: u128, delta_cycles: i128) -> Option<u128> {
    if delta_cycles < 0 {
        return Some(topup_cycles.saturating_add(delta_cycles.unsigned_abs()));
    }

    let delta = delta_cycles.cast_unsigned();
    topup_cycles.checked_sub(delta)
}

const fn coverage_status(
    baseline: Option<&CycleTrackerSample>,
    requested_since_secs: u64,
) -> CyclesCoverageStatus {
    match baseline {
        Some(sample) if sample.timestamp_secs <= requested_since_secs => {
            CyclesCoverageStatus::Covered
        }
        Some(_) => CyclesCoverageStatus::Partial,
        None => CyclesCoverageStatus::None,
    }
}

fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn resolve_cycles_deployment(
    options: &CyclesOptions,
) -> Result<InstalledDeploymentResolution, CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    resolve_installed_deployment_from_root(
        &InstalledDeploymentRequest {
            deployment: options.deployment.clone(),
            environment: options.environment.clone(),
            icp: options.icp.clone(),
            detect_lost_local_root: false,
        },
        &root,
    )
    .map_err(CyclesCommandError::from)
}

fn resolve_cycles_icp_root() -> Option<PathBuf> {
    resolve_current_canic_icp_root().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panicked_cycles_worker_becomes_an_explicit_canister_error() {
        let entry = RegistryEntry {
            pid: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            parent_pid: None,
            module_hash: None,
        };
        let reports = collect_cycle_worker_reports(
            vec![(
                entry.clone(),
                "root".to_string(),
                thread::spawn(|| panic!("simulated cycles worker panic")),
            )],
            100,
        );

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].canister_id, entry.pid);
        assert_eq!(reports[0].status, CyclesCanisterStatus::Error);
        assert_eq!(reports[0].error.as_deref(), Some(CYCLES_WORKER_PANIC));
    }

    #[test]
    fn supplemental_cycle_query_failures_are_not_silently_dropped() {
        assert_eq!(
            cycle_observation_error(
                Some("balance transport failed\nraw details"),
                Some("top-up response malformed"),
            )
            .as_deref(),
            Some(
                "live cycle balance: balance transport failed; top-up events: top-up response malformed"
            )
        );
        assert_eq!(cycle_observation_error(None, None), None);
    }

    #[test]
    fn cycle_response_failure_preserves_typed_cause_until_projection() {
        let error = CycleObservationError::Response {
            method: canic_core::protocol::CANIC_CYCLE_TRACKER,
            source: IcpJsonResponseError::MissingResponseBytes,
        };
        let source = std::error::Error::source(&error).expect("typed response source");

        assert!(matches!(
            source.downcast_ref::<IcpJsonResponseError>(),
            Some(IcpJsonResponseError::MissingResponseBytes)
        ));
    }
}

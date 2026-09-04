//! Module: cycles::transport
//!
//! Responsibility: collect cycle history and supplemental observations for Fleet canisters.
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
        parse::{cycle_tracker_page, topup_event_page},
    },
    support::registry_tree::{RegistryRow, visible_rows},
};
use canic_core::dto::{
    observability::{CanisterObservabilityRequest, CanisterObservabilityResponse},
    page::PageRequest,
};
use canic_core::{ids::CanisterRole, role_contract::RoleCapabilityKey};
use canic_host::{
    fleet_ensure::{CurrentFleetResolution, resolve_current_fleet},
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
    observability::{FleetObservabilityError, observe_fleet_canister},
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
const CYCLES_WORKER_PANIC: &str = "cycles query worker panicked";
///
/// CycleQueryTarget
///

struct CycleQueryTarget {
    icp: IcpCli,
    entry: RegistryEntry,
    environment: String,
    icp_root: PathBuf,
    fleet: Arc<CurrentFleetResolution>,
}

#[derive(Debug, ThisError)]
enum CycleObservationError {
    #[error(transparent)]
    Observability(#[from] FleetObservabilityError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CycleObservationPlan {
    BalanceOnly,
    History,
    HistoryWithTopups,
    Unavailable,
}

#[derive(Debug, ThisError)]
enum SupplementalCycleObservationError {
    #[error("live cycle balance: {source}")]
    LiveBalance {
        #[source]
        source: CycleObservationError,
    },

    #[error("top-up events: {source}")]
    TopupEvents {
        #[source]
        source: CycleObservationError,
    },

    #[error("live cycle balance: {live_balance}; top-up events: {topup_events}")]
    LiveBalanceAndTopups {
        live_balance: CycleObservationError,
        topup_events: CycleObservationError,
    },
}

pub(super) fn cycles_report(options: &CyclesOptions) -> Result<CyclesReport, CyclesCommandError> {
    let fleet = Arc::new(load_registry(options)?);
    let generated_at_secs = current_unix_seconds();
    let requested_since_secs = generated_at_secs.saturating_sub(options.since_seconds);
    let canisters =
        collect_cycle_tracker_reports(options, fleet, requested_since_secs, generated_at_secs)?;

    Ok(CyclesReport {
        fleet: options.fleet.clone(),
        environment: options.environment.clone(),
        since_seconds: options.since_seconds,
        generated_at_secs,
        canisters,
    })
}

fn load_registry(options: &CyclesOptions) -> Result<CurrentFleetResolution, CyclesCommandError> {
    resolve_cycles_fleet(options)
}

fn collect_cycle_tracker_reports(
    options: &CyclesOptions,
    fleet: Arc<CurrentFleetResolution>,
    requested_since_secs: u64,
    generated_at_secs: u64,
) -> Result<Vec<CyclesCanisterReport>, CyclesCommandError> {
    let query = Arc::new(options.clone());
    let mut handles = Vec::new();
    let rows = visible_rows(&fleet.registry.entries, options.subtree.as_deref())?;
    for row in rows {
        let RegistryRow { entry, tree_prefix } = row;
        let entry = entry.clone();
        let worker_entry = entry.clone();
        let worker_tree_prefix = tree_prefix.clone();
        let query = Arc::clone(&query);
        let fleet = Arc::clone(&fleet);
        handles.push((
            worker_entry,
            worker_tree_prefix,
            thread::spawn(move || {
                cycle_tracker_report(
                    &query,
                    fleet,
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
    fleet: Arc<CurrentFleetResolution>,
    entry: &RegistryEntry,
    tree_prefix: String,
    requested_since_secs: u64,
    generated_at_secs: u64,
) -> CyclesCanisterReport {
    let plan = cycle_observation_plan(entry);
    if plan == CycleObservationPlan::Unavailable {
        return cycles_unavailable_report(entry, tree_prefix, requested_since_secs);
    }
    let target = match cycle_query_target(options, fleet, entry) {
        Ok(target) => target,
        Err(error) => {
            return cycles_error_report(
                entry,
                tree_prefix,
                requested_since_secs,
                None,
                error.to_string(),
            );
        }
    };
    let live_cycles = query_live_cycle_balance(&target);
    if plan == CycleObservationPlan::BalanceOnly {
        return match live_cycles {
            Ok(cycles) => cycles_balance_only_report(
                entry,
                tree_prefix,
                requested_since_secs,
                generated_at_secs,
                cycles,
            ),
            Err(error) => cycles_error_report(
                entry,
                tree_prefix,
                requested_since_secs,
                None,
                error.to_string(),
            ),
        };
    }
    let result = query_cycle_tracker(&target, options.limit);
    match result {
        Ok(page) => {
            let (live_cycles, live_cycles_error) = split_cycle_observation(live_cycles);
            let (topup_events, topup_events_error) =
                if plan == CycleObservationPlan::HistoryWithTopups {
                    split_cycle_observation(query_topup_events(&target))
                } else {
                    (None, None)
                };
            let observation_error =
                supplemental_cycle_observation_error(live_cycles_error, topup_events_error);
            let mut report = summarize_cycle_tracker(
                entry,
                page,
                tree_prefix,
                requested_since_secs,
                generated_at_secs,
                live_cycles,
                topup_events,
            );
            if let Some(error) = observation_error {
                report.status = CyclesCanisterStatus::Error;
                report.error = Some(error.to_string());
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

fn cycle_observation_plan(entry: &RegistryEntry) -> CycleObservationPlan {
    cycle_observation_plan_for(
        entry.role.as_deref(),
        entry
            .protocol_binding
            .as_ref()
            .map(|binding| &binding.capabilities),
    )
}

fn cycle_observation_plan_for(
    role: Option<&str>,
    capabilities: Option<&std::collections::BTreeSet<RoleCapabilityKey>>,
) -> CycleObservationPlan {
    if role == Some(CanisterRole::FLEET_COORDINATOR.as_str()) {
        return CycleObservationPlan::Unavailable;
    }
    let Some(capabilities) = capabilities else {
        return CycleObservationPlan::BalanceOnly;
    };
    if !capabilities.contains(&RoleCapabilityKey::Runtime) {
        return CycleObservationPlan::BalanceOnly;
    }
    if capabilities.contains(&RoleCapabilityKey::AutomaticTopup)
        && !matches!(
            role,
            Some(value)
                if value == CanisterRole::ROOT.as_str()
                    || value == CanisterRole::WASM_STORE.as_str()
        )
    {
        CycleObservationPlan::HistoryWithTopups
    } else {
        CycleObservationPlan::History
    }
}

fn split_cycle_observation<T>(
    result: Result<T, CycleObservationError>,
) -> (Option<T>, Option<CycleObservationError>) {
    match result {
        Ok(value) => (Some(value), None),
        Err(error) => (None, Some(error)),
    }
}

fn supplemental_cycle_observation_error(
    live_balance: Option<CycleObservationError>,
    topup_events: Option<CycleObservationError>,
) -> Option<SupplementalCycleObservationError> {
    match (live_balance, topup_events) {
        (Some(source), None) => Some(SupplementalCycleObservationError::LiveBalance { source }),
        (None, Some(source)) => Some(SupplementalCycleObservationError::TopupEvents { source }),
        (Some(live_balance), Some(topup_events)) => {
            Some(SupplementalCycleObservationError::LiveBalanceAndTopups {
                live_balance,
                topup_events,
            })
        }
        (None, None) => None,
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

fn cycles_unavailable_report(
    entry: &RegistryEntry,
    tree_prefix: String,
    requested_since_secs: u64,
) -> CyclesCanisterReport {
    cycles_limited_report(
        entry,
        tree_prefix,
        requested_since_secs,
        CyclesCanisterStatus::Unavailable,
        None,
    )
}

fn cycles_balance_only_report(
    entry: &RegistryEntry,
    tree_prefix: String,
    requested_since_secs: u64,
    observed_at_secs: u64,
    cycles: u128,
) -> CyclesCanisterReport {
    cycles_limited_report(
        entry,
        tree_prefix,
        requested_since_secs,
        CyclesCanisterStatus::BalanceOnly,
        Some((observed_at_secs, cycles)),
    )
}

fn cycles_limited_report(
    entry: &RegistryEntry,
    tree_prefix: String,
    requested_since_secs: u64,
    status: CyclesCanisterStatus,
    live_cycles: Option<(u64, u128)>,
) -> CyclesCanisterReport {
    CyclesCanisterReport {
        role: entry.role.clone().unwrap_or_else(|| "-".to_string()),
        tree_prefix,
        canister_id: entry.pid.clone(),
        status,
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
        error: None,
    }
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
    let response = observe_fleet_canister(
        &target.icp,
        &target.icp_root,
        &target.environment,
        &target.fleet,
        &target.entry,
        CanisterObservabilityRequest::CycleBalance,
    )?;
    let CanisterObservabilityResponse::CycleBalance(response) = response else {
        unreachable!("CycleBalance request returned a different observability response");
    };
    Ok(response.cycles)
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
    let response = observe_fleet_canister(
        &target.icp,
        &target.icp_root,
        &target.environment,
        &target.fleet,
        &target.entry,
        CanisterObservabilityRequest::CycleTopups(PageRequest { offset, limit }),
    )?;
    let CanisterObservabilityResponse::CycleTopups(page) = response else {
        unreachable!("CycleTopups request returned a different observability response");
    };
    Ok(topup_event_page(page))
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
    let response = observe_fleet_canister(
        &target.icp,
        &target.icp_root,
        &target.environment,
        &target.fleet,
        &target.entry,
        CanisterObservabilityRequest::CycleHistory(PageRequest { offset, limit }),
    )?;
    let CanisterObservabilityResponse::CycleHistory(page) = response else {
        unreachable!("CycleHistory request returned a different observability response");
    };
    Ok(cycle_tracker_page(page))
}

fn cycle_query_target(
    options: &CyclesOptions,
    fleet: Arc<CurrentFleetResolution>,
    entry: &RegistryEntry,
) -> Result<CycleQueryTarget, canic_host::icp_config::IcpConfigError> {
    let root = resolve_current_canic_icp_root()?;
    Ok(CycleQueryTarget {
        icp: cycles_icp(options, Some(&root)),
        entry: entry.clone(),
        environment: options.environment.clone(),
        icp_root: root,
        fleet,
    })
}

fn cycles_icp(options: &CyclesOptions, root: Option<&Path>) -> IcpCli {
    let icp = IcpCli::new(&options.icp, Some(options.environment.clone()));
    if let Some(root) = root {
        return icp.with_cwd(root);
    }
    icp
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

fn resolve_cycles_fleet(
    options: &CyclesOptions,
) -> Result<CurrentFleetResolution, CyclesCommandError> {
    let root = resolve_current_canic_icp_root().map_err(CyclesCommandError::IcpRoot)?;
    resolve_current_fleet(&root, &options.environment, &options.fleet)
        .map_err(CyclesCommandError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_host::{CanisterProtocolError, icp::IcpJsonResponseError};

    #[test]
    fn panicked_cycles_worker_becomes_an_explicit_canister_error() {
        let entry = RegistryEntry {
            pid: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            parent_pid: None,
            module_hash: None,
            protocol_binding: None,
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
    fn cycle_observation_plan_is_capability_and_role_bound() {
        use std::collections::BTreeSet;

        let runtime = BTreeSet::from([RoleCapabilityKey::Runtime]);
        let automatic = BTreeSet::from([
            RoleCapabilityKey::AutomaticTopup,
            RoleCapabilityKey::Runtime,
        ]);

        assert_eq!(
            cycle_observation_plan_for(Some(CanisterRole::FLEET_COORDINATOR.as_str()), None),
            CycleObservationPlan::Unavailable
        );
        assert_eq!(
            cycle_observation_plan_for(Some("canister_pool_asset"), None),
            CycleObservationPlan::BalanceOnly
        );
        assert_eq!(
            cycle_observation_plan_for(Some("plain"), Some(&runtime)),
            CycleObservationPlan::History
        );
        assert_eq!(
            cycle_observation_plan_for(Some("funded"), Some(&automatic)),
            CycleObservationPlan::HistoryWithTopups
        );
        assert_eq!(
            cycle_observation_plan_for(Some(CanisterRole::ROOT.as_str()), Some(&automatic)),
            CycleObservationPlan::History
        );
        assert_eq!(
            cycle_observation_plan_for(Some(CanisterRole::WASM_STORE.as_str()), Some(&automatic)),
            CycleObservationPlan::History
        );
    }

    #[test]
    fn limited_cycle_reports_distinguish_unavailable_from_balance_only() {
        let entry = RegistryEntry {
            pid: "aaaaa-aa".to_string(),
            role: Some(CanisterRole::FLEET_COORDINATOR.to_string()),
            parent_pid: None,
            module_hash: None,
            protocol_binding: None,
        };

        let unavailable = cycles_unavailable_report(&entry, String::new(), 10);
        let balance_only = cycles_balance_only_report(&entry, String::new(), 10, 20, 30);

        assert_eq!(unavailable.status, CyclesCanisterStatus::Unavailable);
        assert_eq!(unavailable.latest_cycles, None);
        assert_eq!(balance_only.status, CyclesCanisterStatus::BalanceOnly);
        assert_eq!(balance_only.latest_timestamp_secs, Some(20));
        assert_eq!(balance_only.latest_cycles, Some(30));
        assert_eq!(
            serde_json::to_value(balance_only).expect("serialize balance-only report")["status"],
            "balance_only"
        );
    }

    #[test]
    fn supplemental_cycle_query_failures_remain_typed_until_projection() {
        let error = supplemental_cycle_observation_error(
            Some(CycleObservationError::Observability(
                FleetObservabilityError::MissingRoot {
                    canister: "aaaaa-aa".to_string(),
                },
            )),
            Some(CycleObservationError::Observability(
                FleetObservabilityError::RootCycleTopupsUnsupported,
            )),
        );

        assert!(matches!(
            error,
            Some(SupplementalCycleObservationError::LiveBalanceAndTopups {
                live_balance: CycleObservationError::Observability(
                    FleetObservabilityError::MissingRoot { canister }
                ),
                topup_events: CycleObservationError::Observability(
                    FleetObservabilityError::RootCycleTopupsUnsupported
                ),
            }) if canister == "aaaaa-aa"
        ));
        assert!(supplemental_cycle_observation_error(None, None).is_none());
    }

    #[test]
    fn cycle_response_failure_preserves_typed_cause_until_projection() {
        let error = CycleObservationError::Observability(FleetObservabilityError::Protocol(
            CanisterProtocolError::Response {
                canister: candid::Principal::management_canister(),
                method: canic_core::protocol::CANIC_ROOT_COMMAND,
                source: IcpJsonResponseError::MissingResponseBytes,
            },
        ));
        let mut source = std::error::Error::source(&error);
        let mut preserved = false;
        while let Some(cause) = source {
            if matches!(
                cause.downcast_ref::<IcpJsonResponseError>(),
                Some(IcpJsonResponseError::MissingResponseBytes)
            ) {
                preserved = true;
                break;
            }
            source = cause.source();
        }

        assert!(preserved, "typed response cause must remain in the chain");
    }
}

use crate::{
    cycles::{
        CyclesCommandError,
        model::{
            CycleTopupEventSample, CycleTopupStatus, CycleTrackerPage, CycleTrackerSample,
            CyclesCanisterReport, CyclesReport, CyclesTopupSummary,
        },
        options::CyclesOptions,
        parse::{
            parse_cycle_tracker_page, parse_cycle_tracker_page_text, parse_topup_event_page,
            parse_topup_event_page_text,
        },
    },
    support::{
        candid::registry_entry_candid_path,
        registry_tree::{RegistryRow, visible_rows},
    },
};
use canic_host::{
    cycle_balance::query_cycle_balance_optional,
    icp::IcpCli,
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

const TOPUP_EVENTS_LIMIT: u64 = 1_000;
const ICP_JSON_OUTPUT: &str = "json";

///
/// CycleQueryTarget
///

struct CycleQueryTarget {
    icp: IcpCli,
    canister_id: String,
    network: String,
    icp_root: Option<PathBuf>,
    candid_path: Option<PathBuf>,
}

pub(super) fn cycles_report(options: &CyclesOptions) -> Result<CyclesReport, CyclesCommandError> {
    let registry = load_registry(options)?;
    let generated_at_secs = current_unix_seconds();
    let requested_since_secs = generated_at_secs.saturating_sub(options.since_seconds);
    let canisters =
        collect_cycle_tracker_reports(options, &registry, requested_since_secs, generated_at_secs)?;

    Ok(CyclesReport {
        deployment: options.deployment.clone(),
        network: options.network.clone(),
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
        let query = Arc::clone(&query);
        handles.push(thread::spawn(move || {
            cycle_tracker_report(
                &query,
                &entry,
                tree_prefix,
                requested_since_secs,
                generated_at_secs,
            )
        }));
    }

    Ok(handles
        .into_iter()
        .filter_map(|handle| handle.join().ok())
        .collect())
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
        Ok(page) => summarize_cycle_tracker(
            entry,
            page,
            tree_prefix,
            requested_since_secs,
            generated_at_secs,
            live_cycles,
            query_topup_events(&target).ok(),
        ),
        Err(error) => CyclesCanisterReport {
            role: entry.role.clone().unwrap_or_else(|| "-".to_string()),
            tree_prefix,
            canister_id: entry.pid.clone(),
            status: "error".to_string(),
            sample_count: 0,
            total_samples: 0,
            requested_since_secs,
            coverage_seconds: None,
            coverage_status: "none".to_string(),
            latest_timestamp_secs: live_cycles.map(|_| generated_at_secs),
            latest_cycles: live_cycles,
            baseline_timestamp_secs: None,
            baseline_cycles: None,
            delta_cycles: None,
            rate_cycles_per_hour: None,
            burn_cycles: None,
            burn_cycles_per_hour: None,
            topup_cycles_per_hour: None,
            topups: None,
            error: Some(error),
        },
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
    let status = if latest.is_some() { "ok" } else { "empty" };

    CyclesCanisterReport {
        role: entry.role.clone().unwrap_or_else(|| "-".to_string()),
        tree_prefix,
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
        burn_cycles,
        burn_cycles_per_hour,
        topup_cycles_per_hour,
        topups: visible_topups,
        error: None,
    }
}

fn query_live_cycle_balance(target: &CycleQueryTarget) -> Option<u128> {
    query_cycle_balance_optional(
        &target.icp,
        &target.canister_id,
        &target.network,
        target.icp_root.as_deref(),
        target.candid_path.as_deref(),
    )
}

fn query_topup_events(target: &CycleQueryTarget) -> Result<Vec<CycleTopupEventSample>, String> {
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
) -> Result<crate::cycles::model::CycleTopupEventPage, String> {
    let arg = page_request_arg(offset, limit);
    let output = target
        .icp
        .canister_query_arg_output_with_candid(
            &target.canister_id,
            canic_core::protocol::CANIC_CYCLE_TOPUPS,
            &arg,
            Some(ICP_JSON_OUTPUT),
            target.candid_path.as_deref(),
        )
        .map_err(|err| err.to_string())?;

    parse_topup_event_page(&output)
        .or_else(|| parse_topup_event_page_text(&output))
        .ok_or_else(|| "could not parse canic_cycle_topups response".to_string())
}

fn query_cycle_tracker(target: &CycleQueryTarget, limit: u64) -> Result<CycleTrackerPage, String> {
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
) -> Result<CycleTrackerPage, String> {
    let arg = page_request_arg(offset, limit);
    let output = target
        .icp
        .canister_query_arg_output_with_candid(
            &target.canister_id,
            canic_core::protocol::CANIC_CYCLE_TRACKER,
            &arg,
            Some(ICP_JSON_OUTPUT),
            target.candid_path.as_deref(),
        )
        .map_err(|err| err.to_string())?;

    parse_cycle_tracker_page(&output)
        .or_else(|| parse_cycle_tracker_page_text(&output))
        .ok_or_else(|| "could not parse canic_cycle_tracker response".to_string())
}

fn cycle_query_target(options: &CyclesOptions, entry: &RegistryEntry) -> CycleQueryTarget {
    let root = resolve_cycles_icp_root();
    CycleQueryTarget {
        icp: cycles_icp(options, root.as_deref()),
        canister_id: entry.pid.clone(),
        network: options.network.clone(),
        icp_root: root.clone(),
        candid_path: registry_entry_candid_path(root.as_deref(), &options.network, entry),
    }
}

fn cycles_icp(options: &CyclesOptions, root: Option<&Path>) -> IcpCli {
    let icp = IcpCli::new(&options.icp, None, Some(options.network.clone()));
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

fn coverage_status(baseline: Option<&CycleTrackerSample>, requested_since_secs: u64) -> String {
    match baseline {
        Some(sample) if sample.timestamp_secs <= requested_since_secs => "covered".to_string(),
        Some(_) => "partial".to_string(),
        None => "none".to_string(),
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
    let root = resolve_cycles_icp_root().ok_or_else(|| {
        CyclesCommandError::InstallState("could not resolve ICP root".to_string())
    })?;
    resolve_installed_deployment_from_root(
        &InstalledDeploymentRequest {
            deployment: options.deployment.clone(),
            network: options.network.clone(),
            icp: options.icp.clone(),
            detect_lost_local_root: false,
        },
        &root,
    )
    .map_err(super::cycles_installed_deployment_error)
}

fn resolve_cycles_icp_root() -> Option<PathBuf> {
    resolve_current_canic_icp_root().ok()
}

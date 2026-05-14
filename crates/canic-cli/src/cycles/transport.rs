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
    support::registry_tree::registry_rows,
};
use canic_host::{
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
    installed_fleet::{
        InstalledFleetError, InstalledFleetRequest, InstalledFleetResolution,
        resolve_installed_fleet_from_root,
    },
    registry::RegistryEntry,
    response_parse::parse_cycle_balance_response,
};
use std::{
    path::PathBuf,
    sync::Arc,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

const TOPUP_EVENTS_LIMIT: u64 = 1_000;

pub fn cycles_report(options: &CyclesOptions) -> Result<CyclesReport, CyclesCommandError> {
    let registry = load_registry(options)?;
    let generated_at_secs = current_unix_seconds();
    let requested_since_secs = generated_at_secs.saturating_sub(options.since_seconds);
    let canisters =
        collect_cycle_tracker_reports(options, &registry, requested_since_secs, generated_at_secs);

    Ok(CyclesReport {
        fleet: options.fleet.clone(),
        network: options.network.clone(),
        since_seconds: options.since_seconds,
        generated_at_secs,
        canisters,
    })
}

fn load_registry(options: &CyclesOptions) -> Result<Vec<RegistryEntry>, CyclesCommandError> {
    Ok(resolve_cycles_fleet(options)?.registry.entries)
}

fn collect_cycle_tracker_reports(
    options: &CyclesOptions,
    registry: &[RegistryEntry],
    requested_since_secs: u64,
    generated_at_secs: u64,
) -> Vec<CyclesCanisterReport> {
    let query = Arc::new(options.clone());
    let mut handles = Vec::new();
    let rows = registry_rows(registry);
    for row in rows {
        let entry = row.entry.clone();
        let tree_prefix = row.tree_prefix;
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

    handles
        .into_iter()
        .filter_map(|handle| handle.join().ok())
        .collect()
}

fn cycle_tracker_report(
    options: &CyclesOptions,
    entry: &RegistryEntry,
    tree_prefix: String,
    requested_since_secs: u64,
    generated_at_secs: u64,
) -> CyclesCanisterReport {
    let live_cycles = query_live_cycle_balance(options, &entry.pid);
    let result = query_cycle_tracker(options, &entry.pid);
    match result {
        Ok(page) => summarize_cycle_tracker(
            entry,
            page,
            tree_prefix,
            requested_since_secs,
            generated_at_secs,
            live_cycles,
            query_topup_summary(options, &entry.pid, requested_since_secs)
                .ok()
                .flatten(),
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
    topups: Option<CyclesTopupSummary>,
) -> CyclesCanisterReport {
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
        latest_timestamp_secs: live_cycles
            .map(|_| generated_at_secs)
            .or_else(|| latest.as_ref().map(|sample| sample.timestamp_secs)),
        latest_cycles: live_cycles.or_else(|| latest.as_ref().map(|sample| sample.cycles)),
        baseline_timestamp_secs: baseline.as_ref().map(|sample| sample.timestamp_secs),
        baseline_cycles: baseline.as_ref().map(|sample| sample.cycles),
        delta_cycles: delta,
        rate_cycles_per_hour,
        topups,
        error: None,
    }
}

fn query_live_cycle_balance(options: &CyclesOptions, canister_id: &str) -> Option<u128> {
    let mut icp = IcpCli::new(&options.icp, None, Some(options.network.clone()));
    if let Some(root) = resolve_cycles_icp_root() {
        icp = icp.with_cwd(root);
    }
    icp.canister_call_output(
        canister_id,
        canic_core::protocol::CANIC_CYCLE_BALANCE,
        Some("json"),
    )
    .ok()
    .and_then(|output| parse_cycle_balance_response(&output))
}

fn query_topup_summary(
    options: &CyclesOptions,
    canister_id: &str,
    requested_since_secs: u64,
) -> Result<Option<CyclesTopupSummary>, String> {
    let mut page = query_topup_event_page(options, canister_id, 0, TOPUP_EVENTS_LIMIT)?;
    if page.total > TOPUP_EVENTS_LIMIT {
        let offset = page.total.saturating_sub(TOPUP_EVENTS_LIMIT);
        page = query_topup_event_page(options, canister_id, offset, TOPUP_EVENTS_LIMIT)?;
    }
    let summary = topup_summary_from_events(&page.entries, requested_since_secs);

    Ok((!summary.is_empty()).then_some(summary))
}

pub(super) fn topup_summary_from_events(
    entries: &[CycleTopupEventSample],
    requested_since_secs: u64,
) -> CyclesTopupSummary {
    let mut summary = CyclesTopupSummary::default();
    for entry in entries {
        if entry.timestamp_secs < requested_since_secs {
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

fn query_topup_event_page(
    options: &CyclesOptions,
    canister_id: &str,
    offset: u64,
    limit: u64,
) -> Result<crate::cycles::model::CycleTopupEventPage, String> {
    let arg = format!("(record {{ offset = {offset} : nat64; limit = {limit} : nat64 }})");
    let mut icp = IcpCli::new(&options.icp, None, Some(options.network.clone()));
    if let Some(root) = resolve_cycles_icp_root() {
        icp = icp.with_cwd(root);
    }
    let output = icp
        .canister_query_arg_output(
            canister_id,
            canic_core::protocol::CANIC_CYCLE_TOPUPS,
            &arg,
            Some("json"),
        )
        .map_err(|err| err.to_string())?;

    parse_topup_event_page(&output)
        .or_else(|| parse_topup_event_page_text(&output))
        .ok_or_else(|| "could not parse canic_cycle_topups response".to_string())
}

fn query_cycle_tracker(
    options: &CyclesOptions,
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
    options: &CyclesOptions,
    canister_id: &str,
    offset: u64,
    limit: u64,
) -> Result<CycleTrackerPage, String> {
    let arg = format!("(record {{ offset = {offset} : nat64; limit = {limit} : nat64 }})");
    let mut icp = IcpCli::new(&options.icp, None, Some(options.network.clone()));
    if let Some(root) = resolve_cycles_icp_root() {
        icp = icp.with_cwd(root);
    }
    let output = icp
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

fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn resolve_cycles_fleet(
    options: &CyclesOptions,
) -> Result<InstalledFleetResolution, CyclesCommandError> {
    let root = resolve_cycles_icp_root().ok_or_else(|| {
        CyclesCommandError::InstallState("could not resolve ICP root".to_string())
    })?;
    resolve_installed_fleet_from_root(
        &InstalledFleetRequest {
            fleet: options.fleet.clone(),
            network: options.network.clone(),
            icp: options.icp.clone(),
            detect_lost_local_root: false,
        },
        &root,
    )
    .map_err(cycles_installed_fleet_error)
}

fn resolve_cycles_icp_root() -> Option<PathBuf> {
    resolve_current_canic_icp_root(None).ok()
}

fn cycles_installed_fleet_error(error: InstalledFleetError) -> CyclesCommandError {
    match error {
        InstalledFleetError::NoInstalledFleet { network, fleet } => {
            CyclesCommandError::NoInstalledFleet { network, fleet }
        }
        InstalledFleetError::InstallState(error) => CyclesCommandError::InstallState(error),
        InstalledFleetError::ReplicaQuery(error) => CyclesCommandError::ReplicaQuery(error),
        InstalledFleetError::IcpFailed { command, stderr } => {
            CyclesCommandError::IcpFailed { command, stderr }
        }
        InstalledFleetError::LostLocalFleet { root, .. } => {
            CyclesCommandError::ReplicaQuery(format!("root canister {root} is not present"))
        }
        InstalledFleetError::Registry(error) => CyclesCommandError::Registry(error),
        InstalledFleetError::Io(error) => CyclesCommandError::Io(error),
    }
}

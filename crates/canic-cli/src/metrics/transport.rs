use crate::metrics::{
    CANIC_METRICS_METHOD, MetricsCommandError,
    model::{MetricEntry, MetricsCanisterReport, MetricsReport},
    options::MetricsOptions,
    parse::parse_metrics_page,
};
use canic_host::{
    icp::IcpCli,
    installed_fleet::{
        InstalledFleetError, InstalledFleetRequest, InstalledFleetResolution,
        resolve_installed_fleet,
    },
    registry::RegistryEntry,
};
use std::{sync::Arc, thread};

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
    let mut registry = resolve_metrics_fleet(options)?.registry.entries;
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

pub(super) fn metrics_error_report(entry: &RegistryEntry, error: &str) -> MetricsCanisterReport {
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

fn resolve_metrics_fleet(
    options: &MetricsOptions,
) -> Result<InstalledFleetResolution, MetricsCommandError> {
    resolve_installed_fleet(&InstalledFleetRequest {
        fleet: options.fleet.clone(),
        network: options.network.clone(),
        icp: options.icp.clone(),
        detect_lost_local_root: false,
    })
    .map_err(metrics_installed_fleet_error)
}

fn metrics_installed_fleet_error(error: InstalledFleetError) -> MetricsCommandError {
    match error {
        InstalledFleetError::NoInstalledFleet { network, fleet } => {
            MetricsCommandError::NoInstalledFleet { network, fleet }
        }
        InstalledFleetError::InstallState(error) => MetricsCommandError::InstallState(error),
        InstalledFleetError::ReplicaQuery(error) => MetricsCommandError::ReplicaQuery(error),
        InstalledFleetError::IcpFailed { command, stderr } => {
            MetricsCommandError::IcpFailed { command, stderr }
        }
        InstalledFleetError::LostLocalFleet { root, .. } => {
            MetricsCommandError::ReplicaQuery(format!("root canister {root} is not present"))
        }
        InstalledFleetError::Registry(error) => MetricsCommandError::Registry(error),
        InstalledFleetError::Io(error) => MetricsCommandError::Io(error),
    }
}

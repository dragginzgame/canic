use crate::metrics::{
    CANIC_METRICS_METHOD, MetricsCommandError,
    model::{
        MetricEntry, MetricValue, MetricsCanisterReport, MetricsCanisterStatus, MetricsKind,
        MetricsReport,
    },
    options::MetricsOptions,
    parse::parse_metrics_page,
};
use crate::support::candid::registry_entry_candid_path;
use canic_host::{
    icp::{IcpCli, IcpDiagnostic, classify_icp_diagnostic},
    icp_config::resolve_current_canic_icp_root,
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest, InstalledDeploymentResolution,
        resolve_installed_deployment_from_root,
    },
    registry::RegistryEntry,
};
use std::{path::PathBuf, sync::Arc, thread};

const METRICS_UNAVAILABLE_HINT: &str =
    "canic_metrics unavailable; check deployed Wasm and metrics profile";
const METRICS_EMPTY_HINT: &str =
    "no metrics rows; check whether this tier is enabled by the deployed role profile";
const METRICS_NONZERO_EMPTY_HINT: &str =
    "no nonzero metrics rows; rerun without --nonzero or check the deployed role profile";
const METRICS_WORKER_PANIC: &str = "metrics query worker panicked";

pub(super) fn metrics_report(
    options: &MetricsOptions,
) -> Result<MetricsReport, MetricsCommandError> {
    let registry = load_registry(options)?;
    let canisters = collect_metrics_reports(options, &registry);

    Ok(MetricsReport {
        deployment: options.deployment.clone(),
        network: options.network.clone(),
        kind: options.kind,
        canisters,
    })
}

fn load_registry(options: &MetricsOptions) -> Result<Vec<RegistryEntry>, MetricsCommandError> {
    let mut registry = resolve_metrics_deployment(options)?.registry.entries;
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
        let worker_entry = entry.clone();
        let query = Arc::clone(&query);
        handles.push((
            worker_entry,
            thread::spawn(move || metrics_canister_report(&query, &entry)),
        ));
    }

    collect_metrics_worker_reports(handles)
}

fn collect_metrics_worker_reports(
    handles: Vec<(RegistryEntry, thread::JoinHandle<MetricsCanisterReport>)>,
) -> Vec<MetricsCanisterReport> {
    handles
        .into_iter()
        .map(|(entry, handle)| {
            handle
                .join()
                .unwrap_or_else(|_| metrics_error_report(&entry, METRICS_WORKER_PANIC))
        })
        .collect()
}

fn metrics_canister_report(
    options: &MetricsOptions,
    entry: &RegistryEntry,
) -> MetricsCanisterReport {
    match query_metrics(options, entry) {
        Ok(mut entries) => {
            if options.nonzero {
                entries.retain(|entry| !metric_value_is_zero(&entry.value));
            }
            if entries.is_empty() {
                return metrics_empty_report(entry, options.nonzero);
            }
            MetricsCanisterReport {
                role: entry.role.clone().unwrap_or_else(|| "-".to_string()),
                canister_id: entry.pid.clone(),
                status: MetricsCanisterStatus::Ok,
                entries,
                error: None,
            }
        }
        Err(error) => metrics_error_report(entry, &error),
    }
}

fn metrics_empty_report(entry: &RegistryEntry, nonzero: bool) -> MetricsCanisterReport {
    MetricsCanisterReport {
        role: entry.role.clone().unwrap_or_else(|| "-".to_string()),
        canister_id: entry.pid.clone(),
        status: MetricsCanisterStatus::Empty,
        entries: Vec::new(),
        error: Some(
            if nonzero {
                METRICS_NONZERO_EMPTY_HINT
            } else {
                METRICS_EMPTY_HINT
            }
            .to_string(),
        ),
    }
}

fn metrics_error_report(entry: &RegistryEntry, error: &str) -> MetricsCanisterReport {
    let (status, error) = if matches!(
        classify_icp_diagnostic(error),
        Some(IcpDiagnostic::MethodMissing)
    ) {
        (MetricsCanisterStatus::Unavailable, METRICS_UNAVAILABLE_HINT)
    } else {
        (
            MetricsCanisterStatus::Error,
            error.lines().next().unwrap_or(error),
        )
    };

    MetricsCanisterReport {
        role: entry.role.clone().unwrap_or_else(|| "-".to_string()),
        canister_id: entry.pid.clone(),
        status,
        entries: Vec::new(),
        error: Some(error.to_string()),
    }
}

const fn metric_value_is_zero(value: &MetricValue) -> bool {
    match value {
        MetricValue::Count { count } => *count == 0,
        MetricValue::CountAndU64 { count, value_u64 } => *count == 0 && *value_u64 == 0,
        MetricValue::U128 { value } => *value == 0,
    }
}

const fn metrics_kind_candid_variant(kind: MetricsKind) -> &'static str {
    match kind {
        MetricsKind::Core => "Core",
        MetricsKind::Placement => "Placement",
        MetricsKind::Platform => "Platform",
        MetricsKind::Runtime => "Runtime",
        MetricsKind::Security => "Security",
        MetricsKind::Storage => "Storage",
    }
}

fn query_metrics(
    options: &MetricsOptions,
    entry: &RegistryEntry,
) -> Result<Vec<MetricEntry>, String> {
    let arg = format!(
        "(variant {{ {} }}, record {{ offset = 0 : nat64; limit = {} : nat64 }})",
        metrics_kind_candid_variant(options.kind),
        options.limit
    );
    let mut icp = IcpCli::new(&options.icp, None, Some(options.network.clone()));
    let root = resolve_metrics_icp_root();
    let candid_path = registry_entry_candid_path(root.as_deref(), &options.network, entry);
    if let Some(root) = root {
        icp = icp.with_cwd(root);
    }
    let output = icp
        .canister_query_arg_output_with_candid(
            &entry.pid,
            CANIC_METRICS_METHOD,
            &arg,
            Some("json"),
            candid_path.as_deref(),
        )
        .map_err(|err| err.to_string())?;

    parse_metrics_page(&output)
        .map_err(|error| format!("could not parse canic_metrics response: {error}"))
}

fn resolve_metrics_deployment(
    options: &MetricsOptions,
) -> Result<InstalledDeploymentResolution, MetricsCommandError> {
    let root = resolve_current_canic_icp_root().map_err(MetricsCommandError::IcpRoot)?;
    resolve_installed_deployment_from_root(
        &InstalledDeploymentRequest {
            deployment: options.deployment.clone(),
            network: options.network.clone(),
            icp: options.icp.clone(),
            detect_lost_local_root: false,
        },
        &root,
    )
    .map_err(metrics_installed_deployment_error)
}

fn resolve_metrics_icp_root() -> Option<PathBuf> {
    resolve_current_canic_icp_root().ok()
}

fn metrics_installed_deployment_error(error: InstalledDeploymentError) -> MetricsCommandError {
    match error {
        InstalledDeploymentError::NoInstalledDeployment {
            network,
            deployment,
        } => MetricsCommandError::NoInstalledDeployment {
            network,
            deployment,
        },
        InstalledDeploymentError::InstallState(error) => MetricsCommandError::InstallState(error),
        InstalledDeploymentError::ReplicaQuery(error) => MetricsCommandError::ReplicaQuery(error),
        InstalledDeploymentError::Icp(error) => MetricsCommandError::Icp(error),
        InstalledDeploymentError::LostLocalDeployment { root, .. } => {
            MetricsCommandError::LostLocalRoot { root }
        }
        InstalledDeploymentError::Registry(error) => MetricsCommandError::Registry(error),
        InstalledDeploymentError::Io(error) => MetricsCommandError::Io(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_entry() -> RegistryEntry {
        RegistryEntry {
            pid: "aaaaa-aa".to_string(),
            role: Some("wasm_store".to_string()),
            parent_pid: None,
            module_hash: None,
        }
    }

    // Ensure method-missing responses do not stretch the table with raw ICP output.
    #[test]
    fn shortens_metrics_unavailable_errors() {
        let report = metrics_error_report(
            &registry_entry(),
            "icp command failed\nCanister has no query method 'canic_metrics'.",
        );

        assert_eq!(report.status, MetricsCanisterStatus::Unavailable);
        assert_eq!(
            serde_json::to_value(&report).expect("serialize metrics report")["status"],
            "unavailable"
        );
        assert_eq!(report.error.as_deref(), Some(METRICS_UNAVAILABLE_HINT));
    }

    // Ensure empty successful metric tiers point operators at profile/deployed-Wasm checks.
    #[test]
    fn empty_metrics_reports_carry_profile_hint() {
        let report = metrics_empty_report(&registry_entry(), false);

        assert_eq!(report.status, MetricsCanisterStatus::Empty);
        assert_eq!(
            serde_json::to_value(&report).expect("serialize metrics report")["status"],
            "empty"
        );
        assert_eq!(report.error.as_deref(), Some(METRICS_EMPTY_HINT));

        let filtered = metrics_empty_report(&registry_entry(), true);
        assert_eq!(filtered.status, MetricsCanisterStatus::Empty);
        assert_eq!(filtered.error.as_deref(), Some(METRICS_NONZERO_EMPTY_HINT));
    }

    // Ensure zero filtering treats every payload shape consistently.
    #[test]
    fn detects_zero_metric_values() {
        assert!(metric_value_is_zero(&MetricValue::Count { count: 0 }));
        assert!(metric_value_is_zero(&MetricValue::CountAndU64 {
            count: 0,
            value_u64: 0
        }));
        assert!(!metric_value_is_zero(&MetricValue::U128 { value: 1 }));
    }

    // Ensure transport preserves the Candid metric kind vocabulary.
    #[test]
    fn maps_metric_kind_to_candid_variant() {
        assert_eq!(
            metrics_kind_candid_variant(MetricsKind::Security),
            "Security"
        );
    }

    #[test]
    fn panicked_metrics_worker_becomes_an_explicit_canister_error() {
        let entry = registry_entry();
        let reports = collect_metrics_worker_reports(vec![(
            entry.clone(),
            thread::spawn(|| panic!("simulated metrics worker panic")),
        )]);

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].canister_id, entry.pid);
        assert_eq!(reports[0].status, MetricsCanisterStatus::Error);
        assert_eq!(reports[0].error.as_deref(), Some(METRICS_WORKER_PANIC));
    }
}

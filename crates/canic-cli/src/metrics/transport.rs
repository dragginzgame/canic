use crate::metrics::{
    CANIC_METRICS_METHOD, MetricsCommandError,
    model::{MetricEntry, MetricValue, MetricsCanisterReport, MetricsKind, MetricsReport},
    options::MetricsOptions,
    parse::parse_metrics_page,
};
use canic_host::{
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest, InstalledDeploymentResolution,
        resolve_installed_deployment_from_root,
    },
    registry::RegistryEntry,
};
use std::{path::PathBuf, sync::Arc, thread};

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
                entries.retain(|entry| !metric_value_is_zero(&entry.value));
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

fn query_metrics(options: &MetricsOptions, canister_id: &str) -> Result<Vec<MetricEntry>, String> {
    let arg = format!(
        "(variant {{ {} }}, record {{ offset = 0 : nat64; limit = {} : nat64 }})",
        metrics_kind_candid_variant(options.kind),
        options.limit
    );
    let mut icp = IcpCli::new(&options.icp, None, Some(options.network.clone()));
    if let Some(root) = resolve_metrics_icp_root() {
        icp = icp.with_cwd(root);
    }
    let output = icp
        .canister_query_arg_output(canister_id, CANIC_METRICS_METHOD, &arg, Some("json"))
        .map_err(|err| err.to_string())?;

    parse_metrics_page(&output).ok_or_else(|| "could not parse canic_metrics response".to_string())
}

fn resolve_metrics_deployment(
    options: &MetricsOptions,
) -> Result<InstalledDeploymentResolution, MetricsCommandError> {
    let root = resolve_metrics_icp_root().ok_or_else(|| {
        MetricsCommandError::InstallState("could not resolve ICP root".to_string())
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
        InstalledDeploymentError::IcpFailed { command, stderr } => {
            MetricsCommandError::IcpFailed { command, stderr }
        }
        InstalledDeploymentError::LostLocalDeployment { root, .. } => {
            MetricsCommandError::ReplicaQuery(format!("root canister {root} is not present"))
        }
        InstalledDeploymentError::Registry(error) => MetricsCommandError::Registry(error),
        InstalledDeploymentError::Io(error) => MetricsCommandError::Io(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

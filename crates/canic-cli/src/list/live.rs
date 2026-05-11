use super::{
    ListCommandError,
    options::ListOptions,
    parse::{parse_canic_metadata_version_response, parse_cycle_balance_response},
    render::ReadyStatus,
    state_network,
    tree::visible_entries,
};
use canic_backup::discovery::{RegistryEntry, parse_registry_entries};
use canic_host::{
    format::{byte_size, cycles_tc},
    icp::{IcpCli, IcpCommandError},
    install_root::{InstallState, read_named_fleet_install_state},
    release_set::icp_root,
    replica_query,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::Arc,
    thread,
};

use super::options::ListSource;

pub(super) fn load_registry_entries(
    options: &ListOptions,
) -> Result<Vec<RegistryEntry>, ListCommandError> {
    let registry_json = match options.source {
        ListSource::RootRegistry => {
            let root = resolve_root_canister(options)?;
            call_subnet_registry(options, &root)?
        }
        ListSource::Config => {
            unreachable!("config source does not use registry entries")
        }
    };

    parse_registry_entries(&registry_json).map_err(ListCommandError::from)
}

pub(super) fn resolve_tree_anchor(options: &ListOptions) -> Option<String> {
    options.subtree.clone()
}

pub(super) fn list_ready_statuses(
    options: &ListOptions,
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, ReadyStatus>, ListCommandError> {
    if replica_query::should_use_local_replica_query(options.network.as_deref()) {
        return local_ready_statuses(options, registry, canister);
    }

    let mut statuses = BTreeMap::new();
    for entry in visible_entries(registry, canister)? {
        statuses.insert(entry.pid.clone(), check_ready_status(options, &entry.pid)?);
    }
    Ok(statuses)
}

pub(super) fn list_cycle_balances(
    options: &ListOptions,
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, String>, ListCommandError> {
    let icp = options.icp.clone();
    let network = options.network.clone();
    collect_visible_optional_values(registry, canister, move |pid| {
        query_cycle_balance_endpoint(&icp, network.clone(), &pid).map(cycles_tc)
    })
}

pub(super) fn list_canic_versions(
    options: &ListOptions,
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, String>, ListCommandError> {
    let icp = options.icp.clone();
    let network = options.network.clone();
    collect_visible_optional_values(registry, canister, move |pid| {
        query_canic_metadata_version(&icp, network.clone(), &pid)
    })
}

pub(super) fn list_module_hashes(
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, String>, ListCommandError> {
    Ok(visible_entries(registry, canister)?
        .into_iter()
        .filter_map(|entry| {
            entry
                .module_hash
                .as_ref()
                .map(|hash| (entry.pid.clone(), hash.clone()))
        })
        .collect())
}

pub(super) fn resolve_wasm_sizes(
    options: &ListOptions,
    registry: &[RegistryEntry],
) -> BTreeMap<String, String> {
    let Some(root) = resolve_icp_artifact_root(options) else {
        return BTreeMap::new();
    };
    let network = state_network(options);
    registry
        .iter()
        .filter_map(|entry| entry.role.as_deref())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(|role| {
            let path = root
                .join(".icp")
                .join(&network)
                .join("canisters")
                .join(role)
                .join(format!("{role}.wasm.gz"));
            fs::metadata(path)
                .ok()
                .map(|metadata| (role.to_string(), byte_size(metadata.len())))
        })
        .collect()
}

fn check_ready_status(
    options: &ListOptions,
    canister: &str,
) -> Result<ReadyStatus, ListCommandError> {
    let Ok(output) = IcpCli::new(&options.icp, None, options.network.clone()).canister_call_output(
        canister,
        "canic_ready",
        Some("json"),
    ) else {
        return Ok(ReadyStatus::Error);
    };
    let data = serde_json::from_str::<serde_json::Value>(&output)?;
    Ok(if replica_query::parse_ready_json_value(&data) {
        ReadyStatus::Ready
    } else {
        ReadyStatus::NotReady
    })
}

fn local_ready_statuses(
    options: &ListOptions,
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, ReadyStatus>, ListCommandError> {
    let network = options.network.clone();
    collect_visible_values(
        registry,
        canister,
        move |pid| match replica_query::query_ready(network.as_deref(), &pid) {
            Ok(true) => ReadyStatus::Ready,
            Ok(false) => ReadyStatus::NotReady,
            Err(_) => ReadyStatus::Error,
        },
    )
}

fn collect_visible_optional_values<T, F>(
    registry: &[RegistryEntry],
    canister: Option<&str>,
    query: F,
) -> Result<BTreeMap<String, T>, ListCommandError>
where
    T: Send + 'static,
    F: Fn(String) -> Option<T> + Send + Sync + 'static,
{
    let values = collect_visible_values(registry, canister, query)?;
    Ok(values
        .into_iter()
        .filter_map(|(pid, value)| value.map(|value| (pid, value)))
        .collect())
}

fn collect_visible_values<T, F>(
    registry: &[RegistryEntry],
    canister: Option<&str>,
    query: F,
) -> Result<BTreeMap<String, T>, ListCommandError>
where
    T: Send + 'static,
    F: Fn(String) -> T + Send + Sync + 'static,
{
    let query = Arc::new(query);
    let mut handles = Vec::new();
    for entry in visible_entries(registry, canister)? {
        let pid = entry.pid.clone();
        let query = Arc::clone(&query);
        handles.push(thread::spawn(move || {
            let value = query(pid.clone());
            (pid, value)
        }));
    }

    Ok(handles
        .into_iter()
        .filter_map(|handle| handle.join().ok())
        .collect())
}

fn query_cycle_balance_endpoint(
    icp: &str,
    network: Option<String>,
    canister: &str,
) -> Option<u128> {
    IcpCli::new(icp, None, network)
        .canister_call_output(canister, canic_core::protocol::CANIC_CYCLE_BALANCE, None)
        .ok()
        .and_then(|output| parse_cycle_balance_response(&output))
}

fn query_canic_metadata_version(
    icp: &str,
    network: Option<String>,
    canister: &str,
) -> Option<String> {
    IcpCli::new(icp, None, network)
        .canister_call_output(canister, canic_core::protocol::CANIC_METADATA, None)
        .ok()
        .and_then(|output| parse_canic_metadata_version_response(&output))
}

fn resolve_root_canister(options: &ListOptions) -> Result<String, ListCommandError> {
    if let Some(state) = read_selected_install_state(options)
        .map_err(|err| ListCommandError::InstallState(err.to_string()))?
    {
        return Ok(state.root_canister_id);
    }

    Err(ListCommandError::NoInstalledFleet {
        network: state_network(options),
        fleet: options.fleet.clone(),
    })
}

fn read_selected_install_state(
    options: &ListOptions,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    read_named_fleet_install_state(&state_network(options), &options.fleet)
}

fn call_subnet_registry(options: &ListOptions, root: &str) -> Result<String, ListCommandError> {
    if replica_query::should_use_local_replica_query(options.network.as_deref()) {
        return replica_query::query_subnet_registry_json(options.network.as_deref(), root)
            .map_err(|err| list_replica_query_error(options, root, err.to_string()));
    }

    IcpCli::new(&options.icp, None, options.network.clone())
        .canister_call_output(root, "canic_subnet_registry", Some("json"))
        .map_err(list_icp_error)
        .map_err(add_root_registry_hint)
}

fn resolve_icp_artifact_root(options: &ListOptions) -> Option<PathBuf> {
    if let Ok(Some(state)) = read_selected_install_state(options) {
        return Some(PathBuf::from(state.icp_root));
    }
    icp_root().ok()
}

fn list_replica_query_error(options: &ListOptions, root: &str, error: String) -> ListCommandError {
    if is_canister_not_found_error(&error)
        && let Ok(Some(state)) = read_selected_install_state(options)
        && state.root_canister_id == root
    {
        return ListCommandError::StaleLocalFleet {
            fleet: state.fleet,
            network: state_network(options),
            root: root.to_string(),
        };
    }

    ListCommandError::ReplicaQuery(error)
}

fn is_canister_not_found_error(error: &str) -> bool {
    error.contains("Canister ") && error.contains(" not found")
}

fn add_root_registry_hint(error: ListCommandError) -> ListCommandError {
    let ListCommandError::IcpFailed { command, stderr } = error else {
        return error;
    };

    let Some(hint) = root_registry_hint(&stderr) else {
        return ListCommandError::IcpFailed { command, stderr };
    };

    ListCommandError::IcpFailed {
        command,
        stderr: format!("{stderr}\nHint: {hint}\n"),
    }
}

fn list_icp_error(error: IcpCommandError) -> ListCommandError {
    match error {
        IcpCommandError::Io(err) => ListCommandError::Io(err),
        IcpCommandError::Failed { command, stderr } => {
            ListCommandError::IcpFailed { command, stderr }
        }
        IcpCommandError::SnapshotIdUnavailable { output } => ListCommandError::IcpFailed {
            command: "icp canister snapshot create".to_string(),
            stderr: output,
        },
    }
}

fn root_registry_hint(stderr: &str) -> Option<&'static str> {
    if stderr.contains("Cannot find canister id") {
        return Some(
            "no root canister id exists for this fleet. Use `canic config <name>` for the selected fleet config, or run `canic install <name>` before querying the root registry.",
        );
    }

    if stderr.contains("contains no Wasm module") || stderr.contains("wasm-module-not-found") {
        return Some(
            "the root canister id exists but no Canic root code is installed. Run `canic install <name>`, then use `canic list <name>`.",
        );
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure empty-root command errors explain root registry setup.
    #[test]
    fn root_registry_hint_explains_empty_root_canister() {
        let hint = root_registry_hint("the canister contains no Wasm module")
            .expect("empty wasm hint should be available");

        assert!(hint.contains("canic install"));
        assert!(hint.contains("no Canic root code is installed"));
    }

    // Ensure local replica missing-canister errors are recognized for stale fleet guidance.
    #[test]
    fn detects_local_canister_not_found_error() {
        assert!(is_canister_not_found_error(
            "local replica rejected query: code=3 message=Canister uxrrr-q7777-77774-qaaaq-cai not found"
        ));
        assert!(!is_canister_not_found_error(
            "local replica rejected query: code=5 message=some other failure"
        ));
    }
}

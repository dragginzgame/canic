use crate::{
    args::{local_network, print_help_or_version},
    version_text,
};
mod config;
mod options;
mod render;

use canic_backup::discovery::{DiscoveryError, RegistryEntry, parse_registry_entries};
use canic_host::{
    format::{byte_size, cycles_tc},
    icp::{IcpCli, IcpCommandError},
    install_root::{InstallState, read_named_fleet_install_state},
    release_set::icp_root,
    replica_query,
};
use config::{load_config_role_rows, missing_config_roles};
use options::{ListOptions, ListSource, config_usage, usage};
use render::RegistryColumnData;
#[cfg(not(test))]
use render::render_config_output;
#[cfg(test)]
use render::{
    CANIC_HEADER, CANISTER_HEADER, CYCLES_HEADER, ConfigRoleRow, READY_HEADER, ROLE_HEADER,
    WASM_HEADER, render_config_output, render_registry_separator, render_registry_table_row,
    render_registry_tree,
};
use render::{ListTitle, ReadyStatus, render_list_output, visible_entries};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs,
    path::PathBuf,
    thread,
};
use thiserror::Error as ThisError;

///
/// ListCommandError
///

#[derive(Debug, ThisError)]
pub enum ListCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("registry JSON did not contain the requested canister {0}")]
    CanisterNotInRegistry(String),

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error(
        "fleet {fleet} points to root {root}, but that canister is not present on network {network}. Local replica state was probably restarted or reset. Run `canic install {fleet}` to recreate it."
    )]
    StaleLocalFleet {
        fleet: String,
        network: String,
        root: String,
    },

    #[error("failed to read canic fleet state: {0}")]
    InstallState(String),

    #[error(
        "fleet {fleet} is not installed on network {network}; run `canic install {fleet}` to deploy it or `canic config {fleet}` to inspect its config"
    )]
    NoInstalledFleet { network: String, fleet: String },

    #[error("fleet {0} is not declared by any config under fleets; run `canic fleet list`")]
    UnknownFleet(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
}

/// Run the deployed canister listing command.
pub fn run<I>(args: I) -> Result<(), ListCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = ListOptions::parse_list(args)?;
    let registry = load_registry_entries(&options)?;
    let anchor = resolve_tree_anchor(&options);
    let readiness = list_ready_statuses(&options, &registry, anchor.as_deref())?;
    let canic_versions = list_canic_versions(&options, &registry, anchor.as_deref())?;
    let wasm_sizes = resolve_wasm_sizes(&options, &registry);
    let cycles = list_cycle_balances(&options, &registry, anchor.as_deref())?;
    let missing_roles = missing_config_roles(&options, &registry);
    let title = list_title(&options);
    let columns = RegistryColumnData {
        readiness: &readiness,
        canic_versions: &canic_versions,
        wasm_sizes: &wasm_sizes,
        cycles: &cycles,
    };
    println!(
        "{}",
        render_list_output(
            &title,
            &registry,
            anchor.as_deref(),
            &columns,
            &missing_roles
        )?
    );
    Ok(())
}

/// Run the selected fleet config listing command.
pub fn run_config<I>(args: I) -> Result<(), ListCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, config_usage, version_text()) {
        return Ok(());
    }

    let options = ListOptions::parse_config(args)?;
    let title = list_title(&options);
    let rows = load_config_role_rows(&options)?;
    println!("{}", render_config_output(&title, &rows, options.verbose));
    Ok(())
}

fn list_title(options: &ListOptions) -> ListTitle {
    ListTitle {
        fleet: options.fleet.clone(),
        network: state_network(options),
    }
}

fn list_ready_statuses(
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
    let mut handles = Vec::new();
    for entry in visible_entries(registry, canister)? {
        let pid = entry.pid.clone();
        let network = network.clone();
        handles.push(thread::spawn(move || {
            let status = match replica_query::query_ready(network.as_deref(), &pid) {
                Ok(true) => ReadyStatus::Ready,
                Ok(false) => ReadyStatus::NotReady,
                Err(_) => ReadyStatus::Error,
            };
            (pid, status)
        }));
    }

    Ok(handles
        .into_iter()
        .filter_map(|handle| handle.join().ok())
        .collect())
}

fn list_cycle_balances(
    options: &ListOptions,
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, String>, ListCommandError> {
    let icp = options.icp.clone();
    let network = options.network.clone();
    let mut handles = Vec::new();
    for entry in visible_entries(registry, canister)? {
        let pid = entry.pid.clone();
        let icp = icp.clone();
        let network = network.clone();
        handles.push(thread::spawn(move || {
            let label = query_cycle_balance_endpoint(&icp, network, &pid).map(cycles_tc);
            (pid, label)
        }));
    }

    Ok(handles
        .into_iter()
        .filter_map(|handle| {
            let (pid, label) = handle.join().ok()?;
            label.map(|label| (pid, label))
        })
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

fn list_canic_versions(
    options: &ListOptions,
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<BTreeMap<String, String>, ListCommandError> {
    let icp = options.icp.clone();
    let network = options.network.clone();
    let mut handles = Vec::new();
    for entry in visible_entries(registry, canister)? {
        let pid = entry.pid.clone();
        let icp = icp.clone();
        let network = network.clone();
        handles.push(thread::spawn(move || {
            let version = query_canic_metadata_version(&icp, network, &pid);
            (pid, version)
        }));
    }

    Ok(handles
        .into_iter()
        .filter_map(|handle| {
            let (pid, version) = handle.join().ok()?;
            version.map(|version| (pid, version))
        })
        .collect())
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

fn parse_canic_metadata_version_response(output: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(output)
        .ok()
        .and_then(|value| find_string_field(&value, "canic_version"))
        .or_else(|| parse_candid_text_field(output, "canic_version"))
}

fn find_string_field(value: &serde_json::Value, field: &str) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => map
            .get(field)
            .and_then(|value| value.as_str().map(ToString::to_string))
            .or_else(|| {
                map.values()
                    .find_map(|value| find_string_field(value, field))
            }),
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|value| find_string_field(value, field)),
        _ => None,
    }
}

fn parse_candid_text_field(output: &str, field: &str) -> Option<String> {
    let (_, after_field) = output.split_once(field)?;
    let (_, after_eq) = after_field.split_once('=')?;
    let after_quote = after_eq.trim_start().strip_prefix('"')?;
    let (value, _) = after_quote.split_once('"')?;
    Some(value.to_string())
}

fn parse_cycle_balance_response(output: &str) -> Option<u128> {
    output
        .split_once('=')
        .map_or(output, |(_, cycles)| cycles)
        .lines()
        .find_map(parse_leading_integer)
}

fn parse_leading_integer(line: &str) -> Option<u128> {
    let digits = line
        .trim_start_matches(|ch: char| ch == '(' || ch.is_whitespace())
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '_' || *ch == ',')
        .filter(char::is_ascii_digit)
        .collect::<String>();
    (!digits.is_empty())
        .then_some(digits)
        .and_then(|digits| digits.parse::<u128>().ok())
}

fn load_registry_entries(options: &ListOptions) -> Result<Vec<RegistryEntry>, ListCommandError> {
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

fn resolve_tree_anchor(options: &ListOptions) -> Option<String> {
    options.subtree.clone()
}

fn state_network(options: &ListOptions) -> String {
    options.network.clone().unwrap_or_else(local_network)
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

fn resolve_wasm_sizes(
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
    use serde_json::json;

    const ROOT: &str = "aaaaa-aa";
    const APP: &str = "renrk-eyaaa-aaaaa-aaada-cai";
    const MINIMAL: &str = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    const WORKER: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
    // Ensure list options parse live registry queries.
    #[test]
    fn parses_live_list_options() {
        let options = ListOptions::parse_list([
            OsString::from("demo"),
            OsString::from("--subtree"),
            OsString::from(APP),
            OsString::from(crate::args::INTERNAL_NETWORK_OPTION),
            OsString::from("local"),
            OsString::from(crate::args::INTERNAL_ICP_OPTION),
            OsString::from("/bin/icp"),
        ])
        .expect("parse list options");

        assert_eq!(options.source, ListSource::RootRegistry);
        assert_eq!(options.fleet, "demo");
        assert_eq!(options.subtree, Some(APP.to_string()));
        assert_eq!(options.network, Some("local".to_string()));
        assert_eq!(options.icp, "/bin/icp");
        assert!(!options.verbose);
    }

    // Ensure config options parse declared fleet inspection.
    #[test]
    fn parses_config_options() {
        let options = ListOptions::parse_config([
            OsString::from("demo"),
            OsString::from(crate::args::INTERNAL_NETWORK_OPTION),
            OsString::from("local"),
            OsString::from("-v"),
        ])
        .expect("parse config options");

        assert_eq!(options.source, ListSource::Config);
        assert_eq!(options.fleet, "demo");
        assert_eq!(options.subtree, None);
        assert_eq!(options.network, Some("local".to_string()));
        assert_eq!(options.icp, "icp");
        assert!(options.verbose);
    }

    // Ensure config rejects deployed-list-only subtree anchors.
    #[test]
    fn config_rejects_subtree_option() {
        let err = ListOptions::parse_config([
            OsString::from("demo"),
            OsString::from("--subtree"),
            OsString::from("user_hub"),
        ])
        .expect_err("config --subtree should fail");

        assert!(matches!(err, ListCommandError::Usage(_)));
    }

    // Ensure removed list selectors are hard rejected.
    #[test]
    fn list_rejects_removed_root_and_from_options() {
        let root_err = ListOptions::parse_list([
            OsString::from("demo"),
            OsString::from("--root"),
            OsString::from("aaaaa-aa"),
        ])
        .expect_err("list --root should fail");
        let from_err = ListOptions::parse_list([
            OsString::from("demo"),
            OsString::from("--from"),
            OsString::from("user_hub"),
        ])
        .expect_err("list --from should fail");

        assert!(matches!(root_err, ListCommandError::Usage(_)));
        assert!(matches!(from_err, ListCommandError::Usage(_)));
    }

    // Ensure list and config help explain fleet selection and subtree rendering.
    #[test]
    fn list_and_config_usage_explain_fleet_and_subtree_options() {
        let list = usage();
        let config = config_usage();

        assert!(list.contains("List canisters registered by the deployed root"));
        assert!(list.contains("Usage: canic list [OPTIONS] <fleet>"));
        assert!(list.contains("<fleet>"));
        assert!(!list.contains("--fleet <name>"));
        assert!(list.contains("--subtree <name-or-principal>"));
        assert!(!list.contains("--from"));
        assert!(!list.contains("--root"));
        assert!(config.contains("Usage: canic config [OPTIONS] <fleet>"));
        assert!(config.contains("<fleet>"));
        assert!(!config.contains("--fleet <name>"));
        assert!(!config.contains("--subtree"));
        assert!(!config.contains("--from"));
        assert!(config.contains("--verbose"));
        assert!(config.contains("-v"));
        assert!(config.contains("Examples:"));
    }

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

    // Ensure registry entries render as a stable whitespace table.
    #[test]
    fn renders_registry_table() {
        let registry = parse_registry_entries(&registry_json()).expect("parse registry");
        let readiness = readiness_map();
        let empty = BTreeMap::new();
        let columns = RegistryColumnData {
            readiness: &readiness,
            canic_versions: &empty,
            wasm_sizes: &empty,
            cycles: &empty,
        };
        let tree = render_registry_tree(&registry, None, &columns).expect("render tree");
        let widths = [12, 27, 5, 5, 7, 6];

        assert_eq!(
            tree,
            [
                render_registry_table_row(
                    &[
                        ROLE_HEADER,
                        CANISTER_HEADER,
                        READY_HEADER,
                        CANIC_HEADER,
                        WASM_HEADER,
                        CYCLES_HEADER,
                    ],
                    &widths
                ),
                render_registry_separator(&widths),
                render_registry_table_row(&["root", ROOT, "yes", "-", "-", "-"], &widths),
                render_registry_table_row(&["├─ app", APP, "no", "-", "-", "-"], &widths),
                render_registry_table_row(
                    &["│  └─ worker", WORKER, "error", "-", "-", "-"],
                    &widths
                ),
                render_registry_table_row(&["└─ minimal", MINIMAL, "yes", "-", "-", "-"], &widths)
            ]
            .join("\n")
        );
    }

    // Ensure one selected subtree can be rendered without siblings.
    #[test]
    fn renders_selected_subtree() {
        let registry = parse_registry_entries(&registry_json()).expect("parse registry");
        let readiness = readiness_map();
        let empty = BTreeMap::new();
        let columns = RegistryColumnData {
            readiness: &readiness,
            canic_versions: &empty,
            wasm_sizes: &empty,
            cycles: &empty,
        };
        let tree = render_registry_tree(&registry, Some(APP), &columns).expect("render subtree");
        let widths = [9, 27, 5, 5, 7, 6];

        assert_eq!(
            tree,
            [
                render_registry_table_row(
                    &[
                        ROLE_HEADER,
                        CANISTER_HEADER,
                        READY_HEADER,
                        CANIC_HEADER,
                        WASM_HEADER,
                        CYCLES_HEADER,
                    ],
                    &widths
                ),
                render_registry_separator(&widths),
                render_registry_table_row(&["app", APP, "no", "-", "-", "-"], &widths),
                render_registry_table_row(&["└─ worker", WORKER, "error", "-", "-", "-"], &widths)
            ]
            .join("\n")
        );
    }

    // Ensure the full list output names the selected fleet before the tree table.
    #[test]
    fn renders_list_output_with_fleet_title() {
        let registry = parse_registry_entries(&registry_json()).expect("parse registry");
        let title = ListTitle {
            fleet: "demo".to_string(),
            network: "local".to_string(),
        };
        let readiness = readiness_map();
        let empty = BTreeMap::new();
        let columns = RegistryColumnData {
            readiness: &readiness,
            canic_versions: &empty,
            wasm_sizes: &empty,
            cycles: &empty,
        };
        let output = render_list_output(&title, &registry, Some(APP), &columns, &[])
            .expect("render list output");

        assert!(output.starts_with("Fleet: demo (network local)\n\nROLE"));
        assert!(output.contains("CANISTER_ID"));
    }

    #[test]
    fn renders_list_output_with_wasm_size_and_missing_roles() {
        let registry = parse_registry_entries(&registry_json()).expect("parse registry");
        let title = ListTitle {
            fleet: "demo".to_string(),
            network: "local".to_string(),
        };
        let canic_versions = BTreeMap::from([(APP.to_string(), "0.33.6".to_string())]);
        let wasm_sizes = BTreeMap::from([("app".to_string(), "811.20 KiB".to_string())]);
        let cycles = BTreeMap::from([(APP.to_string(), "12.35 TC".to_string())]);
        let readiness = readiness_map();
        let columns = RegistryColumnData {
            readiness: &readiness,
            canic_versions: &canic_versions,
            wasm_sizes: &wasm_sizes,
            cycles: &cycles,
        };
        let output = render_list_output(&title, &registry, None, &columns, &["audit".to_string()])
            .expect("render list output");

        assert!(output.contains("WASM_GZ"));
        assert!(output.contains("CYCLES"));
        assert!(output.contains("0.33.6"));
        assert!(output.contains("811.20 KiB"));
        assert!(output.contains("12.35 TC"));
        assert!(output.contains("Missing roles: audit"));
    }

    // Ensure config-only fleets render their declared roles instead of deployed inventory.
    #[test]
    fn renders_config_output_with_fleet_roles() {
        let title = ListTitle {
            fleet: "test_me".to_string(),
            network: "local".to_string(),
        };
        let rows = vec![
            ConfigRoleRow {
                role: "root".to_string(),
                kind: "root".to_string(),
                capabilities: "-".to_string(),
                auto_create: "-".to_string(),
                topup: "-".to_string(),
                metrics: "root".to_string(),
                details: Vec::new(),
            },
            ConfigRoleRow {
                role: "app".to_string(),
                kind: "singleton".to_string(),
                capabilities: "auth, sharding".to_string(),
                auto_create: "yes".to_string(),
                topup: "4.00 TC @ 10.00 TC".to_string(),
                metrics: "hub".to_string(),
                details: vec![
                    "app_index".to_string(),
                    "metrics profile=hub tiers=core,placement,runtime,security (inferred)"
                        .to_string(),
                    "sharding user_shards->user_shard cap=100 initial=1 max=4".to_string(),
                ],
            },
        ];
        let output = render_config_output(&title, &rows, true);

        assert_eq!(
            output,
            [
                "Fleet: test_me (network local)",
                "",
                "ROLE   KIND        AUTO   FEATURES         METRICS   TOPUP",
                "----   ---------   ----   --------------   -------   ------------------",
                "root   root        -      -                root      -",
                "app    singleton   yes    auth, sharding   hub       4.00 TC @ 10.00 TC",
                "  - app_index",
                "  - metrics profile=hub tiers=core,placement,runtime,security (inferred)",
                "  - sharding user_shards->user_shard cap=100 initial=1 max=4",
            ]
            .join("\n")
        );
    }

    // Ensure cycle balances parse from canic_cycle_balance command output.
    #[test]
    fn parses_cycle_balance_from_endpoint_output() {
        assert_eq!(
            parse_cycle_balance_response("(variant { 17_724 = 4_487_280_757_485 : nat })"),
            Some(4_487_280_757_485)
        );
        assert_eq!(
            parse_cycle_balance_response("(variant { 17_725 = record { code = 1 : nat } })"),
            None
        );
        assert_eq!(cycles_tc(12_345_678_900_000), "12.35 TC");
    }

    // Ensure metadata responses provide the Canic framework version for list output.
    #[test]
    fn parses_canic_version_from_metadata_output() {
        assert_eq!(
            parse_canic_metadata_version_response(
                r#"{"package_name":"app","canic_version":"0.33.6"}"#
            ),
            Some("0.33.6".to_string())
        );
        assert_eq!(
            parse_canic_metadata_version_response(
                r#"[{"package_name":"app","canic_version":"0.33.7"}]"#
            ),
            Some("0.33.7".to_string())
        );
        assert_eq!(
            parse_canic_metadata_version_response(
                r#"(record { package_name = "app"; canic_version = "0.33.8" })"#
            ),
            Some("0.33.8".to_string())
        );
        assert_eq!(parse_canic_metadata_version_response("{}"), None);
    }

    // Ensure readiness parsing accepts common command-line JSON shapes.
    #[test]
    fn parses_ready_json_shapes() {
        assert!(replica_query::parse_ready_json_value(&json!(true)));
        assert!(replica_query::parse_ready_json_value(
            &json!({ "Ok": true })
        ));
        assert!(!replica_query::parse_ready_json_value(&json!(false)));
        assert!(!replica_query::parse_ready_json_value(
            &json!({ "Ok": false })
        ));
    }

    // Build representative subnet registry JSON.
    fn registry_json() -> String {
        json!({
            "Ok": [
                {
                    "pid": ROOT,
                    "role": "root",
                    "record": {
                        "pid": ROOT,
                        "role": "root",
                        "kind": "root",
                        "parent_pid": null
                    }
                },
                {
                    "pid": APP,
                    "role": "app",
                    "record": {
                        "pid": APP,
                        "role": "app",
                        "kind": "singleton",
                        "parent_pid": ROOT
                    }
                },
                {
                    "pid": MINIMAL,
                    "role": "minimal",
                    "record": {
                        "pid": MINIMAL,
                        "role": "minimal",
                        "kind": "singleton",
                        "parent_pid": ROOT
                    }
                },
                {
                    "pid": WORKER,
                    "role": "worker",
                    "record": {
                        "pid": WORKER,
                        "role": "worker",
                        "kind": "replica",
                        "parent_pid": [APP]
                    }
                }
            ]
        })
        .to_string()
    }

    fn readiness_map() -> BTreeMap<String, ReadyStatus> {
        BTreeMap::from([
            (ROOT.to_string(), ReadyStatus::Ready),
            (APP.to_string(), ReadyStatus::NotReady),
            (MINIMAL.to_string(), ReadyStatus::Ready),
            (WORKER.to_string(), ReadyStatus::Error),
        ])
    }
}

use crate::{
    args::{local_network, print_help_or_version},
    version_text,
};
mod config;
mod options;
mod render;

#[cfg(test)]
use canic::ids::CanisterRole;
use canic_backup::discovery::{DiscoveryError, RegistryEntry, parse_registry_entries};
use canic_host::{
    format::byte_size,
    icp::{IcpCli, IcpCommandError},
    install_root::{InstallState, read_named_fleet_install_state},
    release_set::icp_root,
    replica_query,
};
use config::{load_config_role_rows, missing_config_roles, resolve_role_kinds};
use options::{ListOptions, ListSource, config_usage, usage};
#[cfg(not(test))]
use render::render_config_output;
#[cfg(test)]
use render::{
    CANISTER_HEADER, ConfigRoleRow, KIND_HEADER, READY_HEADER, ROLE_HEADER, RegistryRow,
    WASM_HEADER, kind_label, render_config_output, render_registry_separator,
    render_registry_table_row, render_registry_tree,
};
use render::{ListTitle, ReadyStatus, render_list_output, visible_entries};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs,
    path::PathBuf,
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
    let role_kinds = resolve_role_kinds(&options);
    let readiness = list_ready_statuses(&options, &registry, anchor.as_deref())?;
    let wasm_sizes = resolve_wasm_sizes(&options, &registry);
    let missing_roles = missing_config_roles(&options, &registry);
    let title = list_title(&options);
    println!(
        "{}",
        render_list_output(
            &title,
            &registry,
            anchor.as_deref(),
            &role_kinds,
            &readiness,
            &wasm_sizes,
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
    if replica_query::should_use_local_replica_query(options.network.as_deref()) {
        return Ok(
            match replica_query::query_ready(options.network.as_deref(), canister) {
                Ok(true) => ReadyStatus::Ready,
                Ok(false) => ReadyStatus::NotReady,
                Err(_) => ReadyStatus::Error,
            },
        );
    }

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
    if let Some(root) = &options.root {
        return Ok(root.clone());
    }

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
    options.anchor.clone()
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
    const WASM_STORE: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";

    // Ensure list options parse live registry queries.
    #[test]
    fn parses_live_list_options() {
        let options = ListOptions::parse_list([
            OsString::from("demo"),
            OsString::from("--root"),
            OsString::from(ROOT),
            OsString::from("--from"),
            OsString::from(APP),
            OsString::from("--network"),
            OsString::from("local"),
            OsString::from("--icp"),
            OsString::from("/bin/icp"),
        ])
        .expect("parse list options");

        assert_eq!(options.source, ListSource::RootRegistry);
        assert_eq!(options.fleet, "demo");
        assert_eq!(options.root, Some(ROOT.to_string()));
        assert_eq!(options.anchor, Some(APP.to_string()));
        assert_eq!(options.network, Some("local".to_string()));
        assert_eq!(options.icp, "/bin/icp");
        assert!(!options.verbose);
    }

    // Ensure config options parse declared fleet inspection.
    #[test]
    fn parses_config_options() {
        let options = ListOptions::parse_config([
            OsString::from("demo"),
            OsString::from("--network"),
            OsString::from("local"),
            OsString::from("--verbose"),
        ])
        .expect("parse config options");

        assert_eq!(options.source, ListSource::Config);
        assert_eq!(options.fleet, "demo");
        assert_eq!(options.root, None);
        assert_eq!(options.anchor, None);
        assert_eq!(options.network, Some("local".to_string()));
        assert_eq!(options.icp, "icp");
        assert!(options.verbose);
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
        assert!(list.contains("--from <name-or-principal>"));
        assert!(list.contains("--root <name-or-principal>"));
        assert!(config.contains("Usage: canic config [OPTIONS] <fleet>"));
        assert!(config.contains("<fleet>"));
        assert!(!config.contains("--fleet <name>"));
        assert!(config.contains("--from <role>"));
        assert!(config.contains("--verbose"));
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
        let role_kinds = BTreeMap::new();
        let readiness = readiness_map();
        let tree = render_registry_tree(&registry, None, &role_kinds, &readiness, &BTreeMap::new())
            .expect("render tree");
        let widths = [33, 7, 9, 5, 7];

        assert_eq!(
            tree,
            [
                render_registry_table_row(
                    &[
                        CANISTER_HEADER,
                        ROLE_HEADER,
                        KIND_HEADER,
                        READY_HEADER,
                        WASM_HEADER,
                    ],
                    &widths
                ),
                render_registry_separator(&widths),
                render_registry_table_row(&[ROOT, "root", "root", "yes", "-"], &widths),
                render_registry_table_row(
                    &[&format!("├─ {APP}"), "app", "singleton", "no", "-"],
                    &widths
                ),
                render_registry_table_row(
                    &[
                        &format!("│  └─ {WORKER}"),
                        "worker",
                        "replica",
                        "error",
                        "-"
                    ],
                    &widths
                ),
                render_registry_table_row(
                    &[&format!("└─ {MINIMAL}"), "minimal", "singleton", "yes", "-"],
                    &widths
                )
            ]
            .join("\n")
        );
    }

    // Ensure one selected subtree can be rendered without siblings.
    #[test]
    fn renders_selected_subtree() {
        let registry = parse_registry_entries(&registry_json()).expect("parse registry");
        let role_kinds = BTreeMap::new();
        let readiness = readiness_map();
        let tree = render_registry_tree(
            &registry,
            Some(APP),
            &role_kinds,
            &readiness,
            &BTreeMap::new(),
        )
        .expect("render subtree");
        let widths = [30, 6, 9, 5, 7];

        assert_eq!(
            tree,
            [
                render_registry_table_row(
                    &[
                        CANISTER_HEADER,
                        ROLE_HEADER,
                        KIND_HEADER,
                        READY_HEADER,
                        WASM_HEADER,
                    ],
                    &widths
                ),
                render_registry_separator(&widths),
                render_registry_table_row(&[APP, "app", "singleton", "no", "-"], &widths),
                render_registry_table_row(
                    &[&format!("└─ {WORKER}"), "worker", "replica", "error", "-"],
                    &widths
                )
            ]
            .join("\n")
        );
    }

    // Ensure config role kinds fill entries that do not carry registry kind data.
    #[test]
    fn renders_registry_table_with_config_kinds() {
        let mut registry = parse_registry_entries(&registry_json()).expect("parse registry");
        for entry in &mut registry {
            entry.kind = None;
        }
        let role_kinds = BTreeMap::from([
            ("root".to_string(), "root".to_string()),
            ("app".to_string(), "singleton".to_string()),
            ("minimal".to_string(), "singleton".to_string()),
            ("worker".to_string(), "replica".to_string()),
        ]);
        let readiness = readiness_map();
        let tree = render_registry_tree(&registry, None, &role_kinds, &readiness, &BTreeMap::new())
            .expect("render tree");
        let widths = [33, 7, 9, 5, 7];

        assert_eq!(
            tree,
            [
                render_registry_table_row(
                    &[
                        CANISTER_HEADER,
                        ROLE_HEADER,
                        KIND_HEADER,
                        READY_HEADER,
                        WASM_HEADER,
                    ],
                    &widths
                ),
                render_registry_separator(&widths),
                render_registry_table_row(&[ROOT, "root", "root", "yes", "-"], &widths),
                render_registry_table_row(
                    &[&format!("├─ {APP}"), "app", "singleton", "no", "-"],
                    &widths
                ),
                render_registry_table_row(
                    &[
                        &format!("│  └─ {WORKER}"),
                        "worker",
                        "replica",
                        "error",
                        "-"
                    ],
                    &widths
                ),
                render_registry_table_row(
                    &[&format!("└─ {MINIMAL}"), "minimal", "singleton", "yes", "-"],
                    &widths
                )
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
        let output = render_list_output(
            &title,
            &registry,
            Some(APP),
            &BTreeMap::new(),
            &readiness_map(),
            &BTreeMap::new(),
            &[],
        )
        .expect("render list output");

        assert!(output.starts_with("Fleet: demo (network local)\n\nCANISTER_ID"));
        assert!(output.contains("\n------------------------------"));
    }

    #[test]
    fn renders_list_output_with_wasm_size_and_missing_roles() {
        let registry = parse_registry_entries(&registry_json()).expect("parse registry");
        let title = ListTitle {
            fleet: "demo".to_string(),
            network: "local".to_string(),
        };
        let wasm_sizes = BTreeMap::from([("app".to_string(), "811.20 KiB".to_string())]);
        let output = render_list_output(
            &title,
            &registry,
            None,
            &BTreeMap::new(),
            &readiness_map(),
            &wasm_sizes,
            &["audit".to_string()],
        )
        .expect("render list output");

        assert!(output.contains("WASM_GZ"));
        assert!(output.contains("811.20 KiB"));
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
                details: Vec::new(),
            },
            ConfigRoleRow {
                role: "app".to_string(),
                kind: "singleton".to_string(),
                capabilities: "auth, sharding".to_string(),
                auto_create: "yes".to_string(),
                topup: "4.0TC @ 10.0TC".to_string(),
                details: vec![
                    "app_index".to_string(),
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
                "ROLE   KIND        FEATURES         AUTO   TOPUP",
                "----   ---------   --------------   ----   --------------",
                "root   root        -                -      -",
                "app    singleton   auth, sharding   yes    4.0TC @ 10.0TC",
                "  - app_index",
                "  - sharding user_shards->user_shard cap=100 initial=1 max=4",
            ]
            .join("\n")
        );
    }

    // Ensure the implicit wasm store role has a concrete kind even though config omits it.
    #[test]
    fn implicit_wasm_store_kind_is_not_unknown() {
        let entry = RegistryEntry {
            pid: WASM_STORE.to_string(),
            role: Some(CanisterRole::WASM_STORE.as_str().to_string()),
            kind: None,
            parent_pid: Some(ROOT.to_string()),
        };
        let row = RegistryRow {
            entry: &entry,
            tree_prefix: String::new(),
        };

        assert_eq!(
            kind_label(&row, &BTreeMap::new()),
            CanisterRole::WASM_STORE.as_str()
        );
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

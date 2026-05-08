use crate::{
    args::{default_network, print_help_or_version},
    version_text,
};
mod options;
mod render;

use candid::Principal;
#[cfg(test)]
use canic::ids::CanisterRole;
use canic_backup::discovery::{DiscoveryError, RegistryEntry, parse_registry_entries};
use canic_host::{
    dfx::{Dfx, DfxCommandError},
    install_root::{InstallState, read_current_or_fleet_install_state},
    release_set::{config_path as default_config_path, configured_role_kinds},
    replica_query,
};
use options::{ListOptions, ListSource, usage};
#[cfg(test)]
use render::{
    CANISTER_HEADER, KIND_HEADER, READY_HEADER, ROLE_HEADER, RegistryRow, kind_label,
    render_registry_separator, render_registry_table_row, render_registry_tree,
};
use render::{ListTitle, ReadyStatus, render_list_output, visible_entries};
use std::{collections::BTreeMap, ffi::OsString};
use thiserror::Error as ThisError;

const DEMO_CANISTER_NAMES: &[&str] = &[
    "app",
    "minimal",
    "user_hub",
    "user_shard",
    "scale_hub",
    "scale",
    "root",
];

///
/// ListCommandError
///

#[derive(Debug, ThisError)]
pub enum ListCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("unknown option {0}")]
    UnknownOption(String),

    #[error("cannot combine --standalone with --root")]
    ConflictingListSources,

    #[error(
        "no local canister ids are available yet; run dfx canister create <name>, or use make demo-install for the full reference topology"
    )]
    NoStandaloneCanisters,

    #[error("registry JSON did not contain the requested canister {0}")]
    CanisterNotInRegistry(String),

    #[error("dfx command failed: {command}\n{stderr}")]
    DfxFailed { command: String, stderr: String },

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error(
        "saved fleet {fleet} points to root {root}, but that canister is not present on network {network}. Local dfx state was probably restarted or reset. Run `canic install` to recreate the fleet, `canic list --standalone` to see local dfx canister ids, or `canic fleet use <fleet>` after reinstalling."
    )]
    StaleLocalFleet {
        fleet: String,
        network: String,
        root: String,
    },

    #[error("failed to read canic fleet state: {0}")]
    InstallState(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
}

/// Run a list subcommand or the default tree listing.
pub fn run<I>(args: I) -> Result<(), ListCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let mut options = ListOptions::parse(args)?;
    options.source = resolve_effective_source(&options)?;
    let registry = load_registry_entries(&options)?;
    let anchor = resolve_tree_anchor(&options)?;
    let role_kinds = resolve_role_kinds(&options);
    let readiness = list_ready_statuses(&options, &registry, anchor.as_deref())?;
    let title = list_title(&options);
    println!(
        "{}",
        render_list_output(
            &title,
            &registry,
            anchor.as_deref(),
            &role_kinds,
            &readiness
        )?
    );
    if let Some(hint) = standalone_next_step_hint(&options, &registry) {
        eprintln!("Hint: {hint}");
    }
    Ok(())
}

// Pick the current installed fleet when the project has Canic fleet state.
fn resolve_effective_source(options: &ListOptions) -> Result<ListSource, ListCommandError> {
    if !matches!(options.source, ListSource::Auto) {
        return Ok(options.source);
    }

    if read_selected_install_state(options)
        .map_err(|err| ListCommandError::InstallState(err.to_string()))?
        .is_some()
    {
        Ok(ListSource::RootRegistry)
    } else {
        Ok(ListSource::Standalone)
    }
}

// Return the operator-facing title for the selected list source.
fn list_title(options: &ListOptions) -> ListTitle {
    let fleet = match options.source {
        ListSource::Standalone => "standalone".to_string(),
        ListSource::Auto | ListSource::RootRegistry => read_selected_install_state(options)
            .ok()
            .flatten()
            .map(|state| state.fleet)
            .or_else(|| options.fleet.clone())
            .unwrap_or_else(|| "root-registry".to_string()),
    };

    ListTitle {
        fleet,
        network: state_network(options),
    }
}

// Resolve role kind labels from the selected project config when it is available.
fn resolve_role_kinds(options: &ListOptions) -> BTreeMap<String, String> {
    role_kind_config_candidates(options)
        .into_iter()
        .find_map(|path| configured_role_kinds(&path).ok())
        .unwrap_or_default()
}

// Return likely config paths in preference order without making list depend on them.
fn role_kind_config_candidates(options: &ListOptions) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    if let Ok(Some(state)) = read_selected_install_state(options) {
        paths.push(std::path::PathBuf::from(state.config_path));
    }

    if let Ok(workspace_root) = std::env::current_dir() {
        paths.push(default_config_path(&workspace_root));
    }

    paths
}

// Return ready statuses for the visible live list.
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

// Query one canister's generated Canic readiness endpoint.
fn check_ready_status(
    options: &ListOptions,
    canister: &str,
) -> Result<ReadyStatus, ListCommandError> {
    if replica_query::should_use_local_replica_query(options.network.as_deref()) {
        return Ok(
            match replica_query::query_ready(&options.dfx, options.network.as_deref(), canister) {
                Ok(true) => ReadyStatus::Ready,
                Ok(false) => ReadyStatus::NotReady,
                Err(_) => ReadyStatus::Error,
            },
        );
    }

    let Ok(output) = Dfx::new(&options.dfx, options.network.clone()).canister_call_output(
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

// Load registry entries from standalone dfx ids or a live root canister query.
fn load_registry_entries(options: &ListOptions) -> Result<Vec<RegistryEntry>, ListCommandError> {
    if matches!(options.source, ListSource::Standalone | ListSource::Auto) {
        return load_standalone_entries(options);
    }

    let registry_json = match options.source {
        ListSource::RootRegistry => {
            let root = resolve_root_canister(options)?;
            call_subnet_registry(options, &root)?
        }
        ListSource::Standalone | ListSource::Auto => {
            unreachable!("standalone source returned above")
        }
    };

    parse_registry_entries(&registry_json).map_err(ListCommandError::from)
}

// Load created canisters from the current dfx project without requiring a Canic root.
fn load_standalone_entries(options: &ListOptions) -> Result<Vec<RegistryEntry>, ListCommandError> {
    let mut entries = Vec::new();

    for name in DEMO_CANISTER_NAMES {
        let Some(pid) = resolve_project_canister_id(options, name)? else {
            continue;
        };
        entries.push(RegistryEntry {
            pid,
            role: Some((*name).to_string()),
            kind: None,
            parent_pid: None,
        });
    }

    if entries.is_empty() {
        return Err(ListCommandError::NoStandaloneCanisters);
    }

    Ok(entries)
}

// Resolve one local project canister id, returning None when it has not been created yet.
fn resolve_project_canister_id(
    options: &ListOptions,
    name: &str,
) -> Result<Option<String>, ListCommandError> {
    Dfx::new(&options.dfx, options.network.clone())
        .canister_id_optional(name)
        .map_err(list_dfx_error)
}

// Resolve the explicit root id or the current dfx project's `root` canister id.
fn resolve_root_canister(options: &ListOptions) -> Result<String, ListCommandError> {
    if let Some(root) = &options.root {
        return resolve_canister_identifier(options, root);
    }

    if let Some(state) = read_selected_install_state(options)
        .map_err(|err| ListCommandError::InstallState(err.to_string()))?
    {
        return Ok(state.root_canister_id);
    }

    Dfx::new(&options.dfx, options.network.clone())
        .canister_id("root")
        .map_err(list_dfx_error)
}

// Read the current or explicitly selected fleet install state.
fn read_selected_install_state(
    options: &ListOptions,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    read_current_or_fleet_install_state(&state_network(options), options.fleet.as_deref())
}

// Resolve the selected tree anchor as a principal when a local dfx name is supplied.
fn resolve_tree_anchor(options: &ListOptions) -> Result<Option<String>, ListCommandError> {
    options
        .anchor
        .as_deref()
        .map(|anchor| resolve_canister_identifier(options, anchor))
        .transpose()
}

// Accept either an IC principal or a local dfx canister name for list inputs.
fn resolve_canister_identifier(
    options: &ListOptions,
    identifier: &str,
) -> Result<String, ListCommandError> {
    if Principal::from_text(identifier).is_ok() {
        return Ok(identifier.to_string());
    }

    resolve_project_canister_id(options, identifier)
        .map(|id| id.unwrap_or_else(|| identifier.to_string()))
}

// Resolve the state network using the same local default as host install commands.
fn state_network(options: &ListOptions) -> String {
    options.network.clone().unwrap_or_else(default_network)
}

// Run `dfx canister call <root> canic_subnet_registry --output json`.
fn call_subnet_registry(options: &ListOptions, root: &str) -> Result<String, ListCommandError> {
    if replica_query::should_use_local_replica_query(options.network.as_deref()) {
        return replica_query::query_subnet_registry_json(
            &options.dfx,
            options.network.as_deref(),
            root,
        )
        .map_err(|err| list_replica_query_error(options, root, err.to_string()));
    }

    Dfx::new(&options.dfx, options.network.clone())
        .canister_call_output(root, "canic_subnet_registry", Some("json"))
        .map_err(list_dfx_error)
        .map_err(add_root_registry_hint)
}

// Convert local replica query failures into operator-facing setup guidance.
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

// Detect the local replica's missing-canister query diagnostic.
fn is_canister_not_found_error(error: &str) -> bool {
    error.contains("Canister ") && error.contains(" not found")
}

// Add a next-step hint for common root registry setup mistakes.
fn add_root_registry_hint(error: ListCommandError) -> ListCommandError {
    let ListCommandError::DfxFailed { command, stderr } = error else {
        return error;
    };

    let Some(hint) = root_registry_hint(&stderr) else {
        return ListCommandError::DfxFailed { command, stderr };
    };

    ListCommandError::DfxFailed {
        command,
        stderr: format!("{stderr}\nHint: {hint}\n"),
    }
}

// Convert host dfx failures into the list command's public error surface.
fn list_dfx_error(error: DfxCommandError) -> ListCommandError {
    match error {
        DfxCommandError::Io(err) => ListCommandError::Io(err),
        DfxCommandError::Failed { command, stderr } => {
            ListCommandError::DfxFailed { command, stderr }
        }
        DfxCommandError::SnapshotIdUnavailable { output } => ListCommandError::DfxFailed {
            command: "dfx canister snapshot create".to_string(),
            stderr: output,
        },
    }
}

// Return guidance for root registry calls that cannot reach an installed Canic root.
fn root_registry_hint(stderr: &str) -> Option<&'static str> {
    if stderr.contains("Cannot find canister id") {
        return Some(
            "no root canister id exists in this dfx project. Use plain `canic list` for local standalone inventory, or run `canic install` before querying the root registry.",
        );
    }

    if stderr.contains("contains no Wasm module") || stderr.contains("wasm-module-not-found") {
        return Some(
            "`dfx canister create root` only reserves an id; it does not install Canic root code. Run `canic install`, then use `canic list`.",
        );
    }

    None
}

// Explain the next setup step when standalone inventory only finds a reserved root id.
fn standalone_next_step_hint(
    options: &ListOptions,
    registry: &[RegistryEntry],
) -> Option<&'static str> {
    if !matches!(options.source, ListSource::Standalone) {
        return None;
    }

    let [entry] = registry else {
        return None;
    };

    if entry.role.as_deref() != Some("root") {
        return None;
    }

    Some(
        "only the local root id exists. Run `canic install` to build, install, stage, and bootstrap the tree; then run `canic list`.",
    )
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
        let options = ListOptions::parse([
            OsString::from("--root"),
            OsString::from(ROOT),
            OsString::from("--fleet"),
            OsString::from("demo"),
            OsString::from("--from"),
            OsString::from(APP),
            OsString::from("--network"),
            OsString::from("local"),
            OsString::from("--dfx"),
            OsString::from("/bin/dfx"),
        ])
        .expect("parse list options");

        assert_eq!(options.source, ListSource::RootRegistry);
        assert_eq!(options.fleet, Some("demo".to_string()));
        assert_eq!(options.root, Some(ROOT.to_string()));
        assert_eq!(options.anchor, Some(APP.to_string()));
        assert_eq!(options.network, Some("local".to_string()));
        assert_eq!(options.dfx, "/bin/dfx");
    }

    // Ensure list defaults to automatic source selection.
    #[test]
    fn parses_default_auto_list_options() {
        let options = ListOptions::parse([OsString::from("--network"), OsString::from("local")])
            .expect("parse default standalone options");

        assert_eq!(options.source, ListSource::Auto);
        assert_eq!(options.fleet, None);
        assert_eq!(options.root, None);
        assert_eq!(options.anchor, None);
        assert_eq!(options.network, Some("local".to_string()));
        assert_eq!(options.dfx, "dfx");
    }

    // Ensure list help explains fleet selection and subtree rendering.
    #[test]
    fn list_usage_explains_fleet_and_subtree_options() {
        let text = usage();

        assert!(text.contains("Show registry canisters as a tree table"));
        assert!(text.contains("Usage: canic list"));
        assert!(text.contains("--fleet <name>"));
        assert!(text.contains("--from <name-or-principal>"));
        assert!(text.contains("Examples:"));
    }

    // Ensure conflicting registry sources are still rejected.
    #[test]
    fn rejects_conflicting_registry_sources() {
        let err = ListOptions::parse([
            OsString::from("--standalone"),
            OsString::from("--root"),
            OsString::from(ROOT),
        ])
        .expect_err("conflicting sources should fail");

        assert!(matches!(err, ListCommandError::ConflictingListSources));
    }

    // Ensure standalone inventory uses the hardcoded demo canister roster.
    #[test]
    fn standalone_inventory_uses_static_demo_canister_names() {
        assert_eq!(
            DEMO_CANISTER_NAMES,
            &[
                "app",
                "minimal",
                "user_hub",
                "user_shard",
                "scale_hub",
                "scale",
                "root",
            ]
        );
    }

    // Ensure empty-root dfx errors explain the standalone/root split.
    #[test]
    fn root_registry_hint_explains_empty_root_canister() {
        let hint = root_registry_hint("the canister contains no Wasm module")
            .expect("empty wasm hint should be available");

        assert!(hint.contains("canic install"));
        assert!(hint.contains("`dfx canister create root` only reserves an id"));
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

    // Ensure root-only standalone inventory explains the install/bootstrap command.
    #[test]
    fn standalone_next_step_hint_explains_root_only_inventory() {
        let options = ListOptions {
            source: ListSource::Standalone,
            fleet: None,
            root: None,
            anchor: None,
            network: Some("local".to_string()),
            dfx: "dfx".to_string(),
        };
        let registry = vec![RegistryEntry {
            pid: ROOT.to_string(),
            role: Some("root".to_string()),
            kind: None,
            parent_pid: None,
        }];

        let hint = standalone_next_step_hint(&options, &registry)
            .expect("root-only standalone hint should be available");

        assert!(hint.contains("canic install"));
        assert!(hint.contains("canic list"));
    }

    // Ensure non-standalone sources do not get local setup hints.
    #[test]
    fn standalone_next_step_hint_skips_root_registry_source() {
        let options = ListOptions::parse([OsString::from("--root"), OsString::from(ROOT)])
            .expect("parse root options");
        let registry = vec![RegistryEntry {
            pid: ROOT.to_string(),
            role: Some("root".to_string()),
            kind: None,
            parent_pid: None,
        }];

        assert!(standalone_next_step_hint(&options, &registry).is_none());
    }

    // Ensure registry entries render as a stable whitespace table.
    #[test]
    fn renders_registry_table() {
        let registry = parse_registry_entries(&registry_json()).expect("parse registry");
        let role_kinds = BTreeMap::new();
        let readiness = readiness_map();
        let tree =
            render_registry_tree(&registry, None, &role_kinds, &readiness).expect("render tree");
        let widths = [33, 7, 9, 5];

        assert_eq!(
            tree,
            [
                render_registry_table_row(
                    &[CANISTER_HEADER, ROLE_HEADER, KIND_HEADER, READY_HEADER],
                    &widths
                ),
                render_registry_separator(&widths),
                render_registry_table_row(&[ROOT, "root", "root", "yes"], &widths),
                render_registry_table_row(
                    &[&format!("├─ {APP}"), "app", "singleton", "no"],
                    &widths
                ),
                render_registry_table_row(
                    &[&format!("│  └─ {WORKER}"), "worker", "replica", "error"],
                    &widths
                ),
                render_registry_table_row(
                    &[&format!("└─ {MINIMAL}"), "minimal", "singleton", "yes"],
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
        let tree = render_registry_tree(&registry, Some(APP), &role_kinds, &readiness)
            .expect("render subtree");
        let widths = [30, 6, 9, 5];

        assert_eq!(
            tree,
            [
                render_registry_table_row(
                    &[CANISTER_HEADER, ROLE_HEADER, KIND_HEADER, READY_HEADER],
                    &widths
                ),
                render_registry_separator(&widths),
                render_registry_table_row(&[APP, "app", "singleton", "no"], &widths),
                render_registry_table_row(
                    &[&format!("└─ {WORKER}"), "worker", "replica", "error"],
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
        let tree =
            render_registry_tree(&registry, None, &role_kinds, &readiness).expect("render tree");
        let widths = [33, 7, 9, 5];

        assert_eq!(
            tree,
            [
                render_registry_table_row(
                    &[CANISTER_HEADER, ROLE_HEADER, KIND_HEADER, READY_HEADER],
                    &widths
                ),
                render_registry_separator(&widths),
                render_registry_table_row(&[ROOT, "root", "root", "yes"], &widths),
                render_registry_table_row(
                    &[&format!("├─ {APP}"), "app", "singleton", "no"],
                    &widths
                ),
                render_registry_table_row(
                    &[&format!("│  └─ {WORKER}"), "worker", "replica", "error"],
                    &widths
                ),
                render_registry_table_row(
                    &[&format!("└─ {MINIMAL}"), "minimal", "singleton", "yes"],
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
        )
        .expect("render list output");

        assert!(output.starts_with("Fleet: demo\nNetwork: local\n\nCANISTER_ID"));
        assert!(output.contains("\n------------------------------"));
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

    // Ensure readiness parsing accepts the JSON shapes emitted by dfx.
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

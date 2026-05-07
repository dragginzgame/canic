use crate::{
    args::{
        first_arg_is_help, first_arg_is_version, flag_arg, parse_matches, string_option, value_arg,
    },
    version_text,
};
use candid::Principal;
use canic::ids::CanisterRole;
use canic_backup::discovery::{DiscoveryError, RegistryEntry, parse_registry_entries};
use canic_host::{
    dfx::{Dfx, DfxCommandError},
    install_root::{InstallState, read_current_or_fleet_install_state},
    release_set::{config_path as default_config_path, configured_role_kinds},
    replica_query,
    table::WhitespaceTable,
};
use clap::Command as ClapCommand;
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::OsString,
};
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
const ROLE_HEADER: &str = "ROLE";
const KIND_HEADER: &str = "KIND";
const CANISTER_HEADER: &str = "CANISTER_ID";
const READY_HEADER: &str = "READY";
const TREE_BRANCH: &str = "├─ ";
const TREE_LAST: &str = "└─ ";
const TREE_PIPE: &str = "│  ";
const TREE_SPACE: &str = "   ";

///
/// ListCommandError
///

#[derive(Debug, ThisError)]
pub enum ListCommandError {
    #[error("{0}")]
    Usage(&'static str),

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

    #[error("failed to read canic fleet state: {0}")]
    InstallState(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
}

///
/// ListOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListOptions {
    pub source: ListSource,
    pub fleet: Option<String>,
    pub root: Option<String>,
    pub anchor: Option<String>,
    pub network: Option<String>,
    pub dfx: String,
}

///
/// ListSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListSource {
    Auto,
    Standalone,
    RootRegistry,
}

impl ListOptions {
    /// Parse canister listing options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, ListCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let args = args.into_iter().collect::<Vec<_>>();
        let matches =
            parse_matches(list_command(), args).map_err(|_| ListCommandError::Usage(usage()))?;
        let standalone = matches.get_flag("standalone");
        let root = string_option(&matches, "root");

        if standalone && root.is_some() {
            return Err(ListCommandError::ConflictingListSources);
        }

        let source = if root.is_some() {
            ListSource::RootRegistry
        } else if standalone {
            ListSource::Standalone
        } else {
            ListSource::Auto
        };

        Ok(Self {
            source,
            fleet: string_option(&matches, "fleet"),
            root,
            anchor: string_option(&matches, "from"),
            network: string_option(&matches, "network"),
            dfx: string_option(&matches, "dfx").unwrap_or_else(|| "dfx".to_string()),
        })
    }
}

// Build the list parser.
fn list_command() -> ClapCommand {
    ClapCommand::new("list")
        .disable_help_flag(true)
        .arg(flag_arg("standalone").long("standalone"))
        .arg(value_arg("fleet").long("fleet"))
        .arg(value_arg("root").long("root"))
        .arg(value_arg("from").long("from"))
        .arg(value_arg("network").long("network"))
        .arg(value_arg("dfx").long("dfx"))
}

/// Run a list subcommand or the default tree listing.
pub fn run<I>(args: I) -> Result<(), ListCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if first_arg_is_help(&args) {
        println!("{}", usage());
        return Ok(());
    }
    if first_arg_is_version(&args) {
        println!("{}", version_text());
        return Ok(());
    }

    let mut options = ListOptions::parse(args)?;
    options.source = resolve_effective_source(&options)?;
    let registry = load_registry_entries(&options)?;
    let anchor = resolve_tree_anchor(&options)?;
    let role_kinds = resolve_role_kinds(&options);
    let readiness = list_ready_statuses(&options, &registry, anchor.as_deref())?;
    println!(
        "{}",
        render_registry_tree(&registry, anchor.as_deref(), &role_kinds, &readiness)?
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

/// Render all registry entries, or one selected subtree, as a whitespace table.
pub fn render_registry_tree(
    registry: &[RegistryEntry],
    canister: Option<&str>,
    role_kinds: &BTreeMap<String, String>,
    readiness: &BTreeMap<String, ReadyStatus>,
) -> Result<String, ListCommandError> {
    let rows = visible_rows(registry, canister)?;
    Ok(render_registry_table(&rows, role_kinds, readiness))
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

    if let Ok(workspace_root) = env::current_dir() {
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
    Ok(if parse_ready_value(&data) {
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
    options
        .network
        .clone()
        .or_else(|| env::var("DFX_NETWORK").ok())
        .unwrap_or_else(|| "local".to_string())
}

// Run `dfx canister call <root> canic_subnet_registry --output json`.
fn call_subnet_registry(options: &ListOptions, root: &str) -> Result<String, ListCommandError> {
    if replica_query::should_use_local_replica_query(options.network.as_deref()) {
        return replica_query::query_subnet_registry_json(
            &options.dfx,
            options.network.as_deref(),
            root,
        )
        .map_err(|err| ListCommandError::ReplicaQuery(err.to_string()));
    }

    Dfx::new(&options.dfx, options.network.clone())
        .canister_call_output(root, "canic_subnet_registry", Some("json"))
        .map_err(list_dfx_error)
        .map_err(add_root_registry_hint)
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

// Select forest roots or validate the requested subtree root.
fn root_entries<'a>(
    registry: &'a [RegistryEntry],
    by_pid: &BTreeMap<&str, &'a RegistryEntry>,
    canister: Option<&str>,
) -> Result<Vec<&'a RegistryEntry>, ListCommandError> {
    if let Some(canister) = canister {
        return by_pid
            .get(canister)
            .copied()
            .map(|entry| vec![entry])
            .ok_or_else(|| ListCommandError::CanisterNotInRegistry(canister.to_string()));
    }

    let ids = registry
        .iter()
        .map(|entry| entry.pid.as_str())
        .collect::<BTreeSet<_>>();
    Ok(registry
        .iter()
        .filter(|entry| {
            entry
                .parent_pid
                .as_deref()
                .is_none_or(|parent| !ids.contains(parent))
        })
        .collect())
}

// Group children by parent and keep each group sorted for stable output.
fn child_entries(registry: &[RegistryEntry]) -> BTreeMap<&str, Vec<&RegistryEntry>> {
    let mut children = BTreeMap::<&str, Vec<&RegistryEntry>>::new();
    for entry in registry {
        if let Some(parent) = entry.parent_pid.as_deref() {
            children.entry(parent).or_default().push(entry);
        }
    }
    for entries in children.values_mut() {
        entries.sort_by_key(|entry| (entry.role.as_deref().unwrap_or(""), entry.pid.as_str()));
    }
    children
}

// Return the entries that would be rendered for the selected table.
fn visible_entries<'a>(
    registry: &'a [RegistryEntry],
    canister: Option<&str>,
) -> Result<Vec<&'a RegistryEntry>, ListCommandError> {
    Ok(visible_rows(registry, canister)?
        .into_iter()
        .map(|row| row.entry)
        .collect())
}

// Return visible rows with tree prefixes so canister ids carry hierarchy.
fn visible_rows<'a>(
    registry: &'a [RegistryEntry],
    canister: Option<&str>,
) -> Result<Vec<RegistryRow<'a>>, ListCommandError> {
    let by_pid = registry
        .iter()
        .map(|entry| (entry.pid.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let roots = root_entries(registry, &by_pid, canister)?;
    let children = child_entries(registry);
    let mut entries = Vec::new();

    for root in roots {
        collect_visible_entry(root, &children, "", "", &mut entries);
    }

    Ok(entries)
}

// Traverse one rendered branch in display order.
fn collect_visible_entry<'a>(
    entry: &'a RegistryEntry,
    children: &BTreeMap<&str, Vec<&'a RegistryEntry>>,
    tree_prefix: &str,
    child_prefix: &str,
    entries: &mut Vec<RegistryRow<'a>>,
) {
    entries.push(RegistryRow {
        entry,
        tree_prefix: tree_prefix.to_string(),
    });
    if let Some(child_entries) = children.get(entry.pid.as_str()) {
        for (index, child) in child_entries.iter().enumerate() {
            let is_last = index + 1 == child_entries.len();
            let branch = if is_last { TREE_LAST } else { TREE_BRANCH };
            let carry = if is_last { TREE_SPACE } else { TREE_PIPE };
            let child_tree_prefix = format!("{child_prefix}{branch}");
            let descendant_prefix = format!("{child_prefix}{carry}");
            collect_visible_entry(
                child,
                children,
                &child_tree_prefix,
                &descendant_prefix,
                entries,
            );
        }
    }
}

///
/// RegistryRow
///

struct RegistryRow<'a> {
    entry: &'a RegistryEntry,
    tree_prefix: String,
}

// Render registry rows as stable whitespace-aligned columns.
fn render_registry_table(
    rows: &[RegistryRow<'_>],
    role_kinds: &BTreeMap<String, String>,
    readiness: &BTreeMap<String, ReadyStatus>,
) -> String {
    let mut table = WhitespaceTable::new([CANISTER_HEADER, ROLE_HEADER, KIND_HEADER, READY_HEADER]);
    for row in rows {
        let ready = readiness
            .get(&row.entry.pid)
            .map_or("unknown", |status| status.label());
        table.push_row([
            canister_label(row),
            role_label(row),
            kind_label(row, role_kinds),
            ready.to_string(),
        ]);
    }

    table.render()
}

// Format one canister principal label with its box-drawing tree branch.
fn canister_label(row: &RegistryRow<'_>) -> String {
    format!("{}{}", row.tree_prefix, row.entry.pid)
}

// Format one role label without adding hierarchy because role names are not unique.
fn role_label(row: &RegistryRow<'_>) -> String {
    let role = row.entry.role.as_deref().filter(|role| !role.is_empty());
    match role {
        Some(role) => role.to_string(),
        None => "unknown".to_string(),
    }
}

// Format one canister kind using registry data first, then config role metadata.
fn kind_label(row: &RegistryRow<'_>, role_kinds: &BTreeMap<String, String>) -> String {
    row.entry
        .kind
        .as_deref()
        .or_else(|| {
            row.entry
                .role
                .as_deref()
                .and_then(|role| role_kinds.get(role).map(String::as_str))
        })
        .or_else(|| {
            row.entry.role.as_deref().and_then(|role| {
                CanisterRole::owned(role.to_string())
                    .is_wasm_store()
                    .then(|| CanisterRole::WASM_STORE.as_str())
            })
        })
        .unwrap_or("unknown")
        .to_string()
}

// Accept both plain-bool and wrapped-result JSON shapes from `dfx --output json`.
fn parse_ready_value(data: &serde_json::Value) -> bool {
    matches!(data, serde_json::Value::Bool(true))
        || matches!(data.get("Ok"), Some(serde_json::Value::Bool(true)))
}

///
/// ReadyStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadyStatus {
    Ready,
    NotReady,
    Error,
}

impl ReadyStatus {
    // Return the compact label used in list output.
    const fn label(self) -> &'static str {
        match self {
            Self::Ready => "yes",
            Self::NotReady => "no",
            Self::Error => "error",
        }
    }
}

// Return list command usage text.
const fn usage() -> &'static str {
    "usage: canic list [--standalone] [--fleet <name>] [--root <root-canister>] [--from <canister>] [--network <name>] [--dfx <path>]"
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

        assert_eq!(
            tree,
            format!(
                "{:<33}  {:<7}  {:<9}  {}\n{:<33}  {:<7}  {:<9}  {}\n{:<33}  {:<7}  {:<9}  {}\n{:<33}  {:<7}  {:<9}  {}\n{:<33}  {:<7}  {:<9}  {}",
                "CANISTER_ID",
                "ROLE",
                "KIND",
                "READY",
                ROOT,
                "root",
                "root",
                "yes",
                format!("├─ {APP}"),
                "app",
                "singleton",
                "no",
                format!("│  └─ {WORKER}"),
                "worker",
                "replica",
                "error",
                format!("└─ {MINIMAL}"),
                "minimal",
                "singleton",
                "yes"
            )
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

        assert_eq!(
            tree,
            format!(
                "{:<30}  {:<6}  {:<9}  {}\n{:<30}  {:<6}  {:<9}  {}\n{:<30}  {:<6}  {:<9}  {}",
                "CANISTER_ID",
                "ROLE",
                "KIND",
                "READY",
                APP,
                "app",
                "singleton",
                "no",
                format!("└─ {WORKER}"),
                "worker",
                "replica",
                "error"
            )
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

        assert_eq!(
            tree,
            format!(
                "{:<33}  {:<7}  {:<9}  {}\n{:<33}  {:<7}  {:<9}  {}\n{:<33}  {:<7}  {:<9}  {}\n{:<33}  {:<7}  {:<9}  {}\n{:<33}  {:<7}  {:<9}  {}",
                "CANISTER_ID",
                "ROLE",
                "KIND",
                "READY",
                ROOT,
                "root",
                "root",
                "yes",
                format!("├─ {APP}"),
                "app",
                "singleton",
                "no",
                format!("│  └─ {WORKER}"),
                "worker",
                "replica",
                "error",
                format!("└─ {MINIMAL}"),
                "minimal",
                "singleton",
                "yes"
            )
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

    // Ensure readiness parsing accepts the JSON shapes emitted by dfx.
    #[test]
    fn parses_ready_json_shapes() {
        assert!(parse_ready_value(&json!(true)));
        assert!(parse_ready_value(&json!({ "Ok": true })));
        assert!(!parse_ready_value(&json!(false)));
        assert!(!parse_ready_value(&json!({ "Ok": false })));
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

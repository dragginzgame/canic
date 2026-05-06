use crate::version_text;
use candid::{CandidType, Decode, Encode, Principal};
use canic::ids::CanisterRole;
use canic_backup::discovery::{DiscoveryError, RegistryEntry, parse_registry_entries};
use canic_installer::{
    install_root::{InstallState, read_current_or_fleet_install_state},
    release_set::{config_path as default_config_path, configured_role_kinds},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::OsString,
    io::{Read, Write},
    net::TcpStream,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
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

    #[error("option {0} requires a value")]
    MissingValue(&'static str),

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

    #[error("local replica rejected query: code={code} message={message}")]
    ReplicaRejected { code: u64, message: String },

    #[error("failed to read canic fleet state: {0}")]
    InstallState(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Cbor(#[from] serde_cbor::Error),

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
        let mut standalone = false;
        let mut fleet = None;
        let mut root = None;
        let mut anchor = None;
        let mut network = None;
        let mut dfx = "dfx".to_string();

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| ListCommandError::Usage(usage()))?;
            if let Some(value) = arg.strip_prefix("--fleet=") {
                fleet = Some(value.to_string());
                continue;
            }
            if let Some(value) = arg.strip_prefix("--root=") {
                root = Some(value.to_string());
                continue;
            }
            if let Some(value) = arg.strip_prefix("--from=") {
                anchor = Some(value.to_string());
                continue;
            }
            if let Some(value) = arg.strip_prefix("--network=") {
                network = Some(value.to_string());
                continue;
            }
            match arg.as_str() {
                "--standalone" => standalone = true,
                "--fleet" => fleet = Some(next_value(&mut args, "--fleet")?),
                "--root" => root = Some(next_value(&mut args, "--root")?),
                "--from" => anchor = Some(next_value(&mut args, "--from")?),
                "--network" => network = Some(next_value(&mut args, "--network")?),
                "--dfx" => dfx = next_value(&mut args, "--dfx")?,
                "--help" | "-h" => return Err(ListCommandError::Usage(usage())),
                _ => return Err(ListCommandError::UnknownOption(arg)),
            }
        }

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
            fleet,
            root,
            anchor,
            network,
            dfx,
        })
    }
}

/// Run a list subcommand or the default tree listing.
pub fn run<I>(args: I) -> Result<(), ListCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if args
        .first()
        .and_then(|arg| arg.to_str())
        .is_some_and(|arg| matches!(arg, "help" | "--help" | "-h"))
    {
        println!("{}", usage());
        return Ok(());
    }
    if args
        .first()
        .and_then(|arg| arg.to_str())
        .is_some_and(|arg| matches!(arg, "version" | "--version" | "-V"))
    {
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
    if should_use_local_replica_query(options) {
        return Ok(match local_query_ready(options, canister) {
            Ok(true) => ReadyStatus::Ready,
            Ok(false) => ReadyStatus::NotReady,
            Err(_) => ReadyStatus::Error,
        });
    }

    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    if let Some(network) = &options.network {
        command.args(["--network", network]);
    }
    command.args(["call", canister, "canic_ready", "--output", "json"]);

    let output = command.output()?;
    if !output.status.success() {
        return Ok(ReadyStatus::Error);
    }

    let data = serde_json::from_slice::<serde_json::Value>(&output.stdout)?;
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
    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    if let Some(network) = &options.network {
        command.args(["--network", network]);
    }
    command.args(["id", name]);

    let display = command_display(&command);
    let output = command.output()?;
    if output.status.success() {
        return Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ));
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if canister_id_missing(&stderr) {
        return Ok(None);
    }

    Err(ListCommandError::DfxFailed {
        command: display,
        stderr,
    })
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

    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    if let Some(network) = &options.network {
        command.args(["--network", network]);
    }
    command.args(["id", "root"]);
    run_output(&mut command)
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

// Resolve the state network using the same local default as installer commands.
fn state_network(options: &ListOptions) -> String {
    options
        .network
        .clone()
        .or_else(|| env::var("DFX_NETWORK").ok())
        .unwrap_or_else(|| "local".to_string())
}

// Run `dfx canister call <root> canic_subnet_registry --output json`.
fn call_subnet_registry(options: &ListOptions, root: &str) -> Result<String, ListCommandError> {
    if should_use_local_replica_query(options) {
        return local_query_subnet_registry(options, root);
    }

    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    if let Some(network) = &options.network {
        command.args(["--network", network]);
    }
    command.args(["call", root, "canic_subnet_registry", "--output", "json"]);
    run_output(&mut command).map_err(add_root_registry_hint)
}

// Use direct local replica queries because DFX local calls can fail when stdout is captured.
fn should_use_local_replica_query(options: &ListOptions) -> bool {
    options
        .network
        .as_deref()
        .is_none_or(|network| network == "local" || network.starts_with("http://"))
}

// Query `canic_subnet_registry` directly through the local replica HTTP API.
fn local_query_subnet_registry(
    options: &ListOptions,
    root: &str,
) -> Result<String, ListCommandError> {
    let bytes = local_query(options, root, "canic_subnet_registry")?;
    let result = Decode!(&bytes, Result<SubnetRegistryResponseWire, CanicErrorWire>)
        .map_err(|err| ListCommandError::ReplicaQuery(err.to_string()))?;
    let response = result.map_err(|err| ListCommandError::ReplicaQuery(err.to_string()))?;
    serde_json::to_string(&response.to_dfx_json()).map_err(ListCommandError::from)
}

// Query `canic_ready` directly through the local replica HTTP API.
fn local_query_ready(options: &ListOptions, canister: &str) -> Result<bool, ListCommandError> {
    let bytes = local_query(options, canister, "canic_ready")?;
    let ready =
        Decode!(&bytes, bool).map_err(|err| ListCommandError::ReplicaQuery(err.to_string()))?;
    Ok(ready)
}

// Execute one anonymous query call against the local replica.
fn local_query(
    options: &ListOptions,
    canister: &str,
    method: &str,
) -> Result<Vec<u8>, ListCommandError> {
    let canister_id = Principal::from_text(canister)
        .map_err(|err| ListCommandError::ReplicaQuery(err.to_string()))?;
    let arg = Encode!().map_err(|err| ListCommandError::ReplicaQuery(err.to_string()))?;
    let sender = Principal::anonymous();
    let envelope = QueryEnvelope {
        content: QueryContent {
            request_type: "query",
            canister_id: canister_id.as_slice(),
            method_name: method,
            arg: &arg,
            sender: sender.as_slice(),
            ingress_expiry: ingress_expiry_nanos()?,
        },
    };
    let body = serde_cbor::to_vec(&envelope)?;
    let endpoint = local_replica_endpoint(options);
    let response = post_cbor(
        &endpoint,
        &format!("/api/v2/canister/{canister}/query"),
        &body,
    )?;
    let query_response = serde_cbor::from_slice::<QueryResponse>(&response)?;

    if query_response.status == "replied" {
        return query_response
            .reply
            .map(|reply| reply.arg)
            .ok_or_else(|| ListCommandError::ReplicaQuery("missing query reply".to_string()));
    }

    Err(ListCommandError::ReplicaRejected {
        code: query_response.reject_code.unwrap_or_default(),
        message: query_response.reject_message.unwrap_or_default(),
    })
}

// Resolve the local replica endpoint from explicit URL or the current DFX port.
fn local_replica_endpoint(options: &ListOptions) -> String {
    if let Some(network) = options
        .network
        .as_deref()
        .filter(|network| network.starts_with("http://"))
    {
        return network.trim_end_matches('/').to_string();
    }

    let mut command = Command::new(&options.dfx);
    command.args(["info", "webserver-port"]);
    let port = run_output(&mut command).unwrap_or_else(|_| "4943".to_string());
    format!("http://127.0.0.1:{port}")
}

// Return an ingress expiry comfortably in the near future for local queries.
fn ingress_expiry_nanos() -> Result<u64, ListCommandError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| ListCommandError::ReplicaQuery(err.to_string()))?;
    let expiry = now
        .as_nanos()
        .saturating_add(5 * 60 * 1_000_000_000)
        .min(u128::from(u64::MAX));
    u64::try_from(expiry).map_err(|err| ListCommandError::ReplicaQuery(err.to_string()))
}

// POST one CBOR request over simple HTTP/1.1 and return the response body.
fn post_cbor(endpoint: &str, path: &str, body: &[u8]) -> Result<Vec<u8>, ListCommandError> {
    let (host, port) = parse_http_endpoint(endpoint)?;
    let mut stream = TcpStream::connect((host.as_str(), port))?;
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/cbor\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(request.as_bytes())?;
    stream.write_all(body)?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    split_http_body(&response)
}

// Parse the limited HTTP endpoints supported by local direct queries.
fn parse_http_endpoint(endpoint: &str) -> Result<(String, u16), ListCommandError> {
    let rest = endpoint.strip_prefix("http://").ok_or_else(|| {
        ListCommandError::ReplicaQuery(format!("unsupported endpoint {endpoint}"))
    })?;
    let authority = rest.split('/').next().unwrap_or(rest);
    let (host, port) = authority
        .rsplit_once(':')
        .ok_or_else(|| ListCommandError::ReplicaQuery(format!("missing port in {endpoint}")))?;
    let port = port
        .parse::<u16>()
        .map_err(|err| ListCommandError::ReplicaQuery(err.to_string()))?;
    Ok((host.to_string(), port))
}

// Split a simple HTTP response and reject non-2xx status codes.
fn split_http_body(response: &[u8]) -> Result<Vec<u8>, ListCommandError> {
    let marker = b"\r\n\r\n";
    let Some(index) = response
        .windows(marker.len())
        .position(|window| window == marker)
    else {
        return Err(ListCommandError::ReplicaQuery(
            "malformed HTTP response".to_string(),
        ));
    };
    let header = String::from_utf8_lossy(&response[..index]);
    let status_ok = header
        .lines()
        .next()
        .is_some_and(|status| status.contains(" 2"));
    if !status_ok {
        return Err(ListCommandError::ReplicaQuery(header.to_string()));
    }
    Ok(response[index + marker.len()..].to_vec())
}

// Execute one command and capture stdout.
fn run_output(command: &mut Command) -> Result<String, ListCommandError> {
    let display = command_display(command);
    let output = command.output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(ListCommandError::DfxFailed {
            command: display,
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
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

// Detect dfx's missing-canister-id diagnostic so standalone mode can skip uncreated entries.
fn canister_id_missing(stderr: &str) -> bool {
    stderr.contains("Cannot find canister id")
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

// Render a command for diagnostics.
fn command_display(command: &Command) -> String {
    let mut parts = vec![command.get_program().to_string_lossy().to_string()];
    parts.extend(
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string()),
    );
    parts.join(" ")
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
    let role_width = rows
        .iter()
        .map(|row| display_width(&role_label(row)))
        .chain([display_width(ROLE_HEADER)])
        .max()
        .unwrap_or_else(|| display_width(ROLE_HEADER));
    let canister_width = rows
        .iter()
        .map(|row| display_width(&canister_label(row)))
        .chain([display_width(CANISTER_HEADER)])
        .max()
        .unwrap_or_else(|| display_width(CANISTER_HEADER));
    let kind_width = rows
        .iter()
        .map(|row| display_width(&kind_label(row, role_kinds)))
        .chain([display_width(KIND_HEADER)])
        .max()
        .unwrap_or_else(|| display_width(KIND_HEADER));

    let mut lines = Vec::new();
    lines.push(registry_table_row(
        CANISTER_HEADER,
        ROLE_HEADER,
        KIND_HEADER,
        READY_HEADER,
        canister_width,
        role_width,
        kind_width,
    ));

    for row in rows {
        let ready = readiness
            .get(&row.entry.pid)
            .map_or("unknown", |status| status.label());
        lines.push(registry_table_row(
            &canister_label(row),
            &role_label(row),
            &kind_label(row, role_kinds),
            ready,
            canister_width,
            role_width,
            kind_width,
        ));
    }

    lines.join("\n")
}

// Render one whitespace-aligned table row.
fn registry_table_row(
    canister: &str,
    role: &str,
    kind: &str,
    ready: &str,
    canister_width: usize,
    role_width: usize,
    kind_width: usize,
) -> String {
    format!("{canister:<canister_width$}  {role:<role_width$}  {kind:<kind_width$}  {ready}")
}

// Count characters so Unicode tree prefixes do not over-pad columns.
fn display_width(value: &str) -> usize {
    value.chars().count()
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
/// QueryEnvelope
///

#[derive(Serialize)]
struct QueryEnvelope<'a> {
    content: QueryContent<'a>,
}

///
/// QueryContent
///

#[derive(Serialize)]
struct QueryContent<'a> {
    request_type: &'static str,
    #[serde(with = "serde_bytes")]
    canister_id: &'a [u8],
    method_name: &'a str,
    #[serde(with = "serde_bytes")]
    arg: &'a [u8],
    #[serde(with = "serde_bytes")]
    sender: &'a [u8],
    ingress_expiry: u64,
}

///
/// QueryResponse
///

#[derive(Deserialize)]
struct QueryResponse {
    status: String,
    reply: Option<QueryReply>,
    reject_code: Option<u64>,
    reject_message: Option<String>,
}

///
/// QueryReply
///

#[derive(Deserialize)]
struct QueryReply {
    #[serde(with = "serde_bytes")]
    arg: Vec<u8>,
}

///
/// SubnetRegistryResponseWire
///

#[derive(CandidType, Deserialize)]
struct SubnetRegistryResponseWire(Vec<SubnetRegistryEntryWire>);

impl SubnetRegistryResponseWire {
    // Convert direct Candid query output into the DFX JSON shape the discovery parser accepts.
    fn to_dfx_json(&self) -> serde_json::Value {
        serde_json::json!({
            "Ok": self.0.iter().map(SubnetRegistryEntryWire::to_dfx_json).collect::<Vec<_>>()
        })
    }
}

///
/// SubnetRegistryEntryWire
///

#[derive(CandidType, Deserialize)]
struct SubnetRegistryEntryWire {
    pid: Principal,
    role: String,
    record: CanisterInfoWire,
}

impl SubnetRegistryEntryWire {
    // Convert one registry entry into the DFX JSON shape used by existing list rendering.
    fn to_dfx_json(&self) -> serde_json::Value {
        serde_json::json!({
            "pid": self.pid.to_text(),
            "role": self.role,
            "record": self.record.to_dfx_json(),
        })
    }
}

///
/// CanisterInfoWire
///

#[derive(CandidType, Deserialize)]
struct CanisterInfoWire {
    pid: Principal,
    role: String,
    parent_pid: Option<Principal>,
    module_hash: Option<Vec<u8>>,
    created_at: u64,
}

impl CanisterInfoWire {
    // Convert one canister info record into a DFX-like JSON object.
    fn to_dfx_json(&self) -> serde_json::Value {
        serde_json::json!({
            "pid": self.pid.to_text(),
            "role": self.role,
            "parent_pid": self.parent_pid.as_ref().map(Principal::to_text),
            "module_hash": self.module_hash,
            "created_at": self.created_at.to_string(),
        })
    }
}

///
/// CanicErrorWire
///

#[derive(CandidType, Deserialize)]
struct CanicErrorWire {
    code: ErrorCodeWire,
    message: String,
}

impl std::fmt::Display for CanicErrorWire {
    // Render a compact public API error from a direct local replica query.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.message)
    }
}

///
/// ErrorCodeWire
///

#[derive(CandidType, Debug, Deserialize)]
enum ErrorCodeWire {
    Conflict,
    Forbidden,
    Internal,
    InvalidInput,
    InvariantViolation,
    NotFound,
    PolicyInstanceRequiresSingletonWithDirectory,
    PolicyReplicaRequiresSingletonWithScaling,
    PolicyRoleAlreadyRegistered,
    PolicyShardRequiresSingletonWithSharding,
    PolicySingletonAlreadyRegisteredUnderParent,
    ResourceExhausted,
    Unauthorized,
    Unavailable,
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

// Read the next required option value.
fn next_value<I>(args: &mut I, option: &'static str) -> Result<String, ListCommandError>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .and_then(|value| value.into_string().ok())
        .ok_or(ListCommandError::MissingValue(option))
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

    // Ensure the old root-tree flag is no longer part of the list surface.
    #[test]
    fn rejects_root_tree_list_options() {
        let err = ListOptions::parse([OsString::from("--root-tree")])
            .expect_err("root-tree should not parse");

        assert!(matches!(err, ListCommandError::UnknownOption(option) if option == "--root-tree"));
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

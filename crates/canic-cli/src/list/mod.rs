use crate::{
    snapshot::{RegistryEntry, SnapshotCommandError, parse_registry_entries},
    version_text,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs,
    process::Command,
};
use thiserror::Error as ThisError;

///
/// ListCommandError
///

#[derive(Debug, ThisError)]
pub enum ListCommandError {
    #[error("{0}")]
    Usage(&'static str),

    #[error("missing required option {0}")]
    MissingOption(&'static str),

    #[error("unknown option {0}")]
    UnknownOption(String),

    #[error("option {0} requires a value")]
    MissingValue(&'static str),

    #[error("cannot combine --root and --registry-json")]
    ConflictingRegistrySources,

    #[error("registry JSON did not contain the requested canister {0}")]
    CanisterNotInRegistry(String),

    #[error("dfx command failed: {command}\n{stderr}")]
    DfxFailed { command: String, stderr: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Snapshot(#[from] SnapshotCommandError),
}

///
/// ListOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListOptions {
    pub root: Option<String>,
    pub registry_json: Option<String>,
    pub canister: Option<String>,
    pub network: Option<String>,
    pub dfx: String,
}

impl ListOptions {
    /// Parse canister listing options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, ListCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut root = None;
        let mut registry_json = None;
        let mut canister = None;
        let mut network = None;
        let mut dfx = "dfx".to_string();

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| ListCommandError::Usage(usage()))?;
            match arg.as_str() {
                "--root" => root = Some(next_value(&mut args, "--root")?),
                "--registry-json" => {
                    registry_json = Some(next_value(&mut args, "--registry-json")?);
                }
                "--canister" => canister = Some(next_value(&mut args, "--canister")?),
                "--network" => network = Some(next_value(&mut args, "--network")?),
                "--dfx" => dfx = next_value(&mut args, "--dfx")?,
                "--help" | "-h" => return Err(ListCommandError::Usage(usage())),
                _ => return Err(ListCommandError::UnknownOption(arg)),
            }
        }

        if root.is_some() && registry_json.is_some() {
            return Err(ListCommandError::ConflictingRegistrySources);
        }

        Ok(Self {
            root,
            registry_json,
            canister,
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

    let options = ListOptions::parse(args)?;
    let registry = load_registry_entries(&options)?;
    println!(
        "{}",
        render_registry_tree(&registry, options.canister.as_deref())?
    );
    Ok(())
}

/// Render all registry entries, or one selected subtree, as an ASCII tree.
pub fn render_registry_tree(
    registry: &[RegistryEntry],
    canister: Option<&str>,
) -> Result<String, ListCommandError> {
    let by_pid = registry
        .iter()
        .map(|entry| (entry.pid.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let roots = root_entries(registry, &by_pid, canister)?;
    let children = child_entries(registry);
    let mut lines = Vec::new();

    for (index, root) in roots.iter().enumerate() {
        let last = index + 1 == roots.len();
        render_entry(root, &children, "", last, true, &mut lines);
    }

    Ok(lines.join("\n"))
}

// Load registry entries from a file or live root canister query.
fn load_registry_entries(options: &ListOptions) -> Result<Vec<RegistryEntry>, ListCommandError> {
    let registry_json = if let Some(path) = &options.registry_json {
        fs::read_to_string(path)?
    } else {
        let root = resolve_root_canister(options)?;
        call_subnet_registry(options, &root)?
    };

    parse_registry_entries(&registry_json).map_err(ListCommandError::from)
}

// Resolve the explicit root id or the current dfx project's `root` canister id.
fn resolve_root_canister(options: &ListOptions) -> Result<String, ListCommandError> {
    if let Some(root) = &options.root {
        return Ok(root.clone());
    }

    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    if let Some(network) = &options.network {
        command.args(["--network", network]);
    }
    command.args(["id", "root"]);
    run_output(&mut command)
}

// Run `dfx canister call <root> canic_subnet_registry --output json`.
fn call_subnet_registry(options: &ListOptions, root: &str) -> Result<String, ListCommandError> {
    let mut command = Command::new(&options.dfx);
    command.arg("canister");
    if let Some(network) = &options.network {
        command.args(["--network", network]);
    }
    command.args(["call", root, "canic_subnet_registry", "--output", "json"]);
    run_output(&mut command)
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

// Render one registry entry and its descendants.
fn render_entry(
    entry: &RegistryEntry,
    children: &BTreeMap<&str, Vec<&RegistryEntry>>,
    prefix: &str,
    last: bool,
    root: bool,
    lines: &mut Vec<String>,
) {
    if root {
        lines.push(entry_label(entry));
    } else {
        let branch = if last { "`- " } else { "|- " };
        lines.push(format!("{prefix}{branch}{}", entry_label(entry)));
    }

    let Some(child_entries) = children.get(entry.pid.as_str()) else {
        return;
    };

    let child_prefix = if root {
        String::new()
    } else if last {
        format!("{prefix}   ")
    } else {
        format!("{prefix}|  ")
    };

    for (index, child) in child_entries.iter().enumerate() {
        render_entry(
            child,
            children,
            &child_prefix,
            index + 1 == child_entries.len(),
            false,
            lines,
        );
    }
}

// Format one tree node label.
fn entry_label(entry: &RegistryEntry) -> String {
    match &entry.role {
        Some(role) if !role.is_empty() => format!("{role} {}", entry.pid),
        _ => format!("unknown {}", entry.pid),
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
    "usage: canic list [--root <root-canister> | --registry-json <file>] [--canister <id>] [--network <name>] [--dfx <path>]"
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ROOT: &str = "aaaaa-aa";
    const APP: &str = "renrk-eyaaa-aaaaa-aaada-cai";
    const WORKER: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";

    // Ensure list options parse live registry queries.
    #[test]
    fn parses_live_list_options() {
        let options = ListOptions::parse([
            OsString::from("--root"),
            OsString::from(ROOT),
            OsString::from("--canister"),
            OsString::from(APP),
            OsString::from("--network"),
            OsString::from("local"),
            OsString::from("--dfx"),
            OsString::from("/bin/dfx"),
        ])
        .expect("parse list options");

        assert_eq!(options.root, Some(ROOT.to_string()));
        assert_eq!(options.registry_json, None);
        assert_eq!(options.canister, Some(APP.to_string()));
        assert_eq!(options.network, Some("local".to_string()));
        assert_eq!(options.dfx, "/bin/dfx");
    }

    // Ensure list defaults to the current dfx project's root canister.
    #[test]
    fn parses_default_project_root_list_options() {
        let options = ListOptions::parse([OsString::from("--network"), OsString::from("local")])
            .expect("parse default root options");

        assert_eq!(options.root, None);
        assert_eq!(options.registry_json, None);
        assert_eq!(options.canister, None);
        assert_eq!(options.network, Some("local".to_string()));
        assert_eq!(options.dfx, "dfx");
    }

    // Ensure conflicting registry sources are still rejected.
    #[test]
    fn rejects_conflicting_registry_sources() {
        let err = ListOptions::parse([
            OsString::from("--root"),
            OsString::from(ROOT),
            OsString::from("--registry-json"),
            OsString::from("registry.json"),
        ])
        .expect_err("conflicting sources should fail");

        assert!(matches!(err, ListCommandError::ConflictingRegistrySources));
    }

    // Ensure registry entries render as a stable ASCII tree.
    #[test]
    fn renders_registry_ascii_tree() {
        let registry = parse_registry_entries(&registry_json()).expect("parse registry");
        let tree = render_registry_tree(&registry, None).expect("render tree");

        assert_eq!(
            tree,
            format!("root {ROOT}\n`- app {APP}\n   `- worker {WORKER}")
        );
    }

    // Ensure one selected subtree can be rendered without siblings.
    #[test]
    fn renders_selected_subtree() {
        let registry = parse_registry_entries(&registry_json()).expect("parse registry");
        let tree = render_registry_tree(&registry, Some(APP)).expect("render subtree");

        assert_eq!(tree, format!("app {APP}\n`- worker {WORKER}"));
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
                        "parent_pid": null
                    }
                },
                {
                    "pid": APP,
                    "role": "app",
                    "record": {
                        "pid": APP,
                        "role": "app",
                        "parent_pid": ROOT
                    }
                },
                {
                    "pid": WORKER,
                    "role": "worker",
                    "record": {
                        "pid": WORKER,
                        "role": "worker",
                        "parent_pid": [APP]
                    }
                }
            ]
        })
        .to_string()
    }
}

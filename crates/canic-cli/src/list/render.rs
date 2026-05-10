use super::ListCommandError;
use canic_backup::discovery::RegistryEntry;
use std::collections::{BTreeMap, BTreeSet};

pub(super) const ROLE_HEADER: &str = "ROLE";
pub(super) const KIND_HEADER: &str = "KIND";
pub(super) const FEATURES_HEADER: &str = "FEATURES";
pub(super) const AUTO_HEADER: &str = "AUTO";
pub(super) const TOPUP_HEADER: &str = "TOPUP";
pub(super) const METRICS_HEADER: &str = "METRICS";
pub(super) const CANISTER_HEADER: &str = "CANISTER_ID";
pub(super) const READY_HEADER: &str = "READY";
pub(super) const CANIC_HEADER: &str = "CANIC";
pub(super) const WASM_HEADER: &str = "WASM_GZ";
pub(super) const CYCLES_HEADER: &str = "CYCLES";
const LIST_COLUMN_GAP: &str = "   ";
const TREE_BRANCH: &str = "├─ ";
const TREE_LAST: &str = "└─ ";
const TREE_PIPE: &str = "│  ";
const TREE_SPACE: &str = "   ";

///
/// ListTitle
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ListTitle {
    pub(super) fleet: String,
    pub(super) network: String,
}

impl ListTitle {
    #[must_use]
    pub(super) fn render(&self) -> String {
        format!("Fleet: {} (network {})", self.fleet, self.network)
    }
}

///
/// ReadyStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReadyStatus {
    Ready,
    NotReady,
    Error,
}

impl ReadyStatus {
    const fn label(self) -> &'static str {
        match self {
            Self::Ready => "yes",
            Self::NotReady => "no",
            Self::Error => "error",
        }
    }
}

///
/// RegistryColumnData
///

pub(super) struct RegistryColumnData<'a> {
    pub(super) readiness: &'a BTreeMap<String, ReadyStatus>,
    pub(super) canic_versions: &'a BTreeMap<String, String>,
    pub(super) wasm_sizes: &'a BTreeMap<String, String>,
    pub(super) cycles: &'a BTreeMap<String, String>,
}

/// Render all registry entries, or one selected subtree, as a whitespace table.
pub(super) fn render_registry_tree(
    registry: &[RegistryEntry],
    canister: Option<&str>,
    columns: &RegistryColumnData<'_>,
) -> Result<String, ListCommandError> {
    let rows = visible_rows(registry, canister)?;
    Ok(render_registry_table(&rows, columns))
}

/// Render a named list view with a fleet/source title above the registry table.
pub(super) fn render_list_output(
    title: &ListTitle,
    registry: &[RegistryEntry],
    canister: Option<&str>,
    columns: &RegistryColumnData<'_>,
    missing_roles: &[String],
) -> Result<String, ListCommandError> {
    let mut output = format!(
        "{}\n\n{}",
        title.render(),
        render_registry_tree(registry, canister, columns)?
    );
    if !missing_roles.is_empty() {
        output.push_str("\n\nMissing roles: ");
        output.push_str(&missing_roles.join(", "));
    }
    Ok(output)
}

/// Render config-defined roles for a selected fleet that has not been installed yet.
pub(super) fn render_config_output(
    title: &ListTitle,
    rows: &[ConfigRoleRow],
    verbose: bool,
) -> String {
    format!(
        "{}\n\n{}",
        title.render(),
        render_config_table(rows, verbose)
    )
}

pub(super) fn visible_entries<'a>(
    registry: &'a [RegistryEntry],
    canister: Option<&str>,
) -> Result<Vec<&'a RegistryEntry>, ListCommandError> {
    Ok(visible_rows(registry, canister)?
        .into_iter()
        .map(|row| row.entry)
        .collect())
}

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

pub(super) struct RegistryRow<'a> {
    pub(super) entry: &'a RegistryEntry,
    pub(super) tree_prefix: String,
}

///
/// ConfigRoleRow
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ConfigRoleRow {
    pub(super) role: String,
    pub(super) kind: String,
    pub(super) capabilities: String,
    pub(super) auto_create: String,
    pub(super) topup: String,
    pub(super) metrics: String,
    pub(super) details: Vec<String>,
}

fn render_config_table(rows: &[ConfigRoleRow], verbose: bool) -> String {
    let table_rows = config_table_rows(rows);
    let widths = config_table_widths(&table_rows);
    let header = render_config_table_row(
        &[
            ROLE_HEADER,
            KIND_HEADER,
            AUTO_HEADER,
            FEATURES_HEADER,
            METRICS_HEADER,
            TOPUP_HEADER,
        ],
        &widths,
    );
    let separator = render_config_separator(&widths);
    let mut lines = Vec::new();
    lines.push(header);
    lines.push(separator);
    for (row, table_row) in rows.iter().zip(table_rows.iter()) {
        lines.push(render_config_table_row(table_row, &widths));
        if verbose {
            lines.extend(row.details.iter().map(|detail| format!("  - {detail}")));
        }
    }
    lines.join("\n")
}

fn config_table_rows(rows: &[ConfigRoleRow]) -> Vec<[String; 6]> {
    rows.iter()
        .map(|row| {
            [
                row.role.clone(),
                row.kind.clone(),
                row.auto_create.clone(),
                row.capabilities.clone(),
                row.metrics.clone(),
                row.topup.clone(),
            ]
        })
        .collect()
}

fn config_table_widths(rows: &[[String; 6]]) -> [usize; 6] {
    let mut widths = [
        ROLE_HEADER.chars().count(),
        KIND_HEADER.chars().count(),
        AUTO_HEADER.chars().count(),
        FEATURES_HEADER.chars().count(),
        METRICS_HEADER.chars().count(),
        TOPUP_HEADER.chars().count(),
    ];

    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }

    widths
}

fn render_config_table_row(row: &[impl AsRef<str>], widths: &[usize; 6]) -> String {
    widths
        .iter()
        .enumerate()
        .map(|(index, width)| {
            let value = row.get(index).map_or("", AsRef::as_ref);
            format!("{value:<width$}")
        })
        .collect::<Vec<_>>()
        .join(LIST_COLUMN_GAP)
        .trim_end()
        .to_string()
}

fn render_config_separator(widths: &[usize; 6]) -> String {
    widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>()
        .join(LIST_COLUMN_GAP)
}

fn render_registry_table(rows: &[RegistryRow<'_>], columns: &RegistryColumnData<'_>) -> String {
    let table_rows = registry_table_rows(rows, columns);
    let widths = registry_table_widths(&table_rows);
    let header = render_registry_table_row(
        &[
            ROLE_HEADER,
            CANISTER_HEADER,
            READY_HEADER,
            CANIC_HEADER,
            WASM_HEADER,
            CYCLES_HEADER,
        ],
        &widths,
    );
    let separator = render_registry_separator(&widths);
    let mut lines = Vec::with_capacity(table_rows.len() + 2);
    lines.push(header);
    lines.push(separator);
    lines.extend(
        table_rows
            .iter()
            .map(|row| render_registry_table_row(row, &widths)),
    );
    lines.join("\n")
}

fn registry_table_rows(
    rows: &[RegistryRow<'_>],
    columns: &RegistryColumnData<'_>,
) -> Vec<[String; 6]> {
    let mut table_rows = Vec::with_capacity(rows.len());
    for row in rows {
        let ready = columns
            .readiness
            .get(&row.entry.pid)
            .map_or("unknown", |status| status.label());
        let canic_version = columns
            .canic_versions
            .get(&row.entry.pid)
            .cloned()
            .unwrap_or_else(|| "-".to_string());
        let wasm_size = row
            .entry
            .role
            .as_deref()
            .and_then(|role| columns.wasm_sizes.get(role))
            .cloned()
            .unwrap_or_else(|| "-".to_string());
        let cycle_balance = columns
            .cycles
            .get(&row.entry.pid)
            .cloned()
            .unwrap_or_else(|| "-".to_string());
        table_rows.push([
            role_label(row),
            canister_label(row),
            ready.to_string(),
            canic_version,
            wasm_size,
            cycle_balance,
        ]);
    }
    table_rows
}

fn registry_table_widths(rows: &[[String; 6]]) -> [usize; 6] {
    let mut widths = [
        ROLE_HEADER.chars().count(),
        CANISTER_HEADER.chars().count(),
        READY_HEADER.chars().count(),
        CANIC_HEADER.chars().count(),
        WASM_HEADER.chars().count(),
        CYCLES_HEADER.chars().count(),
    ];

    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }

    widths
}

pub(super) fn render_registry_table_row(row: &[impl AsRef<str>], widths: &[usize; 6]) -> String {
    widths
        .iter()
        .enumerate()
        .map(|(index, width)| {
            let value = row.get(index).map_or("", AsRef::as_ref);
            if matches!(index, 4 | 5) {
                format!("{value:>width$}")
            } else {
                format!("{value:<width$}")
            }
        })
        .collect::<Vec<_>>()
        .join(LIST_COLUMN_GAP)
        .trim_end()
        .to_string()
}

pub(super) fn render_registry_separator(widths: &[usize; 6]) -> String {
    widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>()
        .join(LIST_COLUMN_GAP)
}

fn canister_label(row: &RegistryRow<'_>) -> String {
    row.entry.pid.clone()
}

fn role_label(row: &RegistryRow<'_>) -> String {
    let role = row.entry.role.as_deref().filter(|role| !role.is_empty());
    let label = match role {
        Some(role) => role.to_string(),
        None => "unknown".to_string(),
    };
    format!("{}{}", row.tree_prefix, label)
}

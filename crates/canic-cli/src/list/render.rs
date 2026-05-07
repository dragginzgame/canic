use super::ListCommandError;
use canic::ids::CanisterRole;
use canic_backup::discovery::RegistryEntry;
use std::collections::{BTreeMap, BTreeSet};

pub(super) const ROLE_HEADER: &str = "ROLE";
pub(super) const KIND_HEADER: &str = "KIND";
pub(super) const CANISTER_HEADER: &str = "CANISTER_ID";
pub(super) const READY_HEADER: &str = "READY";
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
    /// Render the compact title block shown above `canic list` tables.
    #[must_use]
    pub(super) fn render(&self) -> String {
        format!("Fleet: {}\nNetwork: {}", self.fleet, self.network)
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
    // Return the compact label used in list output.
    const fn label(self) -> &'static str {
        match self {
            Self::Ready => "yes",
            Self::NotReady => "no",
            Self::Error => "error",
        }
    }
}

/// Render all registry entries, or one selected subtree, as a whitespace table.
pub(super) fn render_registry_tree(
    registry: &[RegistryEntry],
    canister: Option<&str>,
    role_kinds: &BTreeMap<String, String>,
    readiness: &BTreeMap<String, ReadyStatus>,
) -> Result<String, ListCommandError> {
    let rows = visible_rows(registry, canister)?;
    Ok(render_registry_table(&rows, role_kinds, readiness))
}

/// Render a named list view with a fleet/source title above the registry table.
pub(super) fn render_list_output(
    title: &ListTitle,
    registry: &[RegistryEntry],
    canister: Option<&str>,
    role_kinds: &BTreeMap<String, String>,
    readiness: &BTreeMap<String, ReadyStatus>,
) -> Result<String, ListCommandError> {
    Ok(format!(
        "{}\n\n{}",
        title.render(),
        render_registry_tree(registry, canister, role_kinds, readiness)?
    ))
}

// Return the entries that would be rendered for the selected table.
pub(super) fn visible_entries<'a>(
    registry: &'a [RegistryEntry],
    canister: Option<&str>,
) -> Result<Vec<&'a RegistryEntry>, ListCommandError> {
    Ok(visible_rows(registry, canister)?
        .into_iter()
        .map(|row| row.entry)
        .collect())
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

pub(super) struct RegistryRow<'a> {
    pub(super) entry: &'a RegistryEntry,
    pub(super) tree_prefix: String,
}

// Render registry rows as stable whitespace-aligned columns.
fn render_registry_table(
    rows: &[RegistryRow<'_>],
    role_kinds: &BTreeMap<String, String>,
    readiness: &BTreeMap<String, ReadyStatus>,
) -> String {
    let table_rows = registry_table_rows(rows, role_kinds, readiness);
    let widths = registry_table_widths(&table_rows);
    let header = render_registry_table_row(
        &[CANISTER_HEADER, ROLE_HEADER, KIND_HEADER, READY_HEADER],
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

// Collect rendered cell values before width calculation.
fn registry_table_rows(
    rows: &[RegistryRow<'_>],
    role_kinds: &BTreeMap<String, String>,
    readiness: &BTreeMap<String, ReadyStatus>,
) -> Vec<[String; 4]> {
    let mut table_rows = Vec::with_capacity(rows.len());
    for row in rows {
        let ready = readiness
            .get(&row.entry.pid)
            .map_or("unknown", |status| status.label());
        table_rows.push([
            canister_label(row),
            role_label(row),
            kind_label(row, role_kinds),
            ready.to_string(),
        ]);
    }
    table_rows
}

// Compute display widths for the list table, including headers.
fn registry_table_widths(rows: &[[String; 4]]) -> [usize; 4] {
    let mut widths = [
        CANISTER_HEADER.chars().count(),
        ROLE_HEADER.chars().count(),
        KIND_HEADER.chars().count(),
        READY_HEADER.chars().count(),
    ];

    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }

    widths
}

// Render one padded list table row with the wider list-specific column gap.
pub(super) fn render_registry_table_row(row: &[impl AsRef<str>], widths: &[usize; 4]) -> String {
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

// Render the line under the table headers.
pub(super) fn render_registry_separator(widths: &[usize; 4]) -> String {
    widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>()
        .join(LIST_COLUMN_GAP)
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
pub(super) fn kind_label(row: &RegistryRow<'_>, role_kinds: &BTreeMap<String, String>) -> String {
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

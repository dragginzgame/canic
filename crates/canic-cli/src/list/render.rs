use super::{
    ListCommandError,
    tree::{RegistryRow, visible_rows},
};
use canic_backup::discovery::RegistryEntry;
use canic_host::table::{
    ColumnAlign, render_separator, render_table, render_table_row, table_widths,
};
use std::collections::BTreeMap;

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
const CONFIG_HEADERS: [&str; 6] = [
    ROLE_HEADER,
    KIND_HEADER,
    AUTO_HEADER,
    FEATURES_HEADER,
    METRICS_HEADER,
    TOPUP_HEADER,
];
const CONFIG_ALIGNMENTS: [ColumnAlign; 6] = [ColumnAlign::Left; 6];
const REGISTRY_HEADERS: [&str; 6] = [
    ROLE_HEADER,
    CANISTER_HEADER,
    READY_HEADER,
    CANIC_HEADER,
    WASM_HEADER,
    CYCLES_HEADER,
];
const REGISTRY_ALIGNMENTS: [ColumnAlign; 6] = [
    ColumnAlign::Left,
    ColumnAlign::Left,
    ColumnAlign::Left,
    ColumnAlign::Left,
    ColumnAlign::Right,
    ColumnAlign::Right,
];

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
    let widths = table_widths(&CONFIG_HEADERS, &table_rows);
    let mut lines = Vec::new();
    lines.push(render_table_row(
        &CONFIG_HEADERS,
        &widths,
        &CONFIG_ALIGNMENTS,
    ));
    lines.push(render_separator(&widths));
    for (row, table_row) in rows.iter().zip(table_rows.iter()) {
        lines.push(render_table_row(table_row, &widths, &CONFIG_ALIGNMENTS));
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

fn render_registry_table(rows: &[RegistryRow<'_>], columns: &RegistryColumnData<'_>) -> String {
    let table_rows = registry_table_rows(rows, columns);
    render_table(&REGISTRY_HEADERS, &table_rows, &REGISTRY_ALIGNMENTS)
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

#[cfg(test)]
pub(super) fn render_registry_table_row(row: &[impl AsRef<str>], widths: &[usize; 6]) -> String {
    render_table_row(row, widths, &REGISTRY_ALIGNMENTS)
}

#[cfg(test)]
pub(super) fn render_registry_separator(widths: &[usize; 6]) -> String {
    render_separator(widths)
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

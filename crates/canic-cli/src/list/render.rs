use super::{
    ListCommandError,
    tree::{RegistryRow, visible_rows},
};
use canic_backup::discovery::RegistryEntry;
use canic_host::table::{
    ColumnAlign, render_separator, render_table, render_table_row, table_widths,
};
use std::collections::BTreeMap;

const COLUMN_GAP: &str = "   ";
const MODULE_PREFIX_CHARS: usize = 8;
const MODULE_VARIANT_COLOR: &str = "\x1b[38;5;179m";
const COLOR_RESET: &str = "\x1b[0m";
const MODULE_COLUMN_INDEX: usize = 1;

pub(super) const ROLE_HEADER: &str = "ROLE";
pub(super) const KIND_HEADER: &str = "KIND";
pub(super) const FEATURES_HEADER: &str = "FEATURES";
pub(super) const AUTO_HEADER: &str = "AUTO";
pub(super) const TOPUP_HEADER: &str = "TOPUP";
pub(super) const METRICS_HEADER: &str = "METRICS";
pub(super) const CANISTER_HEADER: &str = "CANISTER_ID";
pub(super) const MODULE_HEADER: &str = "MODULE";
pub(super) const MODULE_HASH_HEADER: &str = "MODULE_HASH";
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
const REGISTRY_HEADERS: [&str; 7] = [
    ROLE_HEADER,
    MODULE_HEADER,
    CANISTER_HEADER,
    READY_HEADER,
    CANIC_HEADER,
    WASM_HEADER,
    CYCLES_HEADER,
];
const REGISTRY_VERBOSE_HEADERS: [&str; 7] = [
    ROLE_HEADER,
    MODULE_HASH_HEADER,
    CANISTER_HEADER,
    READY_HEADER,
    CANIC_HEADER,
    WASM_HEADER,
    CYCLES_HEADER,
];
const REGISTRY_ALIGNMENTS: [ColumnAlign; 7] = [
    ColumnAlign::Left,
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
    pub(super) module_hashes: &'a BTreeMap<String, String>,
    pub(super) wasm_sizes: &'a BTreeMap<String, String>,
    pub(super) cycles: &'a BTreeMap<String, String>,
    pub(super) full_module_hashes: bool,
    pub(super) color_module_variants: bool,
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
    let headers = if columns.full_module_hashes {
        &REGISTRY_VERBOSE_HEADERS
    } else {
        &REGISTRY_HEADERS
    };
    if !columns.color_module_variants {
        return render_table(headers, &table_rows, &REGISTRY_ALIGNMENTS);
    }

    let widths = table_widths(headers, &table_rows);
    let variants = module_variant_flags(rows, columns.module_hashes);
    let mut lines = Vec::with_capacity(table_rows.len() + 2);
    lines.push(render_table_row(headers, &widths, &REGISTRY_ALIGNMENTS));
    lines.push(render_separator(&widths));
    lines.extend(
        table_rows
            .iter()
            .zip(variants)
            .map(|(row, variant)| render_registry_row(row, &widths, variant)),
    );
    lines.join("\n")
}

fn registry_table_rows(
    rows: &[RegistryRow<'_>],
    columns: &RegistryColumnData<'_>,
) -> Vec<[String; 7]> {
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
            module_hash_label(row, columns.module_hashes, columns.full_module_hashes),
            canister_label(row),
            ready.to_string(),
            canic_version,
            wasm_size,
            cycle_balance,
        ]);
    }
    table_rows
}

fn render_registry_row(row: &[String; 7], widths: &[usize; 7], color_module: bool) -> String {
    widths
        .iter()
        .zip(REGISTRY_ALIGNMENTS)
        .enumerate()
        .map(|(index, (width, alignment))| {
            let value = &row[index];
            let cell = match alignment {
                ColumnAlign::Left => format!("{value:<width$}"),
                ColumnAlign::Right => format!("{value:>width$}"),
            };
            if color_module && index == MODULE_COLUMN_INDEX {
                format!("{MODULE_VARIANT_COLOR}{cell}{COLOR_RESET}")
            } else {
                cell
            }
        })
        .collect::<Vec<_>>()
        .join(COLUMN_GAP)
        .trim_end()
        .to_string()
}

fn module_variant_flags(
    rows: &[RegistryRow<'_>],
    module_hashes: &BTreeMap<String, String>,
) -> Vec<bool> {
    let baseline = module_variant_baseline(rows, module_hashes);
    rows.iter()
        .map(|row| {
            row_module_hash(row, module_hashes)
                .zip(baseline)
                .is_some_and(|(hash, baseline)| hash != baseline)
        })
        .collect()
}

fn module_variant_baseline<'a>(
    rows: &[RegistryRow<'_>],
    module_hashes: &'a BTreeMap<String, String>,
) -> Option<&'a str> {
    rows.iter()
        .filter(|row| is_module_baseline_role(row.entry.role.as_deref()))
        .find_map(|row| row_module_hash(row, module_hashes))
        .or_else(|| {
            rows.iter()
                .find_map(|row| row_module_hash(row, module_hashes))
        })
}

fn row_module_hash<'a>(
    row: &RegistryRow<'_>,
    module_hashes: &'a BTreeMap<String, String>,
) -> Option<&'a str> {
    module_hashes
        .get(&row.entry.pid)
        .map(String::as_str)
        .filter(|hash| !hash.is_empty())
}

fn is_module_baseline_role(role: Option<&str>) -> bool {
    !matches!(role, Some("root" | "wasm_store"))
}

#[cfg(test)]
pub(super) fn render_registry_table_row(row: &[impl AsRef<str>], widths: &[usize; 7]) -> String {
    render_table_row(row, widths, &REGISTRY_ALIGNMENTS)
}

#[cfg(test)]
pub(super) fn render_registry_separator(widths: &[usize; 7]) -> String {
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

fn module_hash_label(
    row: &RegistryRow<'_>,
    module_hashes: &BTreeMap<String, String>,
    full: bool,
) -> String {
    let Some(hash) = module_hashes
        .get(&row.entry.pid)
        .filter(|hash| !hash.is_empty())
    else {
        return "-".to_string();
    };
    if full {
        return hash.clone();
    }
    hash.chars().take(MODULE_PREFIX_CHARS).collect()
}

use super::ListCommandError;
use crate::support::registry_tree::{RegistryRow, visible_rows};
use canic_host::registry::RegistryEntry;
use ic_host_text::{
    escape::terminal_text,
    table::{self, Alignment, Options},
};
use std::collections::{BTreeMap, BTreeSet};

const MODULE_PREFIX_CHARS: usize = 8;
const MODULE_VARIANT_COLOR: &str = "\x1b[38;5;179m";
const COLOR_RESET: &str = "\x1b[0m";

const ROLE_HEADER: &str = "ROLE";
const KIND_HEADER: &str = "KIND";
const CAPS_HEADER: &str = "CAPS";
const AUTO_HEADER: &str = "AUTO";
const TOPUP_HEADER: &str = "TOPUP";
const METRICS_HEADER: &str = "METRICS";
const CANISTER_HEADER: &str = "CANISTER_ID";
const MODULE_HEADER: &str = "MODULE";
const MODULE_HASH_HEADER: &str = "MODULE_HASH";
const READY_HEADER: &str = "READY";
const CANIC_HEADER: &str = "CANIC";
const WASM_HEADER: &str = "WASM";
const CYCLES_HEADER: &str = "CYCLES";
const CONFIG_HEADERS: [&str; 6] = [
    ROLE_HEADER,
    KIND_HEADER,
    AUTO_HEADER,
    CAPS_HEADER,
    METRICS_HEADER,
    TOPUP_HEADER,
];
const CONFIG_ALIGNMENTS: [Alignment; 6] = [Alignment::Left; 6];
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
const REGISTRY_ALIGNMENTS: [Alignment; 7] = [
    Alignment::Left,
    Alignment::Left,
    Alignment::Left,
    Alignment::Left,
    Alignment::Left,
    Alignment::Right,
    Alignment::Right,
];

///
/// ListTitle
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ListTitle {
    pub(super) source: ListTitleSource,
    pub(super) name: String,
    pub(super) environment: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ListTitleSource {
    Fleet,
    App,
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

/// Render a named list view with a Fleet/source title above the registry table.
pub(super) fn render_list_output(
    title: &ListTitle,
    registry: &[RegistryEntry],
    canister: Option<&str>,
    columns: &RegistryColumnData<'_>,
    missing_roles: &[String],
) -> Result<String, ListCommandError> {
    let mut output = format!(
        "{}\n\n{}",
        render_list_title(title),
        render_registry_tree(registry, canister, columns)?
    );
    if !missing_roles.is_empty() {
        output.push_str("\n\nMissing roles: ");
        output.push_str(&terminal_text(&missing_roles.join(", ")));
    }
    Ok(output)
}

/// Render config-defined roles for a selected App that has not been installed yet.
pub(super) fn render_config_output(
    title: &ListTitle,
    rows: &[ConfigRoleRow],
    verbose: bool,
) -> String {
    format!(
        "{}\n\n{}",
        render_list_title(title),
        render_config_table(rows, verbose)
    )
}

fn render_list_title(title: &ListTitle) -> String {
    let label = match title.source {
        ListTitleSource::Fleet => "Fleet",
        ListTitleSource::App => "App",
    };
    format!(
        "{label}: {} (environment {})",
        terminal_text(&title.name),
        terminal_text(&title.environment)
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
    let rendered = table::render(
        &CONFIG_HEADERS,
        &table_rows,
        &CONFIG_ALIGNMENTS,
        Options {
            column_gap: 3,
            header_separator: true,
        },
    );
    let mut rendered_lines = rendered.lines();
    let mut lines = rendered_lines
        .by_ref()
        .take(2)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    for (row, line) in rows.iter().zip(rendered_lines) {
        lines.push(line.to_owned());
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
        return table::render(
            headers,
            &table_rows,
            &REGISTRY_ALIGNMENTS,
            Options {
                column_gap: 3,
                header_separator: true,
            },
        );
    }

    let rendered = table::render(
        headers,
        &table_rows,
        &REGISTRY_ALIGNMENTS,
        Options {
            column_gap: 3,
            header_separator: true,
        },
    );
    let variants = module_variant_flags(rows, columns.module_hashes);
    let mut rendered_lines = rendered.lines();
    let mut lines = rendered_lines
        .by_ref()
        .take(2)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    lines.extend(rendered_lines.zip(variants).map(|(line, variant)| {
        if variant {
            format!("{MODULE_VARIANT_COLOR}{line}{COLOR_RESET}")
        } else {
            line.to_owned()
        }
    }));
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

fn module_variant_flags(
    rows: &[RegistryRow<'_>],
    module_hashes: &BTreeMap<String, String>,
) -> Vec<bool> {
    let baselines = role_module_baselines(rows, module_hashes);
    let distinct = role_distinct_module_hashes(rows, module_hashes);
    rows.iter()
        .map(|row| {
            row_role(row)
                .and_then(|role| {
                    row_module_hash(row, module_hashes)
                        .zip(baselines.get(role).map(String::as_str))
                        .zip(distinct.get(role))
                })
                .is_some_and(|((hash, baseline), role_hashes)| {
                    role_hashes.len() > 1 && hash != baseline
                })
        })
        .collect()
}

fn role_module_baselines(
    rows: &[RegistryRow<'_>],
    module_hashes: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut baselines = BTreeMap::new();
    for row in rows {
        if let Some((role, hash)) = row_role(row).zip(row_module_hash(row, module_hashes)) {
            baselines
                .entry(role.to_string())
                .or_insert_with(|| hash.to_string());
        }
    }
    baselines
}

fn role_distinct_module_hashes(
    rows: &[RegistryRow<'_>],
    module_hashes: &BTreeMap<String, String>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut distinct = BTreeMap::<String, BTreeSet<String>>::new();
    for row in rows {
        if let Some((role, hash)) = row_role(row).zip(row_module_hash(row, module_hashes)) {
            distinct
                .entry(role.to_string())
                .or_default()
                .insert(hash.to_string());
        }
    }
    distinct
}

fn row_role<'a>(row: &RegistryRow<'a>) -> Option<&'a str> {
    row.entry.role.as_deref().filter(|role| !role.is_empty())
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

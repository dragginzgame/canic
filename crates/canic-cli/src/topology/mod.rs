//! Layout of the declared application tree, independent of controller observations.

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

pub(crate) const ROOT_BADGE: &str = "[ROOT]";
pub(crate) const STORE_BADGE: &str = "[WASM_STORE]";

pub(crate) fn infrastructure_badge(role: &str) -> Option<&'static str> {
    match role {
        "root" => Some(ROOT_BADGE),
        "wasm-store" => Some(STORE_BADGE),
        "coordinator" => Some("[COORDINATOR]"),
        _ => None,
    }
}

/// Order report records by declared parent, retaining context for filtered builds.
pub(crate) fn labeled<'a, T>(
    records: &'a [T],
    slot: impl Fn(&'a T) -> (&'a str, Option<&'a str>),
) -> Vec<(String, &'a T)> {
    let by_name: BTreeMap<_, _> = records.iter().map(|item| (slot(item).0, item)).collect();
    rows(records.iter().map(&slot))
        .into_iter()
        .map(|(name, prefix)| {
            let item = by_name[name];
            let label = match slot(item).1 {
                Some(parent) if !by_name.contains_key(parent) => {
                    format!("{name} (parent: {parent})")
                }
                _ => format!("{prefix}{name}"),
            };
            (label, item)
        })
        .collect()
}

/// Slots and branch prefixes in stable parent-before-child order.
pub(crate) fn rows<'a>(
    parents: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
) -> Vec<(&'a str, String)> {
    let parents: BTreeMap<_, _> = parents.into_iter().collect();
    let mut rows = Vec::new();
    let mut visited = BTreeSet::new();
    for (slot, parent) in &parents {
        if parent.is_none_or(|parent| !parents.contains_key(parent)) {
            walk(&parents, slot, "", "", &mut visited, &mut rows);
        }
    }
    // Loaded configuration is validated. Keep malformed synthetic/partial snapshots
    // finite and visible rather than dropping an entire cycle from a report.
    for slot in parents.keys() {
        if !visited.contains(slot) {
            walk(&parents, slot, "", "", &mut visited, &mut rows);
        }
    }
    rows
}

fn walk<'a>(
    parents: &BTreeMap<&'a str, Option<&'a str>>,
    slot: &'a str,
    prefix: &str,
    carry: &str,
    visited: &mut BTreeSet<&'a str>,
    rows: &mut Vec<(&'a str, String)>,
) {
    if !visited.insert(slot) {
        return;
    }
    rows.push((slot, prefix.into()));
    let children: Vec<_> = parents
        .iter()
        .filter(|(_, parent)| **parent == Some(slot))
        .map(|(name, _)| *name)
        .collect();
    for (index, child) in children.iter().enumerate() {
        let last = index + 1 == children.len();
        walk(
            parents,
            child,
            &format!("{carry}{}", if last { "└─ " } else { "├─ " }),
            &format!("{carry}{}", if last { "   " } else { "│  " }),
            visited,
            rows,
        );
    }
}

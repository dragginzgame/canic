//! Layout of the declared application tree, independent of controller observations.

use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
mod tests;

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

//! Layout of the declared application tree, independent of controller observations.

use canic_host::FleetConfig;
use std::collections::BTreeSet;

#[cfg(test)]
mod tests;

/// Slots and branch prefixes in stable parent-before-child order.
pub(crate) fn rows(config: &FleetConfig) -> Vec<(&str, String)> {
    let mut rows = Vec::new();
    let mut visited = BTreeSet::new();
    for (slot, declaration) in &config.canisters {
        if declaration
            .parent
            .as_ref()
            .is_none_or(|parent| !config.canisters.contains_key(parent))
        {
            walk(config, slot, "", "", &mut visited, &mut rows);
        }
    }
    // Loaded configuration is validated. Keep malformed synthetic/partial snapshots
    // finite and visible rather than dropping an entire cycle from a report.
    for slot in config.canisters.keys() {
        if !visited.contains(slot.as_str()) {
            walk(config, slot, "", "", &mut visited, &mut rows);
        }
    }
    rows
}

fn walk<'a>(
    config: &'a FleetConfig,
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
    let children: Vec<_> = config
        .canisters
        .iter()
        .filter(|(_, child)| child.parent.as_deref() == Some(slot))
        .map(|(name, _)| name.as_str())
        .collect();
    for (index, child) in children.iter().enumerate() {
        let last = index + 1 == children.len();
        walk(
            config,
            child,
            &format!("{carry}{}", if last { "└─ " } else { "├─ " }),
            &format!("{carry}{}", if last { "   " } else { "│  " }),
            visited,
            rows,
        );
    }
}

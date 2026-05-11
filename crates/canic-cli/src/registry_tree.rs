use canic_backup::discovery::RegistryEntry;
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

const TREE_BRANCH: &str = "├─ ";
const TREE_LAST: &str = "└─ ";
const TREE_PIPE: &str = "│  ";
const TREE_SPACE: &str = "   ";

///
/// RegistryTreeError
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryTreeError {
    CanisterNotInRegistry(String),
}

impl fmt::Display for RegistryTreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CanisterNotInRegistry(canister) => {
                write!(
                    f,
                    "registry JSON did not contain the requested canister {canister}"
                )
            }
        }
    }
}

impl Error for RegistryTreeError {}

///
/// RegistryRow
///

pub struct RegistryRow<'a> {
    pub entry: &'a RegistryEntry,
    pub tree_prefix: String,
}

pub fn registry_rows(registry: &[RegistryEntry]) -> Vec<RegistryRow<'_>> {
    let roots = root_entries(registry);
    let children = child_entries(registry);
    let mut entries = Vec::new();

    for root in roots {
        collect_visible_entry(root, &children, "", "", &mut entries);
    }

    entries
}

pub fn visible_entries<'a>(
    registry: &'a [RegistryEntry],
    canister: Option<&str>,
) -> Result<Vec<&'a RegistryEntry>, RegistryTreeError> {
    Ok(visible_rows(registry, canister)?
        .into_iter()
        .map(|row| row.entry)
        .collect())
}

pub fn visible_rows<'a>(
    registry: &'a [RegistryEntry],
    canister: Option<&str>,
) -> Result<Vec<RegistryRow<'a>>, RegistryTreeError> {
    let Some(canister) = canister else {
        return Ok(registry_rows(registry));
    };

    let by_pid = registry
        .iter()
        .map(|entry| (entry.pid.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let root = by_pid
        .get(canister)
        .copied()
        .ok_or_else(|| RegistryTreeError::CanisterNotInRegistry(canister.to_string()))?;
    let children = child_entries(registry);
    let mut entries = Vec::new();
    collect_visible_entry(root, &children, "", "", &mut entries);
    Ok(entries)
}

fn root_entries(registry: &[RegistryEntry]) -> Vec<&RegistryEntry> {
    let ids = registry
        .iter()
        .map(|entry| entry.pid.as_str())
        .collect::<BTreeSet<_>>();
    registry
        .iter()
        .filter(|entry| {
            entry
                .parent_pid
                .as_deref()
                .is_none_or(|parent| !ids.contains(parent))
        })
        .collect()
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

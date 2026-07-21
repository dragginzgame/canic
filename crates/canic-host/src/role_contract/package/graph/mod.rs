//! Module: role_contract::package::graph
//!
//! Responsibility: correlate one package-selected Cargo tree with its metadata catalog.
//! Does not own: role dependency policy or user-facing diagnostics.
//! Boundary: raw Cargo labels and package IDs remain inside package validation.

#[cfg(test)]
mod tests;

use crate::cargo_metadata::{
    CargoMetadata, CargoMetadataNode, CargoMetadataNodeDependency, CargoMetadataPackage,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

pub(super) const TREE_FIELD_SEPARATOR: char = '\t';
pub(super) const TREE_FORMAT: &str = "{p}\t{lib}\t{f}";

const MAX_TREE_LINES: usize = 100_000;
const MAX_TREE_LINE_BYTES: usize = 16 * 1024;
const MAX_TREE_DEPTH: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CargoGraphEvidence {
    pub selected_package_id: String,
    pub workspace_root: PathBuf,
    pub packages: BTreeMap<String, CargoGraphPackage>,
    pub edges: BTreeMap<String, Vec<CargoGraphEdge>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CargoGraphPackage {
    pub name: String,
    pub version: String,
    pub source: Option<String>,
    pub manifest_path: PathBuf,
    pub enabled_features: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct CargoGraphEdge {
    pub alias: String,
    pub package_id: String,
}

pub(super) fn correlate_package_tree(
    catalog: &CargoMetadata,
    target_metadata: &CargoMetadata,
    selected: &CargoMetadataPackage,
    tree: &str,
) -> Result<CargoGraphEvidence, String> {
    if tree.lines().count() > MAX_TREE_LINES {
        return Err("package-selected Cargo tree exceeds the supported line limit".to_string());
    }

    let node_by_id = target_metadata
        .resolve
        .as_ref()
        .ok_or_else(|| "Cargo metadata omitted the dependency catalog".to_string())?
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let mut state = CorrelationState {
        catalog,
        selected,
        node_by_id,
        packages: BTreeMap::new(),
        edges: BTreeMap::new(),
        stack: Vec::new(),
        saw_root: false,
    };

    for (line_index, line) in tree.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        state.push_line(line, line_index + 1)?;
    }

    state.finish()
}

struct CorrelationState<'catalog, 'target> {
    catalog: &'catalog CargoMetadata,
    selected: &'catalog CargoMetadataPackage,
    node_by_id: BTreeMap<&'target str, &'target CargoMetadataNode>,
    packages: BTreeMap<String, CargoGraphPackage>,
    edges: BTreeMap<String, BTreeSet<CargoGraphEdge>>,
    stack: Vec<(String, bool)>,
    saw_root: bool,
}

impl CorrelationState<'_, '_> {
    fn push_line(&mut self, line: &str, line_number: usize) -> Result<(), String> {
        if line.len() > MAX_TREE_LINE_BYTES {
            return Err(format!(
                "package-selected Cargo tree line {line_number} exceeds the supported byte limit"
            ));
        }
        let row = parse_tree_row(line, line_number)?;
        if row.depth > MAX_TREE_DEPTH {
            return Err(format!(
                "package-selected Cargo tree line {line_number} exceeds the supported depth limit"
            ));
        }
        let package = resolve_tree_package(self.catalog, &row).map_err(|reason| {
            format!("{reason} on package-selected Cargo tree line {line_number}")
        })?;
        self.validate_position(&row, package, line_number)?;
        let active_alias = self.active_alias(&row, package, line_number)?;
        let is_active = row.depth == 0 || active_alias.is_some();
        if is_active {
            self.insert_active_package(package, row.enabled_features, line_number)?;
            if let Some(alias) = active_alias {
                let parent_id = self.stack[row.depth - 1].0.clone();
                self.edges
                    .entry(parent_id)
                    .or_default()
                    .insert(CargoGraphEdge {
                        alias,
                        package_id: package.id.clone(),
                    });
            }
        }
        self.stack.truncate(row.depth);
        self.stack.push((package.id.clone(), is_active));
        Ok(())
    }

    fn validate_position(
        &mut self,
        row: &TreeRow,
        package: &CargoMetadataPackage,
        line_number: usize,
    ) -> Result<(), String> {
        if !self.saw_root {
            if row.depth != 0 || package.id != self.selected.id {
                return Err(
                    "package-selected Cargo tree does not begin with the selected role package"
                        .to_string(),
                );
            }
            self.saw_root = true;
        } else if row.depth == 0 {
            return Err("package-selected Cargo tree contains more than one root".to_string());
        }
        if row.depth > self.stack.len() {
            return Err(format!(
                "package-selected Cargo tree skips a parent depth on line {line_number}"
            ));
        }
        Ok(())
    }

    fn active_alias(
        &self,
        row: &TreeRow,
        package: &CargoMetadataPackage,
        line_number: usize,
    ) -> Result<Option<String>, String> {
        if row.depth == 0 {
            return Ok(None);
        }
        let (parent_id, parent_active) = self.stack.get(row.depth - 1).ok_or_else(|| {
            format!("package-selected Cargo tree has no parent on line {line_number}")
        })?;
        if *parent_active {
            let parent_node = self.node_by_id.get(parent_id.as_str()).ok_or_else(|| {
                format!(
                    "target-filtered Cargo metadata omits an active parent on tree line {line_number}"
                )
            })?;
            target_normal_edge_alias(&parent_node.deps, &package.id).map_err(|reason| {
                format!("{reason} on package-selected Cargo tree line {line_number}")
            })
        } else {
            Ok(None)
        }
    }

    fn insert_active_package(
        &mut self,
        package: &CargoMetadataPackage,
        enabled_features: BTreeSet<String>,
        line_number: usize,
    ) -> Result<(), String> {
        let package_fact = CargoGraphPackage {
            name: package.name.clone(),
            version: package.version.clone(),
            source: package.source.clone(),
            manifest_path: package.manifest_path.clone(),
            enabled_features,
        };
        if let Some(existing) = self.packages.get_mut(&package.id) {
            if existing.name != package_fact.name
                || existing.version != package_fact.version
                || existing.source != package_fact.source
                || existing.manifest_path != package_fact.manifest_path
            {
                return Err(format!(
                    "package-selected Cargo tree reports inconsistent facts on line {line_number}"
                ));
            }
            existing
                .enabled_features
                .extend(package_fact.enabled_features);
        } else {
            self.packages.insert(package.id.clone(), package_fact);
        }
        Ok(())
    }

    fn finish(self) -> Result<CargoGraphEvidence, String> {
        if !self.saw_root {
            return Err("package-selected Cargo tree is empty".to_string());
        }
        Ok(CargoGraphEvidence {
            selected_package_id: self.selected.id.clone(),
            workspace_root: self.catalog.workspace_root.clone(),
            packages: self.packages,
            edges: self
                .edges
                .into_iter()
                .map(|(parent, children)| (parent, children.into_iter().collect()))
                .collect(),
        })
    }
}

struct TreeRow {
    depth: usize,
    label: String,
    library_name: String,
    enabled_features: BTreeSet<String>,
}

fn parse_tree_row(line: &str, line_number: usize) -> Result<TreeRow, String> {
    let depth_bytes = line.bytes().take_while(u8::is_ascii_digit).count();
    if depth_bytes == 0 {
        return Err(format!(
            "package-selected Cargo tree line {line_number} has no depth prefix"
        ));
    }
    let depth = line[..depth_bytes].parse::<usize>().map_err(|_| {
        format!("package-selected Cargo tree line {line_number} has an invalid depth")
    })?;
    let fields = line[depth_bytes..]
        .split(TREE_FIELD_SEPARATOR)
        .collect::<Vec<_>>();
    let [label, library_name, features] = fields.as_slice() else {
        return Err(format!(
            "package-selected Cargo tree line {line_number} has an invalid field count"
        ));
    };
    if label.is_empty() || library_name.is_empty() {
        return Err(format!(
            "package-selected Cargo tree line {line_number} omits package identity"
        ));
    }

    let enabled_features = if features.is_empty() {
        BTreeSet::new()
    } else {
        let parsed = features
            .split(',')
            .map(ToString::to_string)
            .collect::<BTreeSet<_>>();
        if parsed.iter().any(String::is_empty) {
            return Err(format!(
                "package-selected Cargo tree line {line_number} has an empty feature"
            ));
        }
        parsed
    };

    Ok(TreeRow {
        depth,
        label: (*label).to_string(),
        library_name: (*library_name).to_string(),
        enabled_features,
    })
}

fn resolve_tree_package<'a>(
    metadata: &'a CargoMetadata,
    row: &TreeRow,
) -> Result<&'a CargoMetadataPackage, String> {
    let mut candidates = metadata
        .packages
        .iter()
        .filter(|package| package_label_matches(package, &row.label))
        .filter(|package| {
            package.targets.iter().any(|target| {
                target.name == row.library_name
                    && target.kind.iter().any(|kind| {
                        matches!(
                            kind.as_str(),
                            "lib" | "rlib" | "cdylib" | "staticlib" | "proc-macro"
                        )
                    })
            })
        })
        .collect::<Vec<_>>();

    if candidates.len() > 1 {
        candidates.retain(|package| {
            package.manifest_path.parent().is_some_and(|directory| {
                row.label.ends_with(&format!(" ({})", directory.display()))
            })
        });
    }

    match candidates.as_slice() {
        [package] => Ok(*package),
        [] => Err("package-selected Cargo tree contains an unknown package".to_string()),
        _ => Err("package-selected Cargo tree contains an ambiguous package".to_string()),
    }
}

fn package_label_matches(package: &CargoMetadataPackage, label: &str) -> bool {
    let prefix = format!("{} v{}", package.name, package.version);
    label == prefix || label.starts_with(&format!("{prefix} "))
}

fn target_normal_edge_alias(
    dependencies: &[CargoMetadataNodeDependency],
    child_package_id: &str,
) -> Result<Option<String>, String> {
    let aliases = dependencies
        .iter()
        .filter(|dependency| dependency.pkg == child_package_id)
        .filter(|dependency| dependency.dep_kinds.iter().any(|kind| kind.kind.is_none()))
        .map(|dependency| dependency.name.clone())
        .collect::<BTreeSet<_>>();

    match aliases.into_iter().collect::<Vec<_>>().as_slice() {
        [alias] => Ok(Some(alias.clone())),
        [] => Ok(None),
        _ => Err("Cargo metadata has ambiguous normal dependency aliases".to_string()),
    }
}

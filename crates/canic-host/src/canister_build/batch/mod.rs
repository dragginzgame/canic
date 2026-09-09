//! Module: canister_build::batch
//!
//! Responsibility: batch packages only when Cargo preserves each resolved dependency tree.
//! Does not own: feature selection, role contracts, or runtime compilation.
//! Boundary: shared dependency feature unification must not change a role's isolated build.

#[cfg(test)]
mod tests;

use crate::{canister_build::WorkspaceBuildContext, cargo_command};
use std::{collections::BTreeMap, path::Path};

/// Partition packages without changing any package's resolved normal/build dependencies.
pub(super) fn compatible_package_groups(
    context: &WorkspaceBuildContext,
    workspace: &Path,
    packages: &[String],
) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    let mut isolated = BTreeMap::new();
    let mut groups: Vec<Vec<String>> = Vec::new();
    for package in packages {
        let tree = resolved_trees(context, workspace, std::slice::from_ref(package))?;
        if tree.len() != 1 {
            return Err("Cargo returned additional isolated package roots".into());
        }
        let selected = tree
            .get(package)
            .ok_or("Cargo omitted the selected package tree")?;
        isolated.insert(package.clone(), selected.clone());
        let mut selected_group = None;
        for (index, group) in groups.iter().enumerate() {
            let mut candidate = group.clone();
            candidate.push(package.clone());
            let combined = resolved_trees(context, workspace, &candidate)?;
            if preserves_individual_trees(&isolated, &combined, &candidate) {
                selected_group = Some(index);
                break;
            }
        }
        if let Some(index) = selected_group {
            groups[index].push(package.clone());
        } else {
            groups.push(vec![package.clone()]);
        }
    }
    Ok(groups)
}

fn resolved_trees(
    context: &WorkspaceBuildContext,
    workspace: &Path,
    packages: &[String],
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let mut command = cargo_command();
    context.apply_to_command(&mut command);
    command.current_dir(workspace).args([
        "tree",
        "--color",
        "never",
        "--locked",
        "--target",
        "wasm32-unknown-unknown",
        "--edges",
        "normal,build",
        "--prefix",
        "depth",
        "--no-dedupe",
        "--format",
        "{p}\t{f}",
    ]);
    for package in packages {
        command.arg("--package").arg(package);
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "Cargo batch feature inspection failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    parse_trees(&String::from_utf8(output.stdout)?)
}

fn parse_trees(text: &str) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let mut trees = BTreeMap::new();
    let mut root = None;
    for line in text.lines().filter(|line| !line.is_empty()) {
        if let Some(package) = line.strip_prefix('0') {
            let name = package
                .split_whitespace()
                .next()
                .ok_or("empty Cargo tree root")?;
            if trees.insert(name.to_string(), String::new()).is_some() {
                return Err("duplicate Cargo tree root".into());
            }
            root = Some(name.to_string());
        }
        let tree = root
            .as_ref()
            .and_then(|name| trees.get_mut(name))
            .ok_or("Cargo dependency has no selected root")?;
        tree.push_str(line);
        tree.push('\n');
    }
    if trees.is_empty() {
        return Err("empty Cargo dependency forest".into());
    }
    Ok(trees)
}

fn preserves_individual_trees(
    isolated: &BTreeMap<String, String>,
    combined: &BTreeMap<String, String>,
    packages: &[String],
) -> bool {
    combined.len() == packages.len()
        && packages.iter().all(|name| {
            isolated
                .get(name)
                .is_some_and(|expected| combined.get(name) == Some(expected))
        })
}

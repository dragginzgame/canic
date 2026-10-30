//! Module: canister_build::reuse::dependencies
//!
//! Responsibility: fingerprint Cargo-recorded includes outside ordinary package roots.
//! Does not own: Cargo freshness, compilation, or generated-source authority.
//! Boundary: unknown dependency syntax declines reuse instead of dropping an input.

use crate::canister_build::{
    WorkspaceBuildContext,
    cache::{canister_build_target_root, declaration_target_root},
    reuse::BuildReuseError,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

pub(super) fn generated_target_names(
    context: &WorkspaceBuildContext,
) -> Result<BTreeSet<String>, BuildReuseError> {
    crate::fleet_package::generated_manifests(&context.workspace_root, &context.config_path)
        .into_iter()
        .map(|manifest| {
            manifest
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .map(|name| name.replace('-', "_"))
                .ok_or_else(|| BuildReuseError::Unsupported(manifest.clone()))
        })
        .collect()
}

pub(super) fn append_observed_cargo_inputs(
    context: &WorkspaceBuildContext,
    selected_targets: &BTreeSet<String>,
    paths: &mut BTreeSet<PathBuf>,
) -> Result<(), BuildReuseError> {
    let runtime = resolve_existing_path(&canister_build_target_root(&context.workspace_root))?;
    let intermediate = std::env::var_os("CARGO_BUILD_BUILD_DIR").map(|value| {
        let path = std::path::PathBuf::from(value);
        if path.is_absolute() {
            path
        } else {
            context.workspace_root.join(path)
        }
    });
    let intermediate = intermediate
        .map(|path| resolve_existing_path(&path))
        .transpose()?;
    let generated =
        crate::fleet_package::generated_root(&context.workspace_root, &context.config_path);
    let generated = resolve_existing_path(&generated)?;
    let mut outputs = vec![runtime.clone(), generated];
    let mut targets = vec![runtime, declaration_target_root(&context.workspace_root)];
    if let Some(intermediate) = intermediate {
        outputs.push(intermediate.clone());
        targets.push(intermediate);
    }
    targets.sort();
    targets.dedup();
    let mut directories = Vec::new();
    for target in targets {
        let profile = target
            .join("wasm32-unknown-unknown")
            .join(context.profile.target_dir_name());
        directories.push(profile);
    }
    // Shared target directories also contain other Apps and framework probes.
    // Only the selected Cargo graph and generated Fleet targets own this recipe.
    for directory in directories {
        for target in selected_targets {
            let record = directory.join(format!("{target}.d"));
            let inputs = match ic_build::artifacts::read_cargo_depfile_inputs(
                &context.workspace_root,
                &record,
                &outputs,
                16 * 1024 * 1024,
            ) {
                Ok(inputs) => inputs,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
                    return Err(BuildReuseError::Evidence(error.to_string()));
                }
                Err(error) => return Err(error.into()),
            };
            paths.extend(inputs);
        }
    }
    Ok(())
}

/// Select conservative configuration roots; IC Build owns bounded include traversal.
pub(super) fn append_cargo_configuration_inputs(
    roots: &BTreeSet<PathBuf>,
    paths: &mut BTreeSet<PathBuf>,
) -> Result<(), BuildReuseError> {
    use ic_build::cargo::environment::{
        configuration_directories, observe_configurations, preferred_configuration,
    };
    let environment: BTreeMap<_, _> = crate::build_environment::inputs().into_iter().collect();
    let mut directories = BTreeSet::new();
    for root in roots {
        directories.extend(configuration_directories(
            root,
            environment
                .get(OsStr::new("CARGO_HOME"))
                .map(OsString::as_os_str),
            environment.get(OsStr::new("HOME")).map(OsString::as_os_str),
        )?);
    }
    // Both names retain absence/replacement custody even though Cargo prefers config.
    for directory in &directories {
        paths.insert(directory.join("config"));
        paths.insert(directory.join("config.toml"));
    }
    let selected: Vec<_> = directories
        .iter()
        .filter_map(|directory| preferred_configuration(directory))
        .collect();
    for observed in
        observe_configurations(&selected, crate::canister_build::CARGO_CONFIGURATION_LIMITS)?
    {
        paths.insert(observed.path);
    }
    Ok(())
}

pub(super) fn resolve_existing_path(path: &Path) -> Result<PathBuf, BuildReuseError> {
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path.to_path_buf()),
        Err(error) => Err(error.into()),
    }
}

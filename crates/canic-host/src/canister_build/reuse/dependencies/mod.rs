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
    collections::BTreeSet,
    path::{Path, PathBuf},
};

pub(super) fn generated_target_names(
    context: &WorkspaceBuildContext,
) -> Result<BTreeSet<String>, BuildReuseError> {
    crate::fleet_package::generated_manifests(&context.config_path)
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
    let generated = context
        .config_path
        .parent()
        .ok_or_else(|| BuildReuseError::Unsupported(context.config_path.clone()))?
        .join(".canic/generated");
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

pub(super) fn resolve_existing_path(path: &Path) -> Result<PathBuf, BuildReuseError> {
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path.to_path_buf()),
        Err(error) => Err(error.into()),
    }
}

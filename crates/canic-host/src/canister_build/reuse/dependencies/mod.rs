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
    fs,
    path::{Component, Path, PathBuf},
};

pub(super) fn append_observed_cargo_inputs(
    context: &WorkspaceBuildContext,
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
        append_generated_exports(&profile.join("build"), &outputs, paths)?;
        directories.push(profile);
    }
    paths.extend(
        ic_build::artifacts::discover_cargo_depfile_inputs(
            &context.workspace_root,
            &directories,
            &outputs,
            16 * 1024 * 1024,
        )
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::InvalidData {
                BuildReuseError::Evidence(error.to_string())
            } else {
                BuildReuseError::Io(error)
            }
        })?,
    );
    Ok(())
}

// Retained build-script output can name includes before a role's .d record exists.
// Observe those bytes now; never grant authority to a path first exported after compilation.
fn append_generated_exports(
    build: &Path,
    outputs: &[PathBuf],
    paths: &mut BTreeSet<PathBuf>,
) -> Result<(), BuildReuseError> {
    let entries = match fs::read_dir(build) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let record = entry.path().join("output");
        let bytes = match ic_host_fs::read::read_file_no_follow(&record, 16 * 1024 * 1024)
            .map_err(std::io::Error::from)
        {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        let text = std::str::from_utf8(&bytes)
            .map_err(|error| BuildReuseError::Evidence(error.to_string()))?;
        for line in text.lines() {
            let Some((name, value)) = line
                .strip_prefix("cargo:rustc-env=")
                .or_else(|| line.strip_prefix("cargo::rustc-env="))
                .and_then(|export| export.split_once('='))
            else {
                continue;
            };
            if matches!(
                name,
                "CANIC_CONFIG_SOURCE_PATH"
                    | "CANIC_CONFIG_MODEL_PATH"
                    | "CANIC_ROLE_RUNTIME_AUTHORITY_PATH"
            ) {
                append_input(Path::new(value), outputs, paths)?;
            }
        }
    }
    Ok(())
}

fn append_input(
    input: &Path,
    outputs: &[PathBuf],
    paths: &mut BTreeSet<PathBuf>,
) -> Result<(), BuildReuseError> {
    if !input.is_absolute() {
        return Err(BuildReuseError::Unsupported(input.to_path_buf()));
    }
    let input = resolve_existing_path(input)?;
    // Classify resolved paths: an alias can enter an output root, and `..` can
    // leave it. Unresolved parent traversal never proves generated ownership.
    if !input.components().any(|part| part == Component::ParentDir)
        && outputs.iter().any(|root| input.starts_with(root))
    {
        return Ok(());
    }
    paths.insert(input);
    Ok(())
}

fn resolve_existing_path(path: &Path) -> Result<PathBuf, BuildReuseError> {
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path.to_path_buf()),
        Err(error) => Err(error.into()),
    }
}

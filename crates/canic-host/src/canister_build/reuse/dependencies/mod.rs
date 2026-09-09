//! Module: canister_build::reuse::dependencies
//!
//! Responsibility: fingerprint Cargo-recorded includes outside ordinary package roots.
//! Does not own: Cargo freshness, compilation, or generated-source authority.
//! Boundary: unknown dependency syntax declines reuse instead of dropping an input.

#[cfg(test)]
mod tests;

use crate::canister_build::{
    WorkspaceBuildContext,
    cache::{canister_build_target_root, declaration_target_root},
    reuse::{BuildReuseError, add_optional, collect_files},
};
use std::{collections::BTreeMap, fs, path::Path};

pub(super) fn append_observed_cargo_inputs(
    context: &WorkspaceBuildContext,
    files: &mut BTreeMap<String, String>,
) -> Result<(), BuildReuseError> {
    let runtime = canister_build_target_root(&context.workspace_root);
    let intermediate = std::env::var_os("CARGO_BUILD_BUILD_DIR").map(|value| {
        let path = std::path::PathBuf::from(value);
        if path.is_absolute() {
            path
        } else {
            context.workspace_root.join(path)
        }
    });
    let generated = context
        .config_path
        .parent()
        .ok_or_else(|| BuildReuseError::Unsupported(context.config_path.clone()))?
        .join(".canic/generated");
    for target in [
        runtime.clone(),
        declaration_target_root(&context.workspace_root),
    ] {
        let profile = target
            .join("wasm32-unknown-unknown")
            .join(context.profile.target_dir_name());
        let entries = match fs::read_dir(profile) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        for entry in entries {
            let path = entry?.path();
            if path.extension().and_then(|value| value.to_str()) != Some("d") {
                continue;
            }
            let bytes = crate::durable_io::read_regular_bytes(&path, 16 * 1024 * 1024)?;
            let text = std::str::from_utf8(&bytes)
                .map_err(|error| BuildReuseError::Evidence(error.to_string()))?;
            for input in dependency_paths(text)? {
                let input = Path::new(&input);
                if !input.is_absolute() {
                    return Err(BuildReuseError::Unsupported(input.to_path_buf()));
                }
                if input.starts_with(&runtime)
                    || intermediate
                        .as_ref()
                        .is_some_and(|root| input.starts_with(root))
                    || input.starts_with(&generated)
                {
                    continue;
                }
                let input = match input.canonicalize() {
                    Ok(path) => path,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        input.to_path_buf()
                    }
                    Err(error) => return Err(error.into()),
                };
                if input.is_dir() {
                    collect_files(&input, &input, files, true)?;
                } else {
                    add_optional(&input, files)?;
                }
            }
        }
    }
    Ok(())
}

fn dependency_paths(text: &str) -> Result<Vec<String>, BuildReuseError> {
    let (_, dependencies) = text.split_once(": ").ok_or_else(|| {
        BuildReuseError::Evidence("Cargo dependency record has no target separator".into())
    })?;
    let mut paths = Vec::new();
    let mut path = String::new();
    let mut chars = dependencies.chars();
    while let Some(character) = chars.next() {
        match character {
            '\\' => match chars.next() {
                Some('\n') => {}
                Some(character @ (' ' | '\t' | '\\' | '#' | ':')) => path.push(character),
                _ => {
                    return Err(BuildReuseError::Evidence(
                        "unsupported Cargo path escape".into(),
                    ));
                }
            },
            '$' => {
                if chars.next() != Some('$') {
                    return Err(BuildReuseError::Evidence(
                        "unsupported Cargo path expansion".into(),
                    ));
                }
                path.push('$');
            }
            character if character.is_ascii_whitespace() => {
                if !path.is_empty() {
                    paths.push(std::mem::take(&mut path));
                }
            }
            character => path.push(character),
        }
    }
    if !path.is_empty() {
        paths.push(path);
    }
    if paths.is_empty() {
        return Err(BuildReuseError::Evidence(
            "empty Cargo dependency input set".into(),
        ));
    }
    Ok(paths)
}

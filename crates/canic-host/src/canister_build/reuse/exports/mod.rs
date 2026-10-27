//! Retain Canic include exports from exact Cargo invocation messages.
//!
//! Selected package identities admit records; directory names never identify producers.

#[cfg(test)]
mod tests;

use super::{BuildReuseError, dependencies::resolve_existing_path, hash_field, hex_bytes};
use crate::canister_build::{WorkspaceBuildContext, cache::canister_build_target_root};
use ic_build::cargo::observation::schema::{Message, TargetKind};
use ic_host_fs::{
    durable::{PublicationMode, WriteOptions, write_bytes},
    read::read_file_no_follow,
};
use serde::{Deserialize, Serialize};
use sha2_host::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    path::{Component, Path, PathBuf},
};

const RECORD_LIMIT: usize = 16 * 1024 * 1024;

/// Current invocation's include inputs, bound to one exact Cargo producer.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProducerExportsRecord {
    schema_version: u8,
    recipe: String,
    package_id: String,
    manifest_path: PathBuf,
    inputs: BTreeSet<PathBuf>,
}

pub fn retain(
    context: &WorkspaceBuildContext,
    messages: &[Message],
) -> Result<(), BuildReuseError> {
    let mut producers = BTreeMap::new();
    for message in messages {
        let Message::CompilerArtifact(artifact) = message else {
            continue;
        };
        if artifact.target.kind.contains(&TargetKind::CustomBuild) {
            let package = artifact.package_id.repr.as_str();
            let manifest = artifact.manifest_path.canonicalize()?;
            if producers
                .insert(package.to_owned(), manifest.clone())
                .is_some_and(|old| old != manifest)
            {
                return Err(BuildReuseError::Evidence(
                    "ambiguous Cargo build-script producer".into(),
                ));
            }
        }
    }
    let recipe = recipe_key(context)?;
    let mut records = BTreeMap::<PathBuf, ProducerExportsRecord>::new();
    for message in messages {
        let Message::BuildScriptExecuted(script) = message else {
            continue;
        };
        let package = script.package_id.repr.as_str();
        let manifest = producers.get(package).ok_or_else(|| {
            BuildReuseError::Evidence("Cargo build-script message has no exact producer".into())
        })?;
        let record = records
            .entry(manifest.clone())
            .or_insert_with(|| ProducerExportsRecord {
                schema_version: 1,
                recipe: recipe.clone(),
                package_id: package.to_owned(),
                manifest_path: manifest.clone(),
                inputs: BTreeSet::new(),
            });
        if record.package_id != package {
            return Err(BuildReuseError::Evidence(
                "ambiguous Cargo export identity".into(),
            ));
        }
        record.inputs.extend(
            script
                .env
                .iter()
                .filter(|(name, _)| {
                    matches!(
                        name.as_str(),
                        "CANIC_CONFIG_SOURCE_PATH"
                            | "CANIC_CONFIG_MODEL_PATH"
                            | "CANIC_ROLE_RUNTIME_AUTHORITY_PATH"
                    )
                })
                .map(|(_, value)| PathBuf::from(value)),
        );
    }
    let records = records
        .into_iter()
        .map(|(manifest, record)| {
            let bytes = serde_json::to_vec(&record)?;
            if bytes.len() > RECORD_LIMIT {
                return Err(BuildReuseError::Unsupported(manifest));
            }
            Ok((record_path(context, &manifest)?, bytes))
        })
        .collect::<Result<Vec<_>, BuildReuseError>>()?;
    for (path, bytes) in records {
        write_bytes(
            &path,
            WriteOptions {
                mode: PublicationMode::Replace,
                permissions: 0o666,
            },
            &bytes,
        )
        .map_err(crate::publication::ops::io_error)?;
    }
    Ok(())
}

pub(super) fn append_observed(
    context: &WorkspaceBuildContext,
    producers: &BTreeMap<PathBuf, String>,
    paths: &mut BTreeSet<PathBuf>,
) -> Result<(), BuildReuseError> {
    let mut outputs = vec![
        canister_build_target_root(&context.workspace_root),
        crate::fleet_package::generated_root(&context.workspace_root, &context.config_path),
    ];
    if let Some(path) = std::env::var_os("CARGO_BUILD_BUILD_DIR") {
        let path = PathBuf::from(path);
        outputs.push(if path.is_absolute() {
            path
        } else {
            context.workspace_root.join(path)
        });
    }
    let outputs = outputs
        .iter()
        .map(|path| resolve_existing_path(path))
        .collect::<Result<Vec<_>, _>>()?;
    let recipe = recipe_key(context)?;
    for (manifest, package) in producers {
        let manifest = manifest.canonicalize()?;
        let bytes = match read_file_no_follow(&record_path(context, &manifest)?, RECORD_LIMIT)
            .map_err(io::Error::from)
        {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        let record: ProducerExportsRecord = serde_json::from_slice(&bytes)?;
        if record.schema_version != 1
            || record.recipe != recipe
            || record.package_id != *package
            || record.manifest_path != manifest
        {
            return Err(BuildReuseError::Evidence(
                "retained Cargo export producer differs".into(),
            ));
        }
        for input in record.inputs {
            if !input.is_absolute() {
                return Err(BuildReuseError::Unsupported(input));
            }
            let input = resolve_existing_path(&input)?;
            if input.components().any(|part| part == Component::ParentDir)
                || !outputs.iter().any(|root| input.starts_with(root))
            {
                paths.insert(input);
            }
        }
    }
    Ok(())
}

fn record_path(
    context: &WorkspaceBuildContext,
    manifest: &Path,
) -> Result<PathBuf, BuildReuseError> {
    Ok(context
        .icp_root
        .join(".canic/build-script-exports")
        .join(recipe_key(context)?)
        .join(format!(
            "{}.json",
            hex_bytes(Sha256::digest(manifest.as_os_str().as_encoded_bytes()))
        )))
}

fn recipe_key(context: &WorkspaceBuildContext) -> Result<String, BuildReuseError> {
    let mut recipe = Sha256::new();
    for field in [
        context
            .config_path
            .canonicalize()?
            .as_os_str()
            .as_encoded_bytes(),
        context.profile.target_dir_name().as_bytes(),
        context.build_network.as_str().as_bytes(),
        context
            .workspace_root
            .canonicalize()?
            .as_os_str()
            .as_encoded_bytes(),
        resolve_existing_path(&canister_build_target_root(&context.workspace_root))?
            .as_os_str()
            .as_encoded_bytes(),
    ] {
        hash_field(&mut recipe, field);
    }
    let mut environment = crate::build_environment::inputs();
    environment.sort();
    for (key, value) in environment {
        hash_field(&mut recipe, key.as_encoded_bytes());
        hash_field(&mut recipe, value.as_encoded_bytes());
    }
    Ok(hex_bytes(recipe.finalize()))
}

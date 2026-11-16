//! App recipes reuse the artifact builder; Cargo owns dependency resolution and locking.
use crate::{Artifact, Error, Result, supplier};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

/// A replaceable build output, not retained deployment evidence.
#[derive(Debug, Serialize)]
pub struct BuiltCanister {
    /// Shared app definition name.
    pub canister: String,
    /// Output path relative to the invoking directory.
    pub wasm: PathBuf,
    /// Exact built bytes.
    pub artifact: Artifact,
}

/// Build all declared app recipes or one selected canister, initializing absent workspace locks once.
/// All recipes are validated before building; selection does not hide invalid app declarations.
/// Existing locks are preserved and compilation always uses `--locked`.
/// # Errors
/// Refuses unknown canisters, missing recipes, non-workspace manifests, duplicate output paths or failed builds.
pub fn build_app(
    path: &Path,
    app: Option<&str>,
    canister: Option<&str>,
) -> Result<Vec<BuiltCanister>> {
    let config = crate::load_config(path)?;
    let (_, app) = config.select_app(app)?;
    if app.canisters.is_empty() {
        return Err(Error::Invalid("app has no canister build recipes".into()));
    }
    if let Some(name) = canister
        && !app.canisters.contains_key(name)
    {
        return Err(Error::Invalid(format!("app has no canister named {name}")));
    }
    let base = path.parent().unwrap_or(Path::new("."));
    let working_directory = std::env::current_dir().map_err(supplier)?;
    let mut recipes = Vec::new();
    let mut outputs = std::collections::BTreeSet::new();
    for (name, canister) in &app.canisters {
        let recipe = canister.build.as_ref().ok_or_else(|| Error::Invalid(format!("canister {name} has no build recipe; add build.manifest and build.package to its app definition")))?;
        let manifest = base
            .join(&recipe.manifest)
            .canonicalize()
            .map_err(supplier)?;
        let bytes = crate::store::read(&manifest, 65_536)?;
        let value: toml::Value =
            toml::from_str(std::str::from_utf8(&bytes).map_err(supplier)?).map_err(supplier)?;
        if value.get("workspace").is_none() {
            return Err(Error::Invalid(
                "build.manifest must name the workspace manifest".into(),
            ));
        }
        let output = base.join(
            canister
                .wasm
                .as_ref()
                .ok_or_else(|| Error::Invalid("build recipe needs a Wasm output".into()))?,
        );
        // Compare physical destinations, including missing outputs. Keep the
        // original path for the artifact writer's independent safety checks.
        let destination =
            crate::output_path::resolve(&output, &working_directory).map_err(supplier)?;
        if !outputs.insert(destination) {
            return Err(Error::Invalid("app build outputs must be distinct".into()));
        }
        recipes.push((name, recipe, manifest, output));
    }
    let mut built = Vec::new();
    for (name, recipe, manifest, output) in recipes {
        if canister.is_some_and(|selected| selected != name) {
            continue;
        }
        let workspace = manifest
            .parent()
            .ok_or_else(|| Error::Invalid("manifest has no parent".into()))?;
        if !workspace
            .join("Cargo.lock")
            .try_exists()
            .map_err(supplier)?
        {
            crate::process::capture(
                Command::new("cargo")
                    .current_dir(workspace)
                    .args(["generate-lockfile", "--manifest-path"])
                    .arg(&manifest),
                1024 * 1024,
                1024 * 1024,
                Duration::from_secs(120),
            )
            .map_err(crate::artifact::cargo_failure)?;
        }
        let wasm = crate::artifact::build(&manifest, &recipe.package, &[], Some(&output))?;
        built.push(BuiltCanister {
            canister: name.clone(),
            artifact: crate::inspect(&wasm)?,
            wasm,
        });
    }
    Ok(built)
}

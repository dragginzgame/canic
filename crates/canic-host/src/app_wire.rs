//! Fleetless application ABI qualification from the selected build recipes.
use crate::{Error, Result, supplier};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io::Write as _,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

/// Check each application actor's complete Wire inventory against its shipped Wasm
/// and the app-owned baseline beside canic.toml.
/// # Errors
/// A missing or changed baseline retains a separately generated candidate for review.
pub fn check_wire(path: &Path, selected: Option<&str>) -> Result<PathBuf> {
    let (baseline, candidate) = wire_manifest(path, selected)?;
    if baseline.try_exists().map_err(supplier)?
        && baseline.metadata().map_err(supplier)?.len() == candidate.len() as u64
        && crate::store::read(&baseline, candidate.len())? == candidate
    {
        return Ok(baseline);
    }
    let mut file = tempfile::Builder::new()
        .prefix("canic-wire-")
        .suffix(".json")
        .tempfile()
        .map_err(supplier)?;
    file.write_all(&candidate).map_err(supplier)?;
    let (_, retained) = file.keep().map_err(supplier)?;
    Err(Error::Invalid(format!(
        "Wire baseline missing or changed: {}\nReview generated candidate: {}",
        baseline.display(),
        retained.display()
    )))
}

/// Build the selected application and generate its export-checked canonical JSON.
/// Returns the expected baseline path and candidate bytes for external CI comparison.
/// # Errors
/// Refuses missing recipes, failed builds, missing schemas and mismatched exports.
pub fn wire_manifest(path: &Path, selected: Option<&str>) -> Result<(PathBuf, Vec<u8>)> {
    let path = path.canonicalize().map_err(supplier)?;
    let base = path
        .parent()
        .ok_or_else(|| Error::Invalid("config needs a parent directory".into()))?;
    let config = crate::load_config(&path)?;
    let (app, declaration) = config.select_app(selected)?;
    let baseline = base.join("wire-baseline").join(app).join("contracts.json");
    let mut inventories = BTreeMap::new();
    for (name, actor) in &declaration.canisters {
        let recipe = actor
            .build
            .as_ref()
            .ok_or_else(|| Error::Invalid(format!("{name} has no build recipe")))?;
        if matches!(
            recipe.package.as_str(),
            "canic-root" | "ic-wasm-store" | "canic-dormant"
        ) {
            continue;
        }
        let built = crate::build_app(&path, Some(app), Some(name))?;
        let exports = crate::endpoints(&built[0].wasm)?;
        let manifest = base
            .join(&recipe.manifest)
            .canonicalize()
            .map_err(supplier)?;
        let methods = inspect_actor(name, recipe, actor.role.as_deref(), &manifest, &exports)?;
        inventories.insert(name, methods);
    }
    if inventories.is_empty() {
        return Err(Error::Invalid(format!(
            "{app} has no application actors to inspect"
        )));
    }
    let mut candidate = serde_json::to_vec_pretty(&inventories).map_err(supplier)?;
    candidate.push(b'\n');
    Ok((baseline, candidate))
}

fn inspect_actor(
    name: &str,
    recipe: &crate::BuildInput,
    role: Option<&str>,
    manifest: &Path,
    exports: &[crate::WasmEndpoint],
) -> Result<BTreeMap<String, Value>> {
    let mut features = recipe.features.clone();
    features.push("inspection".into());
    let mut command = Command::new("cargo");
    let role_input = format!("CANIC_APP_ROLE_{}", recipe.package);
    if let Some(role) = role {
        command.env(role_input, role);
    } else {
        command.env_remove(role_input);
    }
    let bytes = crate::process::capture(
        command
            .current_dir(
                manifest
                    .parent()
                    .ok_or_else(|| Error::Invalid("workspace manifest needs a directory".into()))?,
            )
            .args(["run", "--release", "--quiet", "--locked", "--manifest-path"])
            .arg(manifest)
            .args([
                "--package",
                &recipe.package,
                "--example",
                &format!("{}-wire", recipe.package),
                "--features",
                &features.join(","),
            ]),
        8 * 1024 * 1024,
        1024 * 1024,
        Duration::from_secs(600),
    )
    .map_err(crate::artifact::cargo_failure)?;
    let methods: BTreeMap<String, Value> = serde_json::from_slice(&bytes).map_err(supplier)?;
    let actual: Vec<_> = exports
        .iter()
        .map(|e| (e.name.as_str(), Some(e.mode)))
        .collect();
    let inspected: Vec<_> = methods
        .iter()
        .map(|(name, m)| (name.as_str(), m["mode"].as_str()))
        .collect();
    if methods.is_empty() || inspected != actual {
        return Err(Error::Invalid(format!(
            "{name} Wire declarations {inspected:?} disagree with Wasm exports {actual:?}"
        )));
    }
    Ok(methods)
}

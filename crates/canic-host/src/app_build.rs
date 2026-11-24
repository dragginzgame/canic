//! App recipes reuse the artifact builder; Cargo owns dependency resolution and locking.
use crate::{Artifact, Error, Result, supplier};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

/// A replaceable build output, not retained deployment evidence.
#[derive(Debug, Serialize)]
pub struct BuiltCanister {
    /// Version of the Cargo package that produced this Wasm.
    pub version: String,
    /// Shared app definition name.
    pub canister: String,
    /// Role declared by the application for this slot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Output path relative to the invoking directory.
    pub wasm: PathBuf,
    /// Exact built bytes.
    pub artifact: Artifact,
    /// App-declared parent slot, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Number of callable query, composite query and update exports.
    pub endpoint_count: usize,
    /// Mounts declared by the built Wasm.
    pub modules: Vec<&'static str>,
    /// Elapsed time for the canister recipe, including any native codec.
    pub build_ms: u128,
    /// Optional app-owned native codec executable, not a deployment artifact.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<PathBuf>,
}

/// Build all declared app recipes or one selected canister, initializing absent workspace locks once.
/// All recipes are validated before building; selection does not hide invalid app declarations.
/// Existing locks are preserved and compilation always uses `--locked`.
/// # Errors
/// Refuses invalid recipes, duplicate outputs, outputs replacing configuration/Cargo inputs or failed builds.
pub fn build_app(
    path: &Path,
    app: Option<&str>,
    canister: Option<&str>,
) -> Result<Vec<BuiltCanister>> {
    let config = crate::config::load_build_config(path, app)?;
    let (app_name, app) = config.select_app(app)?;
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
    let mut inputs =
        std::collections::BTreeSet::from([
            crate::output_path::resolve(path, &working_directory).map_err(supplier)?
        ]);
    inputs.insert(
        crate::output_path::resolve(
            &base
                .join("wire-baseline")
                .join(app_name)
                .join("contracts.json"),
            &working_directory,
        )
        .map_err(supplier)?,
    );
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
        inputs.insert(manifest.clone());
        inputs.insert(
            crate::output_path::resolve(&manifest.with_file_name("Cargo.lock"), &working_directory)
                .map_err(supplier)?,
        );
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
        if recipe.client.is_some()
            && !outputs.insert(
                crate::output_path::resolve(
                    &crate::application::codec::output(&output),
                    &working_directory,
                )
                .map_err(supplier)?,
            )
        {
            return Err(Error::Invalid("app build outputs must be distinct".into()));
        }
        recipes.push((name, canister, recipe, manifest, output));
    }
    // Check the complete app, including recipes not selected for this build.
    // Missing lockfiles are inputs too: Cargo may create them below.
    if outputs.iter().any(|output| inputs.contains(output)) {
        return Err(Error::Invalid(
            "app build output would replace its config, workspace manifest, Cargo.lock or reviewed Wire baseline".into(),
        ));
    }
    let mut built = Vec::new();
    for (name, definition, recipe, manifest, output) in recipes {
        if canister.is_some_and(|selected| selected != name) {
            continue;
        }
        let started = Instant::now();
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
        let children = app
            .canisters
            .iter()
            .filter(|(_, child)| child.parent.as_ref() == Some(name))
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let mut features = recipe.features.clone();
        let source =
            crate::artifact::build_manifest(&manifest, &recipe.package, Some(&mut features))?;
        let crate::artifact::BuildReport {
            version,
            wasm,
            artifact,
            endpoint_count,
            modules,
        } = crate::artifact::build_for_role(
            &manifest,
            &source,
            &recipe.package,
            &features,
            Some(&output),
            definition.role.as_deref(),
            Some(&children),
        )?;
        let client = recipe
            .client
            .as_ref()
            .map(|client| {
                build_client(
                    workspace,
                    &source,
                    &recipe.package,
                    client,
                    &features,
                    &children,
                    &crate::application::codec::output(&wasm),
                )
            })
            .transpose()?;
        built.push(BuiltCanister {
            version,
            canister: name.clone(),
            parent: definition.parent.clone(),
            role: definition.role.clone(),
            endpoint_count,
            modules,
            build_ms: started.elapsed().as_millis(),
            artifact,
            wasm,
            client,
        });
    }
    Ok(built)
}

fn build_client(
    workspace: &Path,
    manifest: &Path,
    package: &str,
    client: &str,
    features: &[String],
    children: &str,
    destination: &Path,
) -> Result<PathBuf> {
    let mut command = Command::new("cargo");
    command
        .env(format!("CANIC_APP_CHILDREN_{package}"), children)
        .current_dir(workspace)
        .args([
            "build",
            "--locked",
            "--release",
            "--message-format=json-render-diagnostics",
            "--manifest-path",
        ])
        .arg(manifest)
        .args(["--package", package, "--bin", client]);
    if package == "canic-root" && client == "canic-root-client" {
        command.args(["--features", "operator-client"]);
    }
    if package == "canic-root" && !manifest.starts_with(workspace) {
        command
            .arg("--target-dir")
            .arg(workspace.join("target/root-client"));
    }
    if !features.is_empty() {
        command.arg("--features").arg(features.join(","));
    }
    let output = crate::process::capture(
        &mut command,
        8 * 1024 * 1024,
        1024 * 1024,
        Duration::from_secs(600),
    )
    .map_err(crate::artifact::cargo_failure)?;
    let mut executable = None;
    for line in output
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
    {
        let value: serde_json::Value = serde_json::from_slice(line).map_err(supplier)?;
        if value["reason"] == "compiler-artifact"
            && value["target"]["name"] == client
            && value["target"]["kind"] == serde_json::json!(["bin"])
            && let Some(path) = value["executable"].as_str()
            && executable.replace(PathBuf::from(path)).is_some()
        {
            return Err(Error::Invalid("ambiguous app command executable".into()));
        }
    }
    let executable = executable
        .ok_or_else(|| Error::Invalid("Cargo produced no app command executable".into()))?;
    let bytes = crate::store::read(&executable, 64 * 1024 * 1024)?;
    crate::output_publication::publish(
        destination,
        &bytes,
        ic_host_fs::Destination::Replace,
        0o700,
        0o700,
    )?;
    Ok(destination.into())
}

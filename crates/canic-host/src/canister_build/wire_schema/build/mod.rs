//! Build the selected role's native producer using its governed Cargo selection.

#[cfg(test)]
mod tests;

use crate::{
    canister_build::{
        CanisterArtifactBuildOptions, WorkspaceBuildContext,
        artifact::{
            canister_runtime_command, require_default_build_options,
            resolve_canister_artifact_build_spec,
        },
        cache::{
            CargoBuildProgress, canister_build_target_root, lock_canister_build_target,
            output_canister_cargo_command,
        },
        infrastructure::{self, InfrastructureRole},
        model::{CanisterArtifactSource, NativeSchemaBuildRecipe},
        reuse::require_native_tool,
        wire_schema::{RetainedWireSchema, acquire_schema, schema_cache_spec},
    },
    release_set::AppConfigSnapshot,
};
use ic_build::{
    artifacts::{
        ArtifactCacheOutcome, ArtifactCachePreparation, ArtifactCacheSpec, WasmBuildSpec,
        prepare_artifact_cache, resolve_cargo_build_inputs,
    },
    cargo::{
        execution::{CargoInvocation, CargoRecipe},
        observation::schema::Message,
    },
};
use ic_host_process::tool::{OutputLimit, OutputLimits, capture_command};
use std::{
    collections::BTreeSet,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

/// Native recipe admission failures, distinct from Cargo/process/input failures.
#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub enum WireSchemaBuildError {
    #[error("final schema for {role} differs from the contract bound into the compiled profile")]
    ChangedContract { role: String },
    #[error("native producer recipe disagrees with its package, manifest or workspace")]
    RecipeSelection,
    #[error("selected compiler did not identify exactly one native host target")]
    HostTarget,
    #[error("Cargo did not report exactly one matching native schema executable")]
    ProducerSelection,
    #[error(
        "native schema inputs changed during {phase}: source={source_changed}, recipe={recipe_changed}, execution={execution_changed}"
    )]
    ChangedInputs {
        phase: &'static str,
        source_changed: bool,
        recipe_changed: bool,
        execution_changed: bool,
    },
}

/// Compile the configured role's `wire_schema` example and retain its admitted schema.
///
/// Preserve runtime profile, default-feature policy and selected features, adding
/// only `wire-schema` to this explicit native build. Lock the shared build boundary
/// through acquisition; do not call while already holding that build lock.
/// The package must declare the native example. This does not seal a release.
pub fn build_workspace_wire_schema(
    context: &WorkspaceBuildContext,
    options: &CanisterArtifactBuildOptions,
    final_wasm: &Path,
    destination: &Path,
) -> Result<RetainedWireSchema, Box<dyn std::error::Error>> {
    let lock = lock_canister_build_target(&context.workspace_root)?;
    build_workspace_wire_schema_locked(context, options, final_wasm, destination, &lock)
}

/// Build an explicitly selected producer for a nonstandard artifact owner.
///
/// The caller supplies its actual runtime package and invocation. The shared pipeline
/// preserves runtime features/environment, selects the compiler host, guards Cargo
/// inputs and admits the resulting native schema against final Wasm before delivery.
/// This does not admit a package for deployment or infer a schema from a role name.
pub fn build_native_wire_schema(
    context: &WorkspaceBuildContext,
    recipe: NativeSchemaBuildRecipe,
    final_wasm: &Path,
    destination: &Path,
) -> Result<RetainedWireSchema, Box<dyn std::error::Error>> {
    validate_recipe(&recipe)?;
    let lock = lock_canister_build_target(&context.workspace_root)?;
    build_native_wire_schema_locked(context, recipe, final_wasm, destination, &lock)
}

fn validate_recipe(recipe: &NativeSchemaBuildRecipe) -> Result<(), WireSchemaBuildError> {
    let invocation = &recipe.invocation;
    if recipe.package.is_empty()
        || invocation.working_directory != recipe.workspace
        || invocation.manifest_path.as_ref() != Some(&recipe.manifest)
        || invocation.packages.as_slice() != std::slice::from_ref(&recipe.package)
        || !invocation.configurations.is_empty()
        || !invocation.selection.locked
    {
        return Err(WireSchemaBuildError::RecipeSelection);
    }
    Ok(())
}

/// Acquire schema after runtime finalization while the owning build retains its lock.
pub(in crate::canister_build) fn build_workspace_wire_schema_locked(
    context: &WorkspaceBuildContext,
    options: &CanisterArtifactBuildOptions,
    final_wasm: &Path,
    destination: &Path,
    build_lock: &std::fs::File,
) -> Result<RetainedWireSchema, Box<dyn std::error::Error>> {
    let recipe = resolve_recipe(context, options)?;
    build_native_wire_schema_locked(context, recipe, final_wasm, destination, build_lock)
}

fn build_native_wire_schema_locked(
    context: &WorkspaceBuildContext,
    recipe: NativeSchemaBuildRecipe,
    final_wasm: &Path,
    destination: &Path,
    _build_lock: &std::fs::File,
) -> Result<RetainedWireSchema, Box<dyn std::error::Error>> {
    let CompiledNativeProducer {
        producer,
        build,
        inputs,
    } = compile_native_producer(context, recipe)?;
    let target = canister_build_target_root(&context.workspace_root);
    let final_wasm = final_wasm.canonicalize()?;
    let cache = target.join("canic-wire-schema-cache");
    let staging = target
        .join("canic-wire-schema-output")
        .join(&context.role)
        .with_extension("wire.json");
    let spec = schema_cache_spec(&cache, &producer, &final_wasm, &staging)?
        .with_cargo_build_inputs("native-producer", &build, &inputs);
    let retained = acquire_schema(&spec, &producer, &final_wasm)?;
    materialize_schema(
        &target.join("canic-wire-delivery-cache"),
        retained,
        destination,
    )
}

struct CompiledNativeProducer {
    producer: PathBuf,
    build: WasmBuildSpec,
    inputs: ic_build::artifacts::ResolvedCargoBuildInputs,
}

fn compile_native_producer(
    context: &WorkspaceBuildContext,
    recipe: NativeSchemaBuildRecipe,
) -> Result<CompiledNativeProducer, Box<dyn std::error::Error>> {
    let mut invocation = recipe.invocation;
    let host = compiler_host(&invocation)?;
    select_native_recipe(&mut invocation, host);
    let target = canister_build_target_root(&context.workspace_root);
    invocation
        .environment
        .insert("CARGO_TARGET_DIR".into(), target.clone().into_os_string());
    // The build spec owns this same target directory; it is not an ambient input override.
    let mut inputs_environment = invocation.environment.clone();
    inputs_environment.remove(OsStr::new("CARGO_TARGET_DIR"));
    let build = WasmBuildSpec::new(
        &recipe.workspace,
        &target,
        &[&recipe.package],
        inputs_environment,
        crate::canister_build::CARGO_CONFIGURATION_LIMITS,
    )
    .with_example("wire_schema")
    .with_target(&invocation.target)
    .with_cargo_program(invocation.cargo_program.clone())
    .with_cargo_selection(invocation.selection.clone())
    .with_additional_inputs([&context.config_path]);
    let inputs = resolve_cargo_build_inputs(&build)?;
    let output =
        output_canister_cargo_command(&invocation, CargoBuildProgress::single(&context.role))?;
    if !output.execution.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.execution.stderr));
    }
    output.execution.require_success()?;
    require_current_inputs(&inputs, &build, "compilation")?;
    let producer = selected_producer(&output.messages, &recipe.manifest)?;
    require_native_tool(&producer)?;
    Ok(CompiledNativeProducer {
        producer,
        build,
        inputs,
    })
}

/// Produce native contract bytes before compiling their runtime profile.
/// This does not admit final Wasm or deliver release artifacts.
pub fn build_native_wire_contract(
    context: &WorkspaceBuildContext,
    recipe: NativeSchemaBuildRecipe,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    validate_recipe(&recipe)?;
    let _lock = lock_canister_build_target(&context.workspace_root)?;
    native_contract(context, recipe)
}

pub(in crate::canister_build) fn workspace_wire_contract(
    context: &WorkspaceBuildContext,
    options: &CanisterArtifactBuildOptions,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // The artifact builder owns the shared target lock for this complete operation.
    native_contract(context, resolve_recipe(context, options)?)
}

fn native_contract(
    context: &WorkspaceBuildContext,
    recipe: NativeSchemaBuildRecipe,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let CompiledNativeProducer {
        producer,
        build,
        inputs,
    } = compile_native_producer(context, recipe)?;
    let mut command = crate::build_environment::command(&producer);
    command.arg("--native");
    let output = capture_command(
        &mut command,
        OutputLimits {
            stdout: OutputLimit::Terminate(crate::MAX_DOCUMENT_READ_BYTES),
            stderr: OutputLimit::Terminate(64 * 1024),
            timeout: Some(Duration::from_secs(120)),
        },
    )?
    .require_success()?
    .require_complete()?;
    require_current_inputs(&inputs, &build, "native projection")?;
    Ok(output.stdout)
}

fn require_current_inputs(
    inputs: &ic_build::artifacts::ResolvedCargoBuildInputs,
    build: &WasmBuildSpec,
    phase: &'static str,
) -> Result<(), Box<dyn std::error::Error>> {
    let current = resolve_cargo_build_inputs(build)?;
    if current.fingerprint() != inputs.fingerprint() {
        return Err(WireSchemaBuildError::ChangedInputs {
            phase,
            source_changed: current.input_digest() != inputs.input_digest(),
            recipe_changed: current.recipe_identity() != inputs.recipe_identity(),
            execution_changed: current.facts() != inputs.facts(),
        }
        .into());
    }
    Ok(())
}

// Publication consumes immutable admitted outputs after the producer input guard
// has completed. Release directories may themselves lie inside package inputs.
fn materialize_schema(
    cache: &Path,
    retained: RetainedWireSchema,
    destination: &Path,
) -> Result<RetainedWireSchema, Box<dyn std::error::Error>> {
    let artifacts = retained.acquisition.record().artifacts();
    let mut spec = ArtifactCacheSpec::new(cache, "canic-wire-delivery", "canic.wire-delivery.v1")
        .with_identity_bytes("recipe", include_bytes!("mod.rs"));
    for artifact in artifacts {
        let output = match artifact.name() {
            "schema" => destination.to_owned(),
            "bindings" => destination.with_extension("ts"),
            _ => return Err(std::io::Error::other("unexpected schema artifact").into()),
        };
        spec = spec
            .with_input(artifact.name(), artifact.path())
            .with_output(artifact.name(), &output);
    }
    let acquisition = match prepare_artifact_cache(&spec)? {
        ArtifactCachePreparation::Reused(record) => ArtifactCacheOutcome::Reused(record),
        ArtifactCachePreparation::Build(transaction) => {
            for artifact in artifacts {
                transaction.import_output(artifact.name(), artifact.path())?;
            }
            transaction.commit()?
        }
    };
    Ok(RetainedWireSchema {
        acquisition,
        bindings: retained.bindings,
    })
}

fn select_native_recipe(invocation: &mut CargoInvocation, host: String) {
    invocation.target = host;
    invocation.recipe = CargoRecipe::Example("wire_schema".into());
    invocation
        .selection
        .features
        .features
        .insert("wire-schema".into());
}

fn compiler_host(invocation: &CargoInvocation) -> Result<String, Box<dyn std::error::Error>> {
    let compiler = invocation
        .environment
        .get(OsStr::new("RUSTC"))
        .ok_or(WireSchemaBuildError::HostTarget)?;
    let mut command = Command::new(compiler);
    command
        .current_dir(&invocation.working_directory)
        .env_clear()
        .envs(&invocation.environment)
        .arg("-vV");
    let output = capture_command(
        &mut command,
        OutputLimits {
            stdout: OutputLimit::Terminate(64 * 1024),
            stderr: OutputLimit::Terminate(64 * 1024),
            timeout: Some(Duration::from_secs(30)),
        },
    )?
    .require_success()?
    .require_complete()?;
    parse_compiler_host(std::str::from_utf8(&output.stdout)?).map_err(Into::into)
}

fn parse_compiler_host(identity: &str) -> Result<String, WireSchemaBuildError> {
    let mut hosts = identity
        .lines()
        .filter_map(|line| line.strip_prefix("host: "));
    let host = hosts
        .next()
        .filter(|value| !value.is_empty() && !value.chars().any(char::is_whitespace));
    match (host, hosts.next()) {
        (Some(host), None) if !host.starts_with("wasm") => Ok(host.to_owned()),
        _ => Err(WireSchemaBuildError::HostTarget),
    }
}

fn selected_producer(
    messages: &[Message],
    manifest: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest = manifest.canonicalize()?;
    let mut paths = BTreeSet::new();
    for message in messages {
        let Message::CompilerArtifact(artifact) = message else {
            continue;
        };
        if artifact.target.is_example()
            && artifact.target.name == "wire_schema"
            && artifact.manifest_path.canonicalize()? == manifest
            && let Some(path) = &artifact.executable
        {
            paths.insert(path.canonicalize()?);
        }
    }
    if paths.len() != 1 {
        return Err(WireSchemaBuildError::ProducerSelection.into());
    }
    Ok(paths.pop_first().expect("one selected executable"))
}

fn resolve_recipe(
    context: &WorkspaceBuildContext,
    options: &CanisterArtifactBuildOptions,
) -> Result<NativeSchemaBuildRecipe, Box<dyn std::error::Error>> {
    let role = match CanisterArtifactSource::for_role(&context.role) {
        CanisterArtifactSource::FleetCoordinator => Some(InfrastructureRole::Coordinator),
        CanisterArtifactSource::Root => Some(InfrastructureRole::Root),
        CanisterArtifactSource::WasmStore => Some(InfrastructureRole::Store),
        CanisterArtifactSource::DeclaredRole => None,
    };
    if let Some(role) = role {
        require_default_build_options(options, role.name())?;
        return infrastructure::native_schema_recipe(role, context);
    }
    let config = AppConfigSnapshot::load(&context.config_path)?;
    let package = resolve_canister_artifact_build_spec(context, config.model(), options)?;
    let mut invocation = canister_runtime_command(
        context,
        &package.package_manifest_path,
        context.profile,
        options,
    );
    invocation.packages.push(package.package_name.clone());
    Ok(NativeSchemaBuildRecipe {
        manifest: package.package_manifest_path,
        workspace: package.cargo_workspace_root,
        package: package.package_name,
        invocation,
    })
}

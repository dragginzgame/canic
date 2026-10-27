//! Acquire a native schema through the shared retained artifact transaction.
//!
//! The selected producer owns mounted Rust layouts. Host admits its output
//! against final Wasm. The configured build entrypoint compiles the producer;
//! callers still own normal artifact-phase invocation and release binding.

mod build;

#[cfg(test)]
mod tests;

use crate::{
    artifact_io::{WireBindings, generate_wire_bindings},
    canister_build::reuse::require_native_tool,
};
use ic_build::artifacts::{
    ArtifactCacheOutcome, ArtifactCachePreparation, ArtifactCacheSpec, prepare_artifact_cache,
};
use ic_host_artifacts::wasm::{InspectionLimits, inspect};
use ic_host_fs::{
    durable::{PublicationMode, WriteOptions, write_bytes},
    read::read_file_no_follow,
};
use ic_host_process::tool::{OutputLimit, OutputLimits, capture_command};
use std::{path::Path, time::Duration};

pub use build::{
    WireSchemaBuildError, build_native_wire_contract, build_native_wire_schema,
    build_workspace_wire_schema,
};
pub(super) use build::{build_workspace_wire_schema_locked, workspace_wire_contract};

/// Admitted schema identity and bindings with ownership of their retained bytes.
#[derive(Debug)]
pub struct RetainedWireSchema {
    pub acquisition: ArtifactCacheOutcome,
    pub bindings: WireBindings,
}

/// Retain an admitted schema and TypeScript keyed by producer, final Wasm and recipe bytes.
///
/// The executable accepts exactly one final-Wasm path and writes only schema JSON
/// to stdout. Bindings use the destination path with a `.ts` extension. Failed
/// generation/admission never publishes a partial retained artifact set. The
/// returned record retains the immutable cache entry while callers consume it.
/// This is acquisition, not release-manifest publication or producer qualification.
pub fn produce_wire_schema(
    cache_root: &Path,
    producer: &Path,
    final_wasm: &Path,
    destination: &Path,
) -> Result<RetainedWireSchema, Box<dyn std::error::Error>> {
    let producer = producer.canonicalize()?;
    let final_wasm = final_wasm.canonicalize()?;
    require_native_tool(&producer)?;
    let spec = schema_cache_spec(cache_root, &producer, &final_wasm, destination)?;
    acquire_schema(&spec, &producer, &final_wasm)
}

fn schema_cache_spec(
    cache_root: &Path,
    producer: &Path,
    final_wasm: &Path,
    destination: &Path,
) -> Result<ArtifactCacheSpec, Box<dyn std::error::Error>> {
    Ok(
        ArtifactCacheSpec::new(cache_root, "canic-wire-schema", "canic.wire-schema.v1")
            .with_input("final-wasm", final_wasm)
            .with_tool("producer", producer)
            .with_tool("host", &std::env::current_exe()?)
            .with_environment(crate::build_environment::inputs())
            .with_identity_bytes("version", env!("CARGO_PKG_VERSION").as_bytes())
            .with_identity_bytes("recipe", include_bytes!("mod.rs"))
            .with_identity_bytes("admission", include_bytes!("../../artifact_io/wire/mod.rs"))
            .with_identity_bytes(
                "environment",
                include_bytes!("../../build_environment/mod.rs"),
            )
            .with_output("schema", destination)
            .with_output("bindings", &destination.with_extension("ts")),
    )
}

fn acquire_schema(
    spec: &ArtifactCacheSpec,
    producer: &Path,
    final_wasm: &Path,
) -> Result<RetainedWireSchema, Box<dyn std::error::Error>> {
    let preparation = prepare_artifact_cache(spec)?;
    let wasm = read_file_no_follow(final_wasm, crate::MAX_ARTIFACT_READ_BYTES)?;
    let facts = inspect(
        &wasm,
        InspectionLimits {
            module_bytes: crate::MAX_ARTIFACT_READ_BYTES,
            sections: 1024,
            exports: 4096,
            custom_sections: 1024,
        },
    )?;
    let transaction = match preparation {
        ArtifactCachePreparation::Reused(record) => {
            let artifact = record
                .artifacts()
                .iter()
                .find(|artifact| artifact.name() == "schema")
                .ok_or_else(|| std::io::Error::other("retained schema output missing"))?;
            let bytes = read_file_no_follow(artifact.path(), crate::MAX_DOCUMENT_READ_BYTES)?;
            let bindings = generate_wire_bindings(&facts, &bytes, "./wire-adapter.js")?;
            let retained = record
                .artifacts()
                .iter()
                .find(|artifact| artifact.name() == "bindings")
                .ok_or_else(|| std::io::Error::other("retained bindings output missing"))?;
            let bytes = read_file_no_follow(retained.path(), crate::MAX_ARTIFACT_READ_BYTES)?;
            if bytes != bindings.typescript.as_bytes() {
                return Err(
                    std::io::Error::other("retained bindings differ from admitted schema").into(),
                );
            }
            return Ok(RetainedWireSchema {
                acquisition: ArtifactCacheOutcome::Reused(record),
                bindings,
            });
        }
        ArtifactCachePreparation::Build(transaction) => transaction,
    };
    let mut command = crate::build_environment::command(producer);
    command.arg(final_wasm);
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
    let bindings = generate_wire_bindings(&facts, &output.stdout, "./wire-adapter.js")?;
    write_bytes(
        &transaction.output_path("schema")?,
        WriteOptions {
            mode: PublicationMode::Replace,
            permissions: 0o666,
        },
        &output.stdout,
    )?;
    write_bytes(
        &transaction.output_path("bindings")?,
        WriteOptions {
            mode: PublicationMode::Replace,
            permissions: 0o666,
        },
        bindings.typescript.as_bytes(),
    )?;
    Ok(RetainedWireSchema {
        acquisition: transaction.commit()?,
        bindings,
    })
}

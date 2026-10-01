//! Qualify the public embedded allocation peer against its selected Cargo inputs.
//!
//! Evidence lives outside the producer dependency graph. Verification never edits source.

#[cfg(test)]
mod tests;

use canic_core::cdk::utils::hash::sha256_hex;
use ic_testkit::artifacts::{
    WasmBuildSpec, build_wasm_canisters_cached, resolve_cargo_build_inputs,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fs, io, path::Path, process::Command};

const ARTIFACT: &str =
    "crates/canic/src/testing/managed_component_group/fixture/sharding_root_stub.wasm";
const EVIDENCE: &str = "scripts/dev/managed-root-fixture.json";
const PACKAGE: &str = "sharding_root_stub";
const TARGET: &str = "wasm32-unknown-unknown";
const PROFILE: &str = "fast";

/// Build provenance for the checked-in peer; selected inputs own freshness, not raw lock churn.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureEvidence {
    schema_version: u16,
    package: String,
    target: String,
    profile: String,
    source_input_digest: String,
    build_fingerprint: String,
    lock_sha256: String,
    artifact_sha256: String,
    cargo: String,
    rustc: String,
}

/// Check recorded identity or compare a freshly qualified cached build after inputs change.
/// Normal qualification never updates the checked-in artifact or evidence.
pub fn verify(workspace: &Path) -> Result<(), Box<dyn Error>> {
    let workspace = workspace.canonicalize()?;
    let spec = build_spec(&workspace);
    let inputs = resolve_cargo_build_inputs(&spec)?;
    let evidence: FixtureEvidence = serde_json::from_slice(&fs::read(workspace.join(EVIDENCE))?)?;
    let embedded = fs::read(workspace.join(ARTIFACT))?;
    if recorded_match(&evidence, &inputs.input_digest().to_hex(), &embedded)? {
        return Ok(());
    }
    // A metadata-only change may select the same executable. Prove that fact
    // through the content-addressed build rather than demanding a source rewrite.
    let (built, _) = build(&workspace, &spec)?;
    if built != embedded {
        return Err(io::Error::new(io::ErrorKind::InvalidData,
            "embedded allocation peer differs from current source; run cargo run --locked --offline -p canic-testing-internal --example refresh_embedded_root").into());
    }
    Ok(())
}

/// Explicit maintainer refresh of the peer and its structured source/tool/lock evidence.
/// Call only after other repository validation has finished.
pub fn refresh(workspace: &Path) -> Result<(), Box<dyn Error>> {
    let workspace = workspace.canonicalize()?;
    let spec = build_spec(&workspace);
    let (bytes, _) = build(&workspace, &spec)?;
    canic_host::durable_io::write_bytes(&workspace.join(ARTIFACT), &bytes)?;
    // The facade contains the host-only embedded bytes. Qualify again after
    // publication so evidence binds the final tree, with no self-hash in it.
    let (confirmed, evidence) = build(&workspace, &spec)?;
    if confirmed != bytes {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "embedded peer changed its own Wasm build",
        )
        .into());
    }
    canic_host::durable_io::write_bytes(
        &workspace.join(EVIDENCE),
        &serde_json::to_vec_pretty(&evidence)?,
    )?;
    Ok(())
}

fn build_spec(workspace: &Path) -> WasmBuildSpec {
    WasmBuildSpec::new(
        workspace,
        &workspace.join("target/embedded-root/cache"),
        &[PACKAGE],
        PROFILE,
    )
    .with_shared_incremental_target(workspace.join("target/embedded-root/cargo"))
    .with_cargo_profile_args(["--profile", PROFILE, "--locked", "--offline"])
    .with_extra_env([("CARGO_INCREMENTAL", "0")])
}

fn build(
    workspace: &Path,
    spec: &WasmBuildSpec,
) -> Result<(Vec<u8>, FixtureEvidence), Box<dyn Error>> {
    let outcome = build_wasm_canisters_cached(spec)?;
    let record = outcome.record();
    let [artifact] = record.artifacts() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected one allocation peer artifact",
        )
        .into());
    };
    let bytes = fs::read(artifact)?;
    let evidence = FixtureEvidence {
        schema_version: 1,
        package: PACKAGE.into(),
        target: TARGET.into(),
        profile: PROFILE.into(),
        source_input_digest: record.input_digest().to_hex(),
        build_fingerprint: record.fingerprint().to_hex(),
        lock_sha256: sha256_hex(&fs::read(workspace.join("Cargo.lock"))?),
        artifact_sha256: sha256_hex(&bytes),
        cargo: tool_identity("cargo", &["--version", "--verbose"])?,
        rustc: tool_identity("rustc", &["-vV"])?,
    };
    Ok((bytes, evidence))
}

fn tool_identity(tool: &str, arguments: &[&str]) -> io::Result<String> {
    let output = Command::new(tool).args(arguments).output()?;
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    String::from_utf8(output.stdout).map_err(io::Error::other)
}

fn recorded_match(evidence: &FixtureEvidence, inputs: &str, bytes: &[u8]) -> io::Result<bool> {
    let producer_matches = evidence.schema_version == 1
        && evidence.package == PACKAGE
        && evidence.target == TARGET
        && evidence.profile == PROFILE;
    let valid_digests = [
        &evidence.source_input_digest,
        &evidence.build_fingerprint,
        &evidence.lock_sha256,
    ]
    .iter()
    .all(|digest| digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let artifact_matches = evidence.artifact_sha256 == sha256_hex(bytes);
    let tools_recorded = !evidence.cargo.is_empty() && !evidence.rustc.is_empty();
    if !producer_matches || !valid_digests || !artifact_matches || !tools_recorded {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "embedded allocation peer evidence does not bind this artifact",
        ));
    }
    Ok(evidence.source_input_digest == inputs)
}

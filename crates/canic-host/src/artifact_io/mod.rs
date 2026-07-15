use crate::{
    canister_build::{
        ArtifactTransformKind, ArtifactTransformMode, ArtifactTransformOutcome,
        ArtifactTransformOutput,
    },
    durable_io::write_bytes,
};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    process::Command,
};

use flate2::{Compression, GzBuilder};

// Apply `ic-wasm shrink` when available; absence of the optional tool is not
// fatal, but execution failures are surfaced because they usually mean bad IO.
pub fn maybe_shrink_wasm_artifact(
    role: &str,
    wasm_path: &Path,
) -> Result<ArtifactTransformOutput, Box<dyn std::error::Error>> {
    maybe_shrink_wasm_artifact_with_command("ic-wasm", role, wasm_path)
}

fn maybe_shrink_wasm_artifact_with_command(
    command_name: &str,
    role: &str,
    wasm_path: &Path,
) -> Result<ArtifactTransformOutput, Box<dyn std::error::Error>> {
    let Some(tool_version) = optional_ic_wasm_version(command_name)? else {
        return Ok(transform_output(
            role,
            ArtifactTransformKind::Shrink,
            None,
            ArtifactTransformOutcome::ToolUnavailable,
        ));
    };
    let shrunk_path = wasm_path.with_extension("wasm.shrunk");
    match Command::new(command_name)
        .arg(wasm_path)
        .arg("-o")
        .arg(&shrunk_path)
        .arg("shrink")
        .output()
    {
        Ok(output) if output.status.success() => {
            fs::rename(shrunk_path, wasm_path)?;
            Ok(transform_output(
                role,
                ArtifactTransformKind::Shrink,
                Some(tool_version),
                ArtifactTransformOutcome::Applied,
            ))
        }
        Ok(output) => {
            let _ = fs::remove_file(shrunk_path);
            Err(format!(
                "ic-wasm shrink failed for {} with status {}: {}",
                wasm_path.display(),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )
            .into())
        }
        Err(err) => {
            let _ = fs::remove_file(shrunk_path);
            Err(format!("failed to run ic-wasm for {}: {err}", wasm_path.display()).into())
        }
    }
}

// Copy one `.wasm` artifact atomically into the local ICP artifact tree.
pub fn write_wasm_artifact(
    source_path: &Path,
    target_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(source_path)?;
    write_bytes(target_path, &bytes)?;
    Ok(())
}

// Write one deterministic `.wasm.gz` artifact with a zeroed gzip timestamp.
pub fn write_gzip_artifact(
    wasm_path: &Path,
    wasm_gz_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut wasm_bytes = Vec::new();
    fs::File::open(wasm_path)?.read_to_end(&mut wasm_bytes)?;

    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(&wasm_bytes)?;
    let gz_bytes = encoder.finish()?;
    write_bytes(wasm_gz_path, &gz_bytes)?;
    Ok(())
}

// Embed the extracted service interface for local artifacts so
// `icp canister metadata <canister> candid:service` introspection works during
// development. Production `ic` builds skip this path.
pub fn embed_candid_metadata(
    role: &str,
    wasm_path: &Path,
    did_path: &Path,
) -> Result<ArtifactTransformOutput, Box<dyn std::error::Error>> {
    embed_candid_metadata_with_command("ic-wasm", role, wasm_path, did_path)
}

fn embed_candid_metadata_with_command(
    command_name: &str,
    role: &str,
    wasm_path: &Path,
    did_path: &Path,
) -> Result<ArtifactTransformOutput, Box<dyn std::error::Error>> {
    let Some(tool_version) = optional_ic_wasm_version(command_name)? else {
        return Ok(transform_output(
            role,
            ArtifactTransformKind::CandidMetadata,
            None,
            ArtifactTransformOutcome::ToolUnavailable,
        ));
    };
    let output = Command::new(command_name)
        .arg(wasm_path)
        .args(["-o"])
        .arg(wasm_path)
        .args(["metadata", "candid:service", "-f"])
        .arg(did_path)
        .args(["-v", "public"])
        .output();

    let output = output.map_err(|err| {
        format!(
            "failed to run ic-wasm metadata for {}: {err}",
            wasm_path.display()
        )
    })?;

    if !output.status.success() {
        return Err(format!(
            "ic-wasm metadata failed for {}: {}",
            wasm_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(transform_output(
        role,
        ArtifactTransformKind::CandidMetadata,
        Some(tool_version),
        ArtifactTransformOutcome::Applied,
    ))
}

fn optional_ic_wasm_version(
    command_name: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let output = match Command::new(command_name).arg("--version").output() {
        Ok(output) => output,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("failed to inspect ic-wasm version: {err}").into()),
    };
    if !output.status.success() {
        return Err(format!(
            "ic-wasm --version failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let version = if stdout.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };
    if version.is_empty() {
        return Err("ic-wasm --version returned no version identity".into());
    }
    Ok(Some(version.to_string()))
}

fn transform_output(
    role: &str,
    transform: ArtifactTransformKind,
    tool_version: Option<String>,
    outcome: ArtifactTransformOutcome,
) -> ArtifactTransformOutput {
    ArtifactTransformOutput {
        role: role.to_string(),
        transform,
        mode: ArtifactTransformMode::Optional,
        tool: "ic-wasm".to_string(),
        tool_version,
        outcome,
    }
}

#[cfg(test)]
mod tests;

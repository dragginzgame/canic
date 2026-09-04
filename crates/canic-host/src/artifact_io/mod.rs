mod wasm;

use crate::{
    binaryen::BinaryenExecutable,
    build_toolchain::BuildToolchain,
    candid_endpoints::parse_candid_service_endpoints,
    canister_build::{
        ArtifactTransformKind, ArtifactTransformOutcome, ArtifactTransformOutput,
        CanisterBuildProfile, WasmTransformMetrics,
    },
    durable_io::write_bytes,
    ic_wasm::IcWasmExecutable,
    output_with_executable_busy_retry,
};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use canic_core::ids::BuildNetwork;
use flate2::{Compression, GzBuilder};

pub use wasm::enforce_wasm_code_section_limit;

const ARTIFACT_STAGE_ATTEMPTS: usize = 64;
static ARTIFACT_STAGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const IC_WASM_FEATURE_FLAGS: &[&str] = &[
    "--enable-bulk-memory",
    "--enable-sign-ext",
    "--enable-nontrapping-float-to-int",
];
const BINARYEN_REPORTED_IC_WASM_FEATURE_FLAGS: &[&str] = &[
    "--enable-bulk-memory",
    "--enable-bulk-memory-opt",
    "--enable-mutable-globals",
    "--enable-nontrapping-float-to-int",
    "--enable-sign-ext",
];

const CANDID_POINTER_EXPORT: &str = "get_candid_pointer";
const IC_CDK_INTERNAL_METHOD_PREFIX: &str = "<ic-cdk internal> ";
const CANISTER_METHOD_EXPORT_PREFIXES: [&str; 3] = [
    "canister_composite_query ",
    "canister_query ",
    "canister_update ",
];

/// Inputs and final paths for one qualification-before-publication Wasm artifact set.
pub struct WasmArtifactFinalization<'a> {
    pub profile: CanisterBuildProfile,
    pub build_network: BuildNetwork,
    pub embed_candid: bool,
    pub validate_sidecar_only: bool,
    pub source_wasm_path: &'a Path,
    pub candid: &'a [u8],
    pub wasm_path: &'a Path,
    pub did_path: &'a Path,
    pub wasm_gz_path: &'a Path,
}

/// Apply the canonical transform/compression pipeline before publishing any final artifact.
pub fn finalize_wasm_artifact(
    finalization: &WasmArtifactFinalization<'_>,
    toolchain: &BuildToolchain,
) -> Result<Vec<ArtifactTransformOutput>, Box<dyn std::error::Error>> {
    toolchain.require_profile(finalization.profile)?;
    stage_and_publish_artifact_set(finalization, |staged| {
        let mut transforms = vec![shrink_wasm_artifact(
            toolchain.ic_wasm(),
            &staged.wasm_path,
        )?];
        if finalization.embed_candid {
            let metadata =
                embed_candid_metadata(toolchain.ic_wasm(), &staged.wasm_path, &staged.did_path)?;
            if metadata.outcome == ArtifactTransformOutcome::Applied {
                validate_embedded_public_candid(&staged.wasm_path, &staged.did_path)?;
            }
            transforms.push(metadata);
        } else {
            transforms.push(ArtifactTransformOutput::not_requested(
                ArtifactTransformKind::CandidMetadata,
            ));
        }
        transforms.push(optimize_release_wasm_artifact(
            finalization.profile,
            toolchain.binaryen(),
            &staged.wasm_path,
        )?);
        enforce_wasm_code_section_limit(finalization.build_network, &staged.wasm_path)?;
        write_gzip_artifact(&staged.wasm_path, &staged.wasm_gz_path)?;
        if finalization.validate_sidecar_only {
            validate_sidecar_only_candid_artifact(&staged.wasm_path, &staged.did_path)?;
        }
        Ok(transforms)
    })
}

fn stage_and_publish_artifact_set<T>(
    finalization: &WasmArtifactFinalization<'_>,
    qualify: impl FnOnce(&StagedArtifactSet) -> Result<T, Box<dyn std::error::Error>>,
) -> Result<T, Box<dyn std::error::Error>> {
    let staged = StagedArtifactSet::create(finalization.wasm_path)?;
    write_wasm_artifact(finalization.source_wasm_path, &staged.wasm_path)?;
    write_bytes(&staged.did_path, finalization.candid)?;

    let result = qualify(&staged)?;
    let wasm = fs::read(&staged.wasm_path)?;
    let candid = fs::read(&staged.did_path)?;
    let wasm_gz = fs::read(&staged.wasm_gz_path)?;

    // The build lock keeps consumers behind this function's success boundary. Each complete
    // file is durably replaced only after the complete candidate set has qualified.
    write_bytes(finalization.wasm_gz_path, &wasm_gz)?;
    write_bytes(finalization.did_path, &candid)?;
    write_bytes(finalization.wasm_path, &wasm)?;
    Ok(result)
}

struct StagedArtifactSet {
    root: PathBuf,
    wasm_path: PathBuf,
    did_path: PathBuf,
    wasm_gz_path: PathBuf,
}

impl StagedArtifactSet {
    fn create(final_wasm_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let parent = final_wasm_path.parent().ok_or_else(|| {
            format!(
                "final Wasm artifact has no parent directory: {}",
                final_wasm_path.display()
            )
        })?;
        for _ in 0..ARTIFACT_STAGE_ATTEMPTS {
            let sequence = ARTIFACT_STAGE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let root = parent.join(format!(
                ".canic-artifact-stage-{}-{sequence}",
                std::process::id()
            ));
            match fs::create_dir(&root) {
                Ok(()) => {
                    return Ok(Self {
                        wasm_path: root.join("candidate.wasm"),
                        did_path: root.join("candidate.did"),
                        wasm_gz_path: root.join("candidate.wasm.gz"),
                        root,
                    });
                }
                Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(source.into()),
            }
        }
        Err(format!(
            "failed to allocate an artifact staging directory beneath {}",
            parent.display()
        )
        .into())
    }
}

impl Drop for StagedArtifactSet {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Prove that a final runtime matches its sidecar method inventory without carrying Candid.
pub fn validate_sidecar_only_candid_artifact(
    wasm_path: &Path,
    did_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let wasm = fs::read(wasm_path)?;
    let candid = fs::read_to_string(did_path)?;
    let snapshot = wasm::wasm_contract_snapshot(&wasm)?;

    if snapshot.exports.contains_key(CANDID_POINTER_EXPORT) {
        return Err(format!(
            "sidecar-only runtime {} still exports {CANDID_POINTER_EXPORT}",
            wasm_path.display()
        )
        .into());
    }
    if !snapshot.public_candid_metadata.is_empty() {
        return Err(format!(
            "sidecar-only runtime {} still embeds public Candid metadata",
            wasm_path.display()
        )
        .into());
    }

    let declared = parse_candid_service_endpoints(&candid)?
        .into_iter()
        .map(|endpoint| endpoint.name)
        .collect::<BTreeSet<_>>();
    let exported = snapshot
        .exports
        .keys()
        .filter_map(|name| {
            CANISTER_METHOD_EXPORT_PREFIXES
                .iter()
                .find_map(|prefix| name.strip_prefix(prefix))
                .filter(|name| !name.starts_with(IC_CDK_INTERNAL_METHOD_PREFIX))
                .map(str::to_string)
        })
        .collect::<BTreeSet<_>>();

    if declared != exported {
        let missing = declared.difference(&exported).cloned().collect::<Vec<_>>();
        let undeclared = exported.difference(&declared).cloned().collect::<Vec<_>>();
        return Err(format!(
            "runtime endpoint exports for {} do not match {}: missing={missing:?}, undeclared={undeclared:?}",
            wasm_path.display(),
            did_path.display()
        )
        .into());
    }
    Ok(())
}

fn validate_embedded_public_candid(
    wasm_path: &Path,
    did_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let wasm = fs::read(wasm_path)?;
    let candid = fs::read(did_path)?;
    let snapshot = wasm::wasm_contract_snapshot(&wasm)?;
    if snapshot.public_candid_metadata.len() != 1
        || snapshot.public_candid_metadata[0].as_slice() != candid.as_slice()
    {
        return Err(format!(
            "embedded public Candid metadata for {} does not exactly match {}",
            wasm_path.display(),
            did_path.display()
        )
        .into());
    }
    Ok(())
}

fn shrink_wasm_artifact(
    tool: &IcWasmExecutable,
    wasm_path: &Path,
) -> Result<ArtifactTransformOutput, Box<dyn std::error::Error>> {
    let shrunk_path = wasm_path.with_extension("wasm.shrunk");
    let mut command = Command::new(tool.path());
    command
        .arg(wasm_path)
        .arg("-o")
        .arg(&shrunk_path)
        .arg("shrink");
    match output_with_executable_busy_retry(&mut command) {
        Ok(output) if output.status.success() => {
            fs::rename(shrunk_path, wasm_path)?;
            Ok(transform_output(
                ArtifactTransformKind::Shrink,
                Some(tool.version_identity().to_string()),
                ArtifactTransformOutcome::Applied,
            ))
        }
        Ok(output) => {
            let _ = fs::remove_file(shrunk_path);
            Err(format!(
                "ic-wasm shrink failed using {} ({}) for {} with status {}: {}",
                tool.path().display(),
                tool.version_identity(),
                wasm_path.display(),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )
            .into())
        }
        Err(err) => {
            let _ = fs::remove_file(shrunk_path);
            Err(format!(
                "failed to run ic-wasm at {} for {}: {err}",
                tool.path().display(),
                wasm_path.display()
            )
            .into())
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

    let gz_bytes = deterministic_gzip_bytes(&wasm_bytes)?;
    write_bytes(wasm_gz_path, &gz_bytes)?;
    Ok(())
}

fn deterministic_gzip_bytes(bytes: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = GzBuilder::new()
        .mtime(0)
        .write(Vec::new(), Compression::best());
    encoder.write_all(bytes)?;
    encoder.finish()
}

// Embed the extracted service interface for local artifacts so
// `icp canister metadata <canister> candid:service` introspection works during
// development. Production `ic` builds skip this path.
pub fn embed_candid_metadata(
    tool: &IcWasmExecutable,
    wasm_path: &Path,
    did_path: &Path,
) -> Result<ArtifactTransformOutput, Box<dyn std::error::Error>> {
    let mut command = Command::new(tool.path());
    command
        .arg(wasm_path)
        .args(["-o"])
        .arg(wasm_path)
        .args(["metadata", "candid:service", "-f"])
        .arg(did_path)
        .args(["-v", "public"]);
    let output = output_with_executable_busy_retry(&mut command);

    let output = output.map_err(|err| {
        format!(
            "failed to run ic-wasm metadata for {}: {err}",
            wasm_path.display()
        )
    })?;

    if !output.status.success() {
        return Err(format!(
            "ic-wasm metadata failed using {} ({}) for {}: {}",
            tool.path().display(),
            tool.version_identity(),
            wasm_path.display(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(transform_output(
        ArtifactTransformKind::CandidMetadata,
        Some(tool.version_identity().to_string()),
        ArtifactTransformOutcome::Applied,
    ))
}

fn optimize_release_wasm_artifact(
    profile: CanisterBuildProfile,
    tool: Option<&BinaryenExecutable>,
    wasm_path: &Path,
) -> Result<ArtifactTransformOutput, Box<dyn std::error::Error>> {
    if profile != CanisterBuildProfile::Release {
        return Ok(ArtifactTransformOutput::not_requested(
            ArtifactTransformKind::Optimize,
        ));
    }
    let tool = tool.expect("release build toolchain always contains Binaryen");
    optimize_release_wasm_artifact_with_tool(tool, wasm_path)
}

#[cfg(test)]
fn optimize_release_wasm_artifact_with_command(
    command_name: &str,
    profile: CanisterBuildProfile,
    wasm_path: &Path,
) -> Result<ArtifactTransformOutput, Box<dyn std::error::Error>> {
    if profile != CanisterBuildProfile::Release {
        return Ok(ArtifactTransformOutput::not_requested(
            ArtifactTransformKind::Optimize,
        ));
    }

    let tool = crate::binaryen::resolve_test_binaryen(command_name)?;
    optimize_release_wasm_artifact_with_tool(&tool, wasm_path)
}

fn optimize_release_wasm_artifact_with_tool(
    tool: &BinaryenExecutable,
    wasm_path: &Path,
) -> Result<ArtifactTransformOutput, Box<dyn std::error::Error>> {
    let before_bytes = fs::read(wasm_path)?;
    let before_contract = wasm::wasm_contract_snapshot(&before_bytes).map_err(|source| {
        format!(
            "failed to inspect release Wasm contract before optimization for {}: {source}",
            wasm_path.display()
        )
    })?;
    let before_features = derive_wasm_features(tool.path(), wasm_path)?;
    let before_gzip = deterministic_gzip_bytes(&before_bytes)?;
    let before_metrics = wasm::wasm_artifact_metrics(&before_bytes, before_gzip.len())?;

    let optimized_path = wasm_path.with_extension("wasm.optimized");
    if let Err(source) =
        run_binaryen_optimizer(tool.path(), wasm_path, &optimized_path, &before_features)
    {
        let _ = fs::remove_file(&optimized_path);
        return Err(source);
    }

    let validation = validate_optimized_wasm(
        tool.path(),
        wasm_path,
        &optimized_path,
        &before_contract,
        &before_features,
        before_metrics,
    );

    let metrics = match validation {
        Ok(metrics) => metrics,
        Err(source) => {
            let _ = fs::remove_file(&optimized_path);
            return Err(source);
        }
    };
    fs::rename(&optimized_path, wasm_path)?;
    eprintln!(
        "release Wasm optimization for {}: raw {} -> {}, gzip {} -> {}, code section {} -> {}, data section {} -> {}, functions {} -> {}",
        wasm_path.display(),
        metrics.before.raw_bytes,
        metrics.after.raw_bytes,
        metrics.before.gzip_bytes,
        metrics.after.gzip_bytes,
        metrics.before.code_section_bytes,
        metrics.after.code_section_bytes,
        metrics.before.data_section_bytes,
        metrics.after.data_section_bytes,
        metrics.before.defined_functions,
        metrics.after.defined_functions,
    );

    Ok(ArtifactTransformOutput {
        transform: ArtifactTransformKind::Optimize,
        tool_version: Some(tool.version_identity().to_string()),
        tool_sha256: Some(tool.sha256().to_string()),
        outcome: ArtifactTransformOutcome::Applied,
        metrics: Some(metrics),
    })
}

fn run_binaryen_optimizer(
    command_path: &Path,
    wasm_path: &Path,
    optimized_path: &Path,
    features: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = Command::new(command_path);
    command
        .arg(wasm_path)
        .arg("-o")
        .arg(optimized_path)
        .arg("-Oz");
    for feature in features {
        command.arg(feature);
    }
    let output = output_with_executable_busy_retry(&mut command).map_err(|source| {
        format!(
            "failed to run required Binaryen optimizer for {}: {source}",
            wasm_path.display()
        )
    })?;
    if !output.status.success() {
        return Err(format!(
            "required Binaryen optimization failed for {} with status {}: {}",
            wasm_path.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }
    Ok(())
}

fn validate_optimized_wasm(
    command_path: &Path,
    wasm_path: &Path,
    optimized_path: &Path,
    before_contract: &wasm::WasmContractSnapshot,
    before_features: &[String],
    before_metrics: crate::canister_build::WasmArtifactMetrics,
) -> Result<WasmTransformMetrics, Box<dyn std::error::Error>> {
    let after_bytes = fs::read(optimized_path).map_err(|source| {
        format!(
            "required Binaryen optimization did not emit {}: {source}",
            optimized_path.display()
        )
    })?;
    let after_contract = wasm::wasm_contract_snapshot(&after_bytes).map_err(|source| {
        format!(
            "failed to inspect release Wasm contract after optimization for {}: {source}",
            wasm_path.display()
        )
    })?;
    if after_contract.exports != before_contract.exports {
        return Err(format!(
            "Binaryen optimization changed the export inventory for {}",
            wasm_path.display()
        )
        .into());
    }
    if after_contract.public_candid_metadata != before_contract.public_candid_metadata {
        return Err(format!(
            "Binaryen optimization changed embedded public Candid metadata for {}",
            wasm_path.display()
        )
        .into());
    }

    let after_features = derive_wasm_features(command_path, optimized_path)?;
    if after_features != before_features {
        return Err(format!(
            "Binaryen optimization changed required Wasm features for {}: before [{}], after [{}]",
            wasm_path.display(),
            before_features.join(", "),
            after_features.join(", ")
        )
        .into());
    }
    let after_gzip = deterministic_gzip_bytes(&after_bytes)?;
    let after_metrics = wasm::wasm_artifact_metrics(&after_bytes, after_gzip.len())?;
    Ok(WasmTransformMetrics {
        before: before_metrics,
        after: after_metrics,
    })
}

fn derive_wasm_features(
    command_path: &Path,
    wasm_path: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut command = Command::new(command_path);
    command.arg(wasm_path);
    for feature in IC_WASM_FEATURE_FLAGS {
        command.arg(feature);
    }
    command.arg("--print-features");
    let output = output_with_executable_busy_retry(&mut command).map_err(|source| {
        format!(
            "failed to derive required Wasm features for {}: {source}",
            wasm_path.display()
        )
    })?;
    if !output.status.success() {
        return Err(format!(
            "Wasm feature validation failed for {} with status {}: {}",
            wasm_path.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }

    let reported = String::from_utf8(output.stdout)?;
    let mut features = Vec::new();
    for line in reported
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if !BINARYEN_REPORTED_IC_WASM_FEATURE_FLAGS.contains(&line) {
            return Err(format!(
                "Wasm artifact {} requires feature `{line}` outside Canic's IC feature contract",
                wasm_path.display()
            )
            .into());
        }
        if !features.iter().any(|feature| feature == line) {
            features.push(line.to_string());
        }
    }
    features.sort();
    Ok(features)
}

const fn transform_output(
    transform: ArtifactTransformKind,
    tool_version: Option<String>,
    outcome: ArtifactTransformOutcome,
) -> ArtifactTransformOutput {
    ArtifactTransformOutput {
        transform,
        tool_version,
        tool_sha256: None,
        outcome,
        metrics: None,
    }
}

#[cfg(test)]
mod tests;

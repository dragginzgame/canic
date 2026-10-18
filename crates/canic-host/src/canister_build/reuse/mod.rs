//! Module: canister_build::reuse
//!
//! Responsibility: reuse an exact finalized complete build after verifying inputs and outputs.
//! Does not own: compilation, release identity allocation, or Fleet deployment.
//! Boundary: source bytes and admitted tools key a cache; immutable release records remain authority.

use ic_build::cargo::observation::{CargoFeatureSelection, schema::Metadata};

mod dependencies;
mod diagnostics;
mod exports;
mod lock;
#[cfg(test)]
mod tests;

use crate::{
    build_toolchain::BuildToolchain,
    canister_build::WorkspaceBuildContext,
    cargo_observation::cargo_metadata_catalog_for_manifest,
    release_build::validate_finalized_release_build_manifest,
    release_set::{
        AppConfigSnapshot, load_persisted_application_artifact_union,
        load_persisted_canic_infrastructure_artifact_manifest,
        load_persisted_current_release_set_manifest,
    },
};
use canic_contracts::ids::ReleaseBuildId;
use canic_core::cdk::utils::hash::hex_bytes;
use ic_build::artifacts::{
    InputEvidence, InputEvidenceError, InputEvidenceSpec, InputEvidenceValue,
};
use ic_host_fs::{
    durable::{PublicationMode, WriteOptions, write_bytes},
    read::{hash_file_no_follow, read_file_no_follow},
};
use serde::{Deserialize, Serialize};
use sha2_host::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, io,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;

pub use exports::retain as retain_cargo_include_exports;

pub use lock::{
    BuildLockInspection, BuildLockOwner, BuildLockPhase, BuildLockWait, BuildProcessActivity,
    BuildProcessIdentity, BuildProcessKind, BuildProcessObservation, BuildProcessVisibility,
    KernelBuildLock, inspect_build_lock,
};

const RECORD_LIMIT: usize = 4 * 1024 * 1024;

///
/// CompleteBuildReuse
///
/// Host-owned invocation lock and byte-derived inputs retained through finalization.
///

pub struct CompleteBuildReuse {
    context: WorkspaceBuildContext,
    tool_paths: Vec<PathBuf>,
    inputs: InputEvidence,
    record_path: PathBuf,
    diagnostics: diagnostics::InputDiagnostics,
    input_locations: diagnostics::InputLocations,
    lock: lock::BuildLock,
}

///
/// BuildReuseProgress
///
/// Host progress while acquiring the complete-build lock, before collecting inputs.
///

pub enum BuildReuseProgress {
    WaitingForLock(BuildLockWait),
    LockFinished(Duration),
}

///
/// ReusedCompleteBuild
///
/// Verified host release returned to the CLI without compilation or transformation.
///

pub struct ReusedCompleteBuild {
    pub release_build_id: ReleaseBuildId,
    pub manifest_path: PathBuf,
    pub roles: Vec<String>,
}

///
/// CompleteBuildReuseRecord
///
/// Host cache index; the finalized release and its manifests remain authoritative.
///

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CompleteBuildReuseRecord {
    schema_version: u8,
    inputs: String,
    release_build_id: ReleaseBuildId,
    files: BTreeMap<String, String>,
    roles: Vec<String>,
}

///
/// BuildReuseError
///
/// Host failure to establish reuse; no cache result authorizes deployment effects.
///

#[derive(Debug, Error)]
pub enum BuildReuseError {
    #[error("complete-build lock acquisition failed: {0}")]
    Lock(io::Error),

    #[error("build inputs changed during compilation; no reusable build was recorded")]
    Changed,

    #[error("build input changed during compilation: {0}; no reusable build was recorded")]
    ChangedInput(PathBuf),

    #[error(
        "build input was first discovered after compilation: {0}; retry to verify it before recording reuse"
    )]
    UnobservedInput(PathBuf),

    #[error("build reuse evidence failed: {0}")]
    Evidence(String),

    #[error("build reuse I/O failed: {0}")]
    Io(#[from] io::Error),

    #[error("build reuse document failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("build input is not a regular file or directory: {0}")]
    Unsupported(PathBuf),

    #[error("complete-build reuse requires a directly bound native tool: {0}")]
    UnboundTool(PathBuf),
}

impl From<InputEvidenceError> for BuildReuseError {
    fn from(error: InputEvidenceError) -> Self {
        match error {
            InputEvidenceError::IdentityChanged | InputEvidenceError::ExclusionsChanged => {
                Self::Changed
            }
            InputEvidenceError::ChangedInput { path } => Self::ChangedInput(path),
            InputEvidenceError::UnobservedInput { path } => Self::UnobservedInput(path),
            InputEvidenceError::Io { path, source } => {
                if source.kind() == io::ErrorKind::InvalidInput || path.to_str().is_none() {
                    Self::Unsupported(path)
                } else {
                    Self::Io(source)
                }
            }
        }
    }
}

impl CompleteBuildReuse {
    pub(crate) fn prepare(
        context: &WorkspaceBuildContext,
        tools: &BuildToolchain,
        mut progress: impl FnMut(BuildReuseProgress) -> io::Result<()>,
    ) -> Result<Self, BuildReuseError> {
        let lock =
            lock::BuildLock::acquire(context, &mut progress).map_err(BuildReuseError::Lock)?;
        prepare_generated_inputs(context)?;
        diagnostics::InputDiagnostics::prepare(&context.icp_root);
        let mut tool_paths = vec![
            env::current_exe()?,
            tools.ic_wasm().path().to_path_buf(),
            resolve_tool(std::ffi::OsStr::new("candid-extractor"))?,
        ];
        if let Some(binaryen) = tools.binaryen() {
            tool_paths.push(binaryen.path().to_path_buf());
        }
        for tool in &tool_paths {
            require_native_tool(tool)?;
        }
        for name in ["CARGO", "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"] {
            let selected = env::var_os(name).or_else(|| match name {
                "CARGO" => Some("cargo".into()),
                "RUSTC" => Some("rustc".into()),
                _ => None,
            });
            if let Some(selected) = selected.filter(|value| !value.is_empty()) {
                let path = resolve_tool(&selected)?;
                require_native_tool(&path)?;
                tool_paths.push(path);
            }
        }
        let inputs = input_snapshot(context, &tool_paths)?;
        let diagnostics = diagnostics::InputDiagnostics::capture(context, &tool_paths, &inputs);
        let input_locations = diagnostics::InputLocations::capture(context);
        let record_path = context
            .icp_root
            .join(".canic/build-reuse")
            .join(format!("{}.json", inputs.digest()?.to_hex()));
        Ok(Self {
            context: context.clone(),
            tool_paths,
            inputs,
            record_path,
            diagnostics,
            input_locations,
            lock,
        })
    }

    /// Explain a miss using optional prior observations, without changing hit admission.
    #[must_use]
    pub fn miss_reason(&self) -> String {
        self.diagnostics
            .explain_miss(&self.context.icp_root.join(".canic/build-reuse"))
    }

    /// Return a hit only after checking every recorded output and all release manifest bindings.
    pub fn load(&self) -> Result<Option<ReusedCompleteBuild>, BuildReuseError> {
        self.lock.phase(BuildLockPhase::VerifyingOutputs);
        let bytes = match read_file_no_follow(&self.record_path, RECORD_LIMIT)
            .map_err(std::io::Error::from)
        {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let record: CompleteBuildReuseRecord = serde_json::from_slice(&bytes)?;
        if record.schema_version != 1
            || record.inputs != self.inputs.digest()?.to_hex()
            || record.files.is_empty()
            || record.roles.is_empty()
        {
            return Err(BuildReuseError::Evidence(
                "invalid complete-build cache record".into(),
            ));
        }
        let config = AppConfigSnapshot::load(&self.context.config_path)
            .map_err(|error| BuildReuseError::Evidence(error.to_string()))?;
        let expected = config
            .model()
            .deployable_roles()
            .into_iter()
            .map(|role| role.to_string())
            .chain(["fleet_coordinator".to_string(), "wasm_store".to_string()])
            .collect::<BTreeSet<_>>();
        let actual = record.roles.iter().cloned().collect::<BTreeSet<_>>();
        if actual != expected || actual.len() != record.roles.len() {
            return Err(BuildReuseError::Evidence(
                "retained role set differs".into(),
            ));
        }
        let directory = self.release_directory(record.release_build_id);
        if output_files(&directory)? != record.files {
            return Err(BuildReuseError::Evidence(
                "retained release bytes changed".into(),
            ));
        }
        let manifest_path = verify_release(&self.context, record.release_build_id)?;
        self.diagnostics
            .retain(&self.context.icp_root.join(".canic/build-reuse"));
        Ok(Some(ReusedCompleteBuild {
            release_build_id: record.release_build_id,
            manifest_path,
            roles: record.roles,
        }))
    }

    /// Publish an advisory phase transition while retaining the same kernel lock.
    pub fn report_building_artifacts(&self) {
        self.lock.phase(BuildLockPhase::BuildingArtifacts);
    }

    /// Record only a finalized release whose governed inputs remained byte-identical during work.
    pub fn record(
        &self,
        release_build_id: ReleaseBuildId,
        roles: Vec<String>,
    ) -> Result<(), BuildReuseError> {
        self.lock.phase(BuildLockPhase::VerifyingAndPublishing);
        verify_release(&self.context, release_build_id)?;
        let after = input_snapshot(&self.context, &self.tool_paths)?;
        self.inputs
            .validate_after(&after)
            .map_err(BuildReuseError::from)
            .inspect_err(|error| {
                if let Some(path) = diagnostics::retain_rejection(
                    &self.context,
                    release_build_id,
                    &self.input_locations,
                    &self.inputs,
                    &after,
                    error,
                ) {
                    eprintln!("Build input rejection evidence: {}", path.display());
                }
            })?;
        let inputs = after.digest()?.to_hex();
        let record_path = self.record_path.with_file_name(format!("{inputs}.json"));
        let record = CompleteBuildReuseRecord {
            schema_version: 1,
            inputs,
            release_build_id,
            files: output_files(&self.release_directory(release_build_id))?,
            roles,
        };
        write_bytes(
            &record_path,
            WriteOptions {
                mode: PublicationMode::Replace,
                permissions: 0o666,
            },
            &serde_json::to_vec(&record)?,
        )
        .map_err(crate::publication::ops::io_error)?;
        diagnostics::InputDiagnostics::capture(&self.context, &self.tool_paths, &after)
            .retain(&self.context.icp_root.join(".canic/build-reuse"));
        Ok(())
    }

    fn release_directory(&self, id: ReleaseBuildId) -> PathBuf {
        self.context
            .icp_root
            .join(".canic/release-builds")
            .join(id.to_string())
    }
}

fn verify_release(
    context: &WorkspaceBuildContext,
    id: ReleaseBuildId,
) -> Result<PathBuf, BuildReuseError> {
    let verify = || -> Result<PathBuf, Box<dyn std::error::Error>> {
        let current = load_persisted_current_release_set_manifest(&context.icp_root, id)?;
        let release =
            validate_finalized_release_build_manifest(&context.icp_root, id, &current.path)?;
        if release.record.build_profile != context.profile
            || release.record.build_network != context.build_network
        {
            return Err("retained build profile/network differs".into());
        }
        let config = AppConfigSnapshot::load(&context.config_path)?;
        let app = load_persisted_application_artifact_union(
            &context.icp_root,
            config.component_topology(),
            id,
        )?;
        let infrastructure =
            load_persisted_canic_infrastructure_artifact_manifest(&context.icp_root, id)?;
        if app.digest != current.manifest.application_artifact_union_sha256
            || infrastructure.digest != current.manifest.infrastructure_artifact_manifest_sha256
        {
            return Err("retained child manifest digest differs".into());
        }
        let fixtures = current
            .manifest
            .verify_fixtures(&context.icp_root, config.component_topology())?;
        crate::release_set::fixture::load_configured_fixture_sources(
            &context.icp_root,
            &context.config_path,
        )?
        .verify_retained(&fixtures)?;
        Ok(current.path)
    };
    verify().map_err(|error| BuildReuseError::Evidence(error.to_string()))
}

fn input_snapshot(
    context: &WorkspaceBuildContext,
    tools: &[PathBuf],
) -> Result<InputEvidence, BuildReuseError> {
    let metadata = cargo_metadata_catalog_for_manifest(
        &context.workspace_root.join("Cargo.toml"),
        true,
        true,
        &CargoFeatureSelection::default(),
    )
    .map_err(|error| BuildReuseError::Evidence(error.to_string()))?;
    let mut roots = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut targets = dependencies::generated_target_names(context)?;
    let producers = selected_export_producers(context, &metadata)?;
    append_generated_inputs(context, &mut paths);
    for package in metadata.packages {
        let manifest = package.manifest_path.into_std_path_buf();
        if package.name == "canic" {
            // Generated infrastructure enables family sources absent from the App's graph.
            roots.extend(
                crate::fleet_package::resolved_family_roots(
                    &manifest,
                    &package.version.to_string(),
                )
                .map_err(|error| BuildReuseError::Evidence(error.to_string()))?
                .into_values()
                .map(|root| root.canonicalize())
                .collect::<Result<Vec<_>, _>>()?,
            );
        }
        roots.insert(
            manifest
                .parent()
                .ok_or_else(|| BuildReuseError::Unsupported(manifest.clone()))?
                .canonicalize()?,
        );
        for target in package.targets {
            targets.insert(target.name.replace('-', "_"));
            let source = target.src_path.into_std_path_buf();
            let path = match source.canonicalize() {
                Ok(path) => path,
                Err(error) if error.kind() == io::ErrorKind::NotFound => source,
                Err(error) => return Err(error.into()),
            };
            paths.insert(path);
        }
    }
    roots.insert(
        context
            .config_path
            .parent()
            .ok_or_else(|| BuildReuseError::Unsupported(context.config_path.clone()))?
            .canonicalize()?,
    );
    paths.extend(roots.iter().cloned());
    dependencies::append_observed_cargo_inputs(context, &targets, &mut paths)?;
    exports::append_observed(context, &producers, &mut paths)?;
    let fixture_hashes = append_fixture_inputs(context, &mut paths)?;
    paths.insert(context.config_path.clone());
    for root in [
        context.workspace_root.as_path(),
        metadata.workspace_root.as_std_path(),
    ] {
        for ancestor in root.ancestors() {
            for name in [
                "Cargo.toml",
                "Cargo.lock",
                "rust-toolchain",
                "rust-toolchain.toml",
                ".cargo/config",
                ".cargo/config.toml",
            ] {
                paths.insert(ancestor.join(name));
            }
        }
    }
    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")));
    if let Some(root) = cargo_home {
        for name in ["config", "config.toml"] {
            paths.insert(root.join(name));
        }
    }
    paths.extend(tools.iter().cloned());
    append_rust_toolchain_inputs(context, &mut paths)?;

    let evidence = InputEvidence::capture(&source_evidence_spec(input_identity(context)?, &paths))?;
    for required in std::iter::once(&context.config_path).chain(tools) {
        if !matches!(
            evidence.entries().get(required),
            Some(InputEvidenceValue::File { .. })
        ) {
            // Keep required-file absence as the original filesystem failure.
            fs::symlink_metadata(required)?;
            return Err(BuildReuseError::Unsupported(required.clone()));
        }
    }
    for (path, expected) in fixture_hashes {
        if !matches!(evidence.entries().get(&path), Some(InputEvidenceValue::File { sha256 }) if sha256.to_hex() == expected)
        {
            return Err(BuildReuseError::ChangedInput(path));
        }
    }
    // Refuse the consumer's non-UTF-8 record encoding before any work or diagnostics.
    evidence.digest()?;
    Ok(evidence)
}

fn selected_export_producers(
    context: &WorkspaceBuildContext,
    metadata: &Metadata,
) -> Result<BTreeMap<PathBuf, String>, BuildReuseError> {
    let mut producers: BTreeMap<_, _> = metadata
        .packages
        .iter()
        .map(|package| {
            (
                package.manifest_path.clone().into_std_path_buf(),
                package.id.repr.clone(),
            )
        })
        .collect();
    for manifest in crate::fleet_package::generated_manifests(&context.config_path) {
        if !manifest.is_file() {
            continue;
        }
        let generated = cargo_metadata_catalog_for_manifest(
            &manifest,
            true,
            true,
            &CargoFeatureSelection::default(),
        )
        .map_err(|error| BuildReuseError::Evidence(error.to_string()))?;
        producers.extend(
            generated
                .packages
                .into_iter()
                .map(|package| (package.manifest_path.into_std_path_buf(), package.id.repr)),
        );
    }
    Ok(producers)
}

fn prepare_generated_inputs(context: &WorkspaceBuildContext) -> Result<(), BuildReuseError> {
    crate::canister_build::prepare_workspace_infrastructure_packages(context)
        .map_err(|error| BuildReuseError::Evidence(error.to_string()))?;
    let config = crate::release_set::AppConfigSnapshot::load(&context.config_path)
        .map_err(|error| BuildReuseError::Evidence(error.to_string()))?;
    let validation = crate::role_contract::validate_declared_role_package(
        &context.config_path,
        config.model(),
        &canic_contracts::ids::CanisterRole::ROOT,
        crate::role_contract::PackageValidationMode::Build,
        &CargoFeatureSelection::default(),
    );
    match validation {
        crate::role_contract::RolePackageValidation::Supported(_) => Ok(()),
        crate::role_contract::RolePackageValidation::Unsupported(finding) => Err(
            BuildReuseError::Evidence(crate::role_contract::finding_detail(&finding)),
        ),
    }
}

fn append_generated_inputs(context: &WorkspaceBuildContext, paths: &mut BTreeSet<PathBuf>) {
    for manifest in crate::fleet_package::generated_manifests(&context.config_path) {
        let root = manifest.parent().expect("generated package directory");
        for name in [
            "Cargo.toml",
            "Cargo.lock",
            "lock-seed.json",
            "src/lib.rs",
            "build.rs",
        ] {
            paths.insert(root.join(name));
        }
    }
}

fn append_fixture_inputs(
    context: &WorkspaceBuildContext,
    paths: &mut BTreeSet<PathBuf>,
) -> Result<BTreeMap<PathBuf, String>, BuildReuseError> {
    let fixtures = crate::release_set::fixture::load_configured_fixture_sources(
        &context.icp_root,
        &context.config_path,
    )
    .map_err(|error| BuildReuseError::Evidence(error.to_string()))?;
    paths.extend(fixtures.source_files.keys().cloned());
    Ok(fixtures.source_files)
}

fn source_evidence_spec(
    identity: impl Into<Vec<u8>>,
    paths: &BTreeSet<PathBuf>,
) -> InputEvidenceSpec {
    let mut spec = InputEvidenceSpec::new(identity);
    // Canic's recipe excludes generated/control entries from recursive source roots.
    for name in ["target", ".git", ".canic", ".icp", ".tmp"] {
        spec = spec.with_excluded_entry_name(name);
    }
    for path in paths {
        spec = spec.with_input(path);
    }
    spec
}

fn input_identity(context: &WorkspaceBuildContext) -> Result<String, BuildReuseError> {
    let mut digest = Sha256::new();
    hash_field(&mut digest, b"canic.complete-build-inputs.v1");
    hash_field(&mut digest, super::cache::CANISTER_CARGO_TARGET.as_bytes());
    hash_field(
        &mut digest,
        include_bytes!("../../build_environment/mod.rs"),
    );
    hash_field(&mut digest, context.profile.target_dir_name().as_bytes());
    hash_field(&mut digest, context.build_network.as_str().as_bytes());
    hash_field(
        &mut digest,
        context.config_path.as_os_str().as_encoded_bytes(),
    );
    let mut environment = crate::build_environment::inputs();
    environment.sort();
    for (key, value) in environment {
        hash_field(&mut digest, key.as_encoded_bytes());
        hash_field(&mut digest, value.as_encoded_bytes());
    }
    for (tool, args) in [("rustc", &["-vV"][..]), ("cargo", &["-V"][..])] {
        let selected = env::var_os(tool.to_uppercase()).unwrap_or_else(|| tool.into());
        let output = crate::build_environment::command(selected)
            .args(args)
            .current_dir(&context.workspace_root)
            .output()?;
        if !output.status.success() {
            return Err(BuildReuseError::Evidence(format!(
                "{tool} identity unavailable"
            )));
        }
        hash_field(&mut digest, &output.stdout);
    }
    Ok(hex_bytes(digest.finalize()))
}

#[cfg(test)]
fn input_digest(
    context: &WorkspaceBuildContext,
    tools: &[PathBuf],
) -> Result<ic_build::artifacts::InputDigest, BuildReuseError> {
    Ok(input_snapshot(context, tools)?.digest()?)
}

fn append_rust_toolchain_inputs(
    context: &WorkspaceBuildContext,
    paths: &mut BTreeSet<PathBuf>,
) -> Result<(), BuildReuseError> {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let sysroot = crate::build_environment::command(&rustc)
        .args(["--print", "sysroot"])
        .current_dir(&context.workspace_root)
        .output()?;
    if !sysroot.status.success() {
        return Err(BuildReuseError::Evidence("Rust sysroot unavailable".into()));
    }
    let sysroot = PathBuf::from(
        String::from_utf8(sysroot.stdout)
            .map_err(|error| BuildReuseError::Evidence(error.to_string()))?
            .trim(),
    );
    for name in ["rustc", "cargo"] {
        let path = sysroot.join("bin").join(name);
        if !fs::symlink_metadata(&path)?.is_file() {
            return Err(BuildReuseError::Unsupported(path));
        }
        paths.insert(path);
    }
    for entry in fs::read_dir(sysroot.join("lib"))? {
        let path = entry?.path();
        if fs::symlink_metadata(&path)?.is_file() {
            paths.insert(path);
        }
    }
    let host = crate::build_environment::command(&rustc)
        .args(["--print", "host-tuple"])
        .current_dir(&context.workspace_root)
        .output()?;
    if !host.status.success() {
        return Err(BuildReuseError::Evidence(
            "Rust host target unavailable".into(),
        ));
    }
    let host = String::from_utf8(host.stdout)
        .map_err(|error| BuildReuseError::Evidence(error.to_string()))?;
    for target in [host.trim(), "wasm32-unknown-unknown"] {
        let root = sysroot.join("lib/rustlib").join(target);
        if !fs::symlink_metadata(&root)?.is_dir() {
            return Err(BuildReuseError::Unsupported(root));
        }
        paths.insert(root);
    }

    Ok(())
}

fn output_files(root: &Path) -> Result<BTreeMap<String, String>, BuildReuseError> {
    let evidence = InputEvidence::capture(&InputEvidenceSpec::new(Vec::new()).with_input(root))?;
    let mut files = BTreeMap::new();
    for (path, value) in evidence.entries() {
        match value {
            InputEvidenceValue::File { sha256 } => {
                let relative = path
                    .strip_prefix(root)
                    .map_err(|error| BuildReuseError::Evidence(error.to_string()))?;
                let key = relative
                    .to_str()
                    .ok_or_else(|| BuildReuseError::Unsupported(path.clone()))?;
                files.insert(key.to_owned(), sha256.to_hex());
            }
            InputEvidenceValue::Directory => {}
            InputEvidenceValue::Missing => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "release output root is missing",
                )
                .into());
            }
        }
    }
    Ok(files)
}

pub(super) fn file_hash(path: &Path) -> Result<String, BuildReuseError> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(BuildReuseError::Unsupported(path.to_path_buf()));
    }
    // Build inputs have no total-byte quota; the shared traversal uses constant storage.
    let identity = hash_file_no_follow(path, u64::MAX).map_err(|error| match error {
        ic_host_artifacts::artifact::ArtifactError::NotRegularFile => {
            BuildReuseError::Unsupported(path.to_path_buf())
        }
        source => BuildReuseError::Io(source.into()),
    })?;
    Ok(identity.sha256.to_string())
}

fn hash_field(digest: &mut Sha256, value: &[u8]) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value);
}

pub(super) fn require_native_tool(path: &Path) -> Result<(), BuildReuseError> {
    let mut file = fs::File::open(path)?;
    let mut magic = [0_u8; 4];
    file.read_exact(&mut magic)?;
    if matches!(
        magic,
        [0x7f, b'E', b'L', b'F']
            | [0xcf, 0xfa, 0xed, 0xfe]
            | [0xfe, 0xed, 0xfa, 0xcf]
            | [0xca, 0xfe, 0xba, 0xbe | 0xbf]
    ) {
        Ok(())
    } else {
        Err(BuildReuseError::UnboundTool(path.to_path_buf()))
    }
}

pub(super) fn resolve_tool(command: &std::ffi::OsStr) -> Result<PathBuf, BuildReuseError> {
    let path = Path::new(command);
    if path.components().count() > 1 {
        return Ok(path.canonicalize()?);
    }
    if let Some(search) = env::var_os("PATH") {
        for directory in env::split_paths(&search) {
            let candidate = directory.join(command);
            if candidate.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt as _;
                    if fs::metadata(&candidate)?.permissions().mode() & 0o111 == 0 {
                        continue;
                    }
                }
                return Ok(candidate.canonicalize()?);
            }
        }
    }
    Err(BuildReuseError::UnboundTool(path.to_path_buf()))
}

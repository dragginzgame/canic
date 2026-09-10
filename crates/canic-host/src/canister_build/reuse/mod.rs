//! Module: canister_build::reuse
//!
//! Responsibility: reuse an exact finalized complete build after verifying inputs and outputs.
//! Does not own: compilation, release identity allocation, or Fleet deployment.
//! Boundary: source bytes and admitted tools key a cache; immutable release records remain authority.

mod dependencies;
mod snapshot;
#[cfg(test)]
mod tests;

use crate::{
    build_toolchain::BuildToolchain,
    canister_build::WorkspaceBuildContext,
    cargo_metadata::cargo_metadata_catalog_for_manifest,
    durable_io::{lock_file, read_regular_bytes, write_bytes},
    release_build::validate_finalized_release_build_manifest,
    release_set::{
        AppConfigSnapshot, load_persisted_application_artifact_union,
        load_persisted_canic_infrastructure_artifact_manifest,
        load_persisted_current_release_set_manifest,
    },
};
use canic_core::ids::ReleaseBuildId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::Command,
};
use thiserror::Error;

use snapshot::BuildInputSnapshot;

const RECORD_LIMIT: usize = 4 * 1024 * 1024;

///
/// CompleteBuildReuse
///
/// Host-owned invocation lock and byte-derived inputs retained through finalization.
///

pub struct CompleteBuildReuse {
    context: WorkspaceBuildContext,
    tool_paths: Vec<PathBuf>,
    inputs: BuildInputSnapshot,
    record_path: PathBuf,
    _lock: fs::File,
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

impl CompleteBuildReuse {
    pub(crate) fn prepare(
        context: &WorkspaceBuildContext,
        tools: &BuildToolchain,
    ) -> Result<Self, BuildReuseError> {
        let lock = lock_file(
            &context
                .icp_root
                .join(".canic/locks/complete-build-reuse.lock"),
        )?;
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
        if env::var_os("RUSTC_WRAPPER").is_none()
            && let Ok(path) = resolve_tool(std::ffi::OsStr::new("sccache"))
        {
            require_native_tool(&path)?;
            tool_paths.push(path);
        }
        let inputs = input_snapshot(context, &tool_paths)?;
        let record_path = context
            .icp_root
            .join(".canic/build-reuse")
            .join(format!("{}.json", inputs.digest()));
        Ok(Self {
            context: context.clone(),
            tool_paths,
            inputs,
            record_path,
            _lock: lock,
        })
    }

    /// Return a hit only after checking every recorded output and all release manifest bindings.
    pub fn load(&self) -> Result<Option<ReusedCompleteBuild>, BuildReuseError> {
        let bytes = match read_regular_bytes(&self.record_path, RECORD_LIMIT) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let record: CompleteBuildReuseRecord = serde_json::from_slice(&bytes)?;
        if record.schema_version != 1
            || record.inputs != self.inputs.digest()
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
        Ok(Some(ReusedCompleteBuild {
            release_build_id: record.release_build_id,
            manifest_path,
            roles: record.roles,
        }))
    }

    /// Record only a finalized release whose governed inputs remained byte-identical during work.
    pub fn record(
        &self,
        release_build_id: ReleaseBuildId,
        roles: Vec<String>,
    ) -> Result<(), BuildReuseError> {
        verify_release(&self.context, release_build_id)?;
        let after = input_snapshot(&self.context, &self.tool_paths)?;
        self.inputs.validate_after(&after)?;
        let inputs = after.digest();
        let record_path = self.record_path.with_file_name(format!("{inputs}.json"));
        let record = CompleteBuildReuseRecord {
            schema_version: 1,
            inputs,
            release_build_id,
            files: output_files(&self.release_directory(release_build_id))?,
            roles,
        };
        write_bytes(&record_path, &serde_json::to_vec(&record)?)?;
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
        Ok(current.path)
    };
    verify().map_err(|error| BuildReuseError::Evidence(error.to_string()))
}

fn input_snapshot(
    context: &WorkspaceBuildContext,
    tools: &[PathBuf],
) -> Result<BuildInputSnapshot, BuildReuseError> {
    let metadata =
        cargo_metadata_catalog_for_manifest(&context.workspace_root.join("Cargo.toml"), true, true)
            .map_err(|error| BuildReuseError::Evidence(error.to_string()))?;
    let mut roots = BTreeSet::new();
    let mut files = BTreeMap::new();
    for package in metadata.packages {
        if package.name == "canic" {
            // Generated infrastructure enables family sources absent from the App's graph.
            roots.extend(
                crate::fleet_package::resolved_family_roots(
                    &package.manifest_path,
                    &package.version,
                )
                .map_err(|error| BuildReuseError::Evidence(error.to_string()))?
                .into_values()
                .map(|root| root.canonicalize())
                .collect::<Result<Vec<_>, _>>()?,
            );
        }
        roots.insert(
            package
                .manifest_path
                .parent()
                .ok_or_else(|| BuildReuseError::Unsupported(package.manifest_path.clone()))?
                .canonicalize()?,
        );
        for target in package.targets {
            let path = match target.src_path.canonicalize() {
                Ok(path) => path,
                Err(error) if error.kind() == io::ErrorKind::NotFound => target.src_path,
                Err(error) => return Err(error.into()),
            };
            add_optional(&path, &mut files)?;
        }
    }
    roots.insert(
        context
            .config_path
            .parent()
            .ok_or_else(|| BuildReuseError::Unsupported(context.config_path.clone()))?
            .canonicalize()?,
    );
    for root in &roots {
        collect_files(root, root, &mut files, true)?;
    }
    dependencies::append_observed_cargo_inputs(context, &mut files)?;
    add_file(&context.config_path, &mut files)?;
    for root in [&context.workspace_root, &metadata.workspace_root] {
        for ancestor in root.ancestors() {
            for name in [
                "Cargo.toml",
                "Cargo.lock",
                "rust-toolchain",
                "rust-toolchain.toml",
                ".cargo/config",
                ".cargo/config.toml",
            ] {
                add_optional(&ancestor.join(name), &mut files)?;
            }
        }
    }
    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")));
    if let Some(root) = cargo_home {
        for name in ["config", "config.toml"] {
            add_optional(&root.join(name), &mut files)?;
        }
    }
    for tool in tools {
        add_file(tool, &mut files)?;
    }
    append_rust_toolchain_inputs(context, &mut files)?;

    Ok(BuildInputSnapshot {
        identity: input_identity(context)?,
        files,
    })
}

fn input_identity(context: &WorkspaceBuildContext) -> Result<String, BuildReuseError> {
    let mut digest = Sha256::new();
    hash_field(&mut digest, b"canic.complete-build-inputs.v1");
    hash_field(&mut digest, context.profile.target_dir_name().as_bytes());
    hash_field(&mut digest, context.build_network.as_str().as_bytes());
    hash_field(
        &mut digest,
        context.config_path.as_os_str().as_encoded_bytes(),
    );
    let mut environment = env::vars_os().collect::<Vec<_>>();
    environment.sort();
    for (key, value) in environment {
        hash_field(&mut digest, key.as_encoded_bytes());
        hash_field(&mut digest, value.as_encoded_bytes());
    }
    for (tool, args) in [("rustc", &["-vV"][..]), ("cargo", &["-V"][..])] {
        let selected = env::var_os(tool.to_uppercase()).unwrap_or_else(|| tool.into());
        let output = Command::new(selected)
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
    Ok(format!("{:x}", digest.finalize()))
}

#[cfg(test)]
fn input_digest(
    context: &WorkspaceBuildContext,
    tools: &[PathBuf],
) -> Result<String, BuildReuseError> {
    Ok(input_snapshot(context, tools)?.digest())
}

fn append_rust_toolchain_inputs(
    context: &WorkspaceBuildContext,
    files: &mut BTreeMap<String, String>,
) -> Result<(), BuildReuseError> {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let sysroot = Command::new(&rustc)
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
        add_file(&sysroot.join("bin").join(name), files)?;
    }
    for entry in fs::read_dir(sysroot.join("lib"))? {
        let path = entry?.path();
        if fs::symlink_metadata(&path)?.is_file() {
            add_file(&path, files)?;
        }
    }
    let host = Command::new(&rustc)
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
        collect_files(&root, &root, files, true)?;
    }

    Ok(())
}

fn output_files(root: &Path) -> Result<BTreeMap<String, String>, BuildReuseError> {
    let mut files = BTreeMap::new();
    collect_files(root, root, &mut files, false)?;
    Ok(files)
}

fn collect_files(
    root: &Path,
    path: &Path,
    files: &mut BTreeMap<String, String>,
    source: bool,
) -> Result<(), BuildReuseError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_file() {
        let key = if source {
            path.to_path_buf()
        } else {
            path.strip_prefix(root)
                .map_err(|error| BuildReuseError::Evidence(error.to_string()))?
                .to_path_buf()
        };
        files.insert(
            key.to_str()
                .ok_or_else(|| BuildReuseError::Unsupported(path.to_path_buf()))?
                .to_string(),
            file_hash(path)?,
        );
    } else if metadata.is_dir() {
        if source {
            files.insert(path.to_string_lossy().into_owned(), "directory".into());
        }
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if source
                && matches!(
                    entry.file_name().to_str(),
                    Some("target" | ".git" | ".canic" | ".icp" | ".tmp")
                )
            {
                continue;
            }
            collect_files(root, &entry.path(), files, source)?;
        }
    } else {
        return Err(BuildReuseError::Unsupported(path.to_path_buf()));
    }
    Ok(())
}

fn add_optional(path: &Path, files: &mut BTreeMap<String, String>) -> Result<(), BuildReuseError> {
    match fs::symlink_metadata(path) {
        Ok(_) => add_file(path, files),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            files.insert(path.to_string_lossy().into_owned(), "absent".to_string());
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

fn add_file(path: &Path, files: &mut BTreeMap<String, String>) -> Result<(), BuildReuseError> {
    files.insert(
        path.to_str()
            .ok_or_else(|| BuildReuseError::Unsupported(path.to_path_buf()))?
            .to_string(),
        file_hash(path)?,
    );
    Ok(())
}

pub(super) fn file_hash(path: &Path) -> Result<String, BuildReuseError> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(BuildReuseError::Unsupported(path.to_path_buf()));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(rustix::fs::OFlags::NOFOLLOW.bits().cast_signed());
    }
    let mut file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(BuildReuseError::Unsupported(path.to_path_buf()));
    }
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
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

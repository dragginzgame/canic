//! Module: ic_wasm
//!
//! Responsibility: own the published `ic-wasm` build-tool authority, installation, and
//! executable admission used by canonical Canic artifact builds.
//! Does not own: artifact transformation order, build provenance shape, or Binaryen policy.
//! Boundary: only the repository-pinned official tool version may transform Canic Wasm.

#[cfg(test)]
mod tests;

use crate::output_with_executable_busy_retry;
use std::{
    env,
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{self, Read},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use canic_core::cdk::utils::hash::hex_bytes;
use sha2::{Digest, Sha256};
use thiserror::Error as ThisError;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub const IC_WASM_REPAIR_COMMAND: &str = "canic toolchain install";
pub const IC_WASM_TOOL: &str = "ic-wasm";
pub const IC_WASM_VERSION: &str = "0.11.1";
pub const IC_WASM_VERSION_IDENTITY: &str = "ic-wasm 0.11.1";

const DOWNLOAD_TOOL: &str = "curl";
const EXTRACT_TOOL: &str = "tar";
const TEMP_ATTEMPTS: usize = 64;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

///
/// IcWasmAuthority
///
/// Immutable official archive identity for one install-capable host platform.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IcWasmAuthority {
    archive_platform: &'static str,
    archive_sha256: &'static str,
}

impl IcWasmAuthority {
    #[must_use]
    pub const fn archive_platform(self) -> &'static str {
        self.archive_platform
    }

    #[must_use]
    pub const fn archive_sha256(self) -> &'static str {
        self.archive_sha256
    }

    fn archive_name(self) -> String {
        format!("ic-wasm-{}.tar.xz", self.archive_platform)
    }

    fn archive_url(self) -> String {
        format!(
            "https://github.com/dfinity/ic-wasm/releases/download/{IC_WASM_VERSION}/{}",
            self.archive_name()
        )
    }

    fn package_name(self) -> String {
        format!("ic-wasm-{}", self.archive_platform)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct IcWasmPlatformAuthority {
    os: &'static str,
    arch: &'static str,
    authority: IcWasmAuthority,
}

const SUPPORTED_IC_WASM_AUTHORITIES: [IcWasmPlatformAuthority; 4] = [
    IcWasmPlatformAuthority {
        os: "macos",
        arch: "aarch64",
        authority: IcWasmAuthority {
            archive_platform: "aarch64-apple-darwin",
            archive_sha256: "1feeb253498b783ce19e9e166d4f205ed35a8ab7fa679173aaa4b41fb78c852d",
        },
    },
    IcWasmPlatformAuthority {
        os: "macos",
        arch: "x86_64",
        authority: IcWasmAuthority {
            archive_platform: "x86_64-apple-darwin",
            archive_sha256: "9bf63f9daaee8d812207807435a1bff23d7b7e50c573b7c2a36b8aa50974e99a",
        },
    },
    IcWasmPlatformAuthority {
        os: "linux",
        arch: "aarch64",
        authority: IcWasmAuthority {
            archive_platform: "aarch64-unknown-linux-gnu",
            archive_sha256: "49d5992ee5f050f8869b6b4b8357eceb1cb3b84f79fbd2de37ed8166c5dc5e30",
        },
    },
    IcWasmPlatformAuthority {
        os: "linux",
        arch: "x86_64",
        authority: IcWasmAuthority {
            archive_platform: "x86_64-unknown-linux-gnu",
            archive_sha256: "099776a745c4d4495761da18f2fe2216759a4166beacd05453bf031d61631746",
        },
    },
];

///
/// IcWasmExecutable
///
/// Exact admitted `ic-wasm` executable consumed by one artifact-build invocation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcWasmExecutable {
    path: PathBuf,
    version_identity: String,
}

impl IcWasmExecutable {
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn version_identity(&self) -> &str {
        &self.version_identity
    }
}

///
/// IcWasmToolError
///
/// Typed setup or admission failure for the mandatory canonical Wasm helper.
///

#[derive(Debug, ThisError)]
pub enum IcWasmToolError {
    #[error("ic-wasm archive download failed with status {status}: {stderr}")]
    ArchiveDownload { status: String, stderr: String },

    #[error("downloaded ic-wasm archive {path} has SHA-256 {actual}; required {expected}")]
    ArchiveHashMismatch {
        path: PathBuf,
        actual: String,
        expected: &'static str,
    },

    #[error("ic-wasm archive extraction failed with status {status}: {stderr}")]
    ArchiveExtraction { status: String, stderr: String },

    #[error("staged ic-wasm executable {path} has SHA-256 {actual}; required {expected}")]
    ExecutableHashMismatch {
        path: PathBuf,
        actual: String,
        expected: String,
    },

    #[error("failed to {operation} {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("HOME is unavailable; `{IC_WASM_REPAIR_COMMAND}` cannot select ~/.local/bin")]
    MissingHome,

    #[error(
        "required {IC_WASM_TOOL} {IC_WASM_VERSION} was not found on PATH or at {canonical_path}; run `{IC_WASM_REPAIR_COMMAND}`"
    )]
    MissingTool { canonical_path: PathBuf },

    #[error(
        "required {IC_WASM_TOOL} {IC_WASM_VERSION} was not found on PATH and HOME is unavailable; run `{IC_WASM_REPAIR_COMMAND}` with HOME set to the intended account home"
    )]
    MissingToolWithoutHome,

    #[error(
        "required {IC_WASM_TOOL} {IC_WASM_VERSION} was not found on PATH or at {canonical_path}; HOME resolves to `/`, so confirm that root-level install location is intentional before running `{IC_WASM_REPAIR_COMMAND}`"
    )]
    MissingToolWithRootHome { canonical_path: PathBuf },

    #[error("installed ic-wasm candidate is not executable: {path}")]
    NotExecutable { path: PathBuf },

    #[error(
        "requested ic-wasm executable {path} is missing or not executable; artifact builds require ic-wasm {IC_WASM_VERSION}; run `{IC_WASM_REPAIR_COMMAND}`"
    )]
    RequestedExecutableMissing { path: PathBuf },

    #[error("temporary ic-wasm installation directory allocation was exhausted under {root}")]
    TempDirectoryExhausted { root: PathBuf },

    #[error("unsupported ic-wasm platform: {os} {arch}")]
    UnsupportedPlatform {
        os: &'static str,
        arch: &'static str,
    },

    #[error(
        "selected ic-wasm executable {path} reports `{actual}`; required `{expected}`; run `{IC_WASM_REPAIR_COMMAND}`"
    )]
    VersionMismatch {
        path: PathBuf,
        actual: String,
        expected: &'static str,
    },

    #[error("failed to inspect ic-wasm version at {path}: {source}")]
    VersionProcess {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Return the archive authority for the current install-capable host platform.
pub fn current_ic_wasm_authority() -> Result<IcWasmAuthority, IcWasmToolError> {
    ic_wasm_authority_for(env::consts::OS, env::consts::ARCH)
}

fn ic_wasm_authority_for(
    os: &'static str,
    arch: &'static str,
) -> Result<IcWasmAuthority, IcWasmToolError> {
    SUPPORTED_IC_WASM_AUTHORITIES
        .iter()
        .find(|projection| (projection.os, projection.arch) == (os, arch))
        .map(|projection| projection.authority)
        .ok_or(IcWasmToolError::UnsupportedPlatform { os, arch })
}

/// Resolve and admit the governed install path, falling back to the first `ic-wasm` on PATH.
pub fn resolve_required_ic_wasm() -> Result<IcWasmExecutable, IcWasmToolError> {
    current_ic_wasm_authority()?;
    let path = resolve_selected_executable()?;
    admit_ic_wasm_executable(&path)
}

/// Install the official current-platform `ic-wasm` under `~/.local/bin`.
pub fn install_required_ic_wasm() -> Result<IcWasmExecutable, IcWasmToolError> {
    let authority = current_ic_wasm_authority()?;
    let install_path = default_ic_wasm_install_path()?;
    let temp = TempDirectory::create()?;
    let archive_path = temp.path().join(authority.archive_name());
    download_archive(authority, &archive_path)?;
    verify_archive(authority, &archive_path)?;
    extract_archive(authority, &archive_path, temp.path())?;
    let candidate = temp
        .path()
        .join(authority.package_name())
        .join(IC_WASM_TOOL);
    admit_ic_wasm_executable(&candidate)?;
    publish_executable(&candidate, &install_path)?;
    admit_ic_wasm_executable(&install_path)
}

/// Return the fixed installation path used by setup and fallback resolution.
pub fn default_ic_wasm_install_path() -> Result<PathBuf, IcWasmToolError> {
    let home = env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .ok_or(IcWasmToolError::MissingHome)?;
    Ok(PathBuf::from(home).join(".local/bin/ic-wasm"))
}

fn resolve_selected_executable() -> Result<PathBuf, IcWasmToolError> {
    match default_ic_wasm_install_path() {
        Ok(path) if is_executable(&path) => return canonical_executable(&path),
        Ok(_) | Err(IcWasmToolError::MissingHome) => {}
        Err(error) => return Err(error),
    }
    if let Some(path) = executable_on_path(OsStr::new(IC_WASM_TOOL))? {
        return Ok(path);
    }
    match default_ic_wasm_install_path() {
        Ok(path) if is_executable(&path) => canonical_executable(&path),
        Ok(canonical_path) if home_is_root() => {
            Err(IcWasmToolError::MissingToolWithRootHome { canonical_path })
        }
        Ok(canonical_path) => Err(IcWasmToolError::MissingTool { canonical_path }),
        Err(IcWasmToolError::MissingHome) => Err(IcWasmToolError::MissingToolWithoutHome),
        Err(error) => Err(error),
    }
}

fn executable_on_path(command: &OsStr) -> Result<Option<PathBuf>, IcWasmToolError> {
    let Some(path) = env::var_os("PATH") else {
        return Ok(None);
    };
    for directory in env::split_paths(&path) {
        let candidate = directory.join(command);
        if is_executable(&candidate) {
            return canonical_executable(&candidate).map(Some);
        }
    }
    Ok(None)
}

fn canonical_executable(path: &Path) -> Result<PathBuf, IcWasmToolError> {
    if !is_executable(path) {
        return Err(IcWasmToolError::RequestedExecutableMissing {
            path: path.to_path_buf(),
        });
    }
    fs::canonicalize(path).map_err(|source| IcWasmToolError::Io {
        operation: "resolve ic-wasm executable",
        path: path.to_path_buf(),
        source,
    })
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        false
    }
}

fn admit_ic_wasm_executable(path: &Path) -> Result<IcWasmExecutable, IcWasmToolError> {
    let path = canonical_executable(path)?;
    if !is_executable(&path) {
        return Err(IcWasmToolError::NotExecutable { path });
    }
    let mut command = Command::new(&path);
    command.arg("--version");
    let output = output_with_executable_busy_retry(&mut command).map_err(|source| {
        IcWasmToolError::VersionProcess {
            path: path.clone(),
            source,
        }
    })?;
    let actual = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        String::from_utf8_lossy(&output.stderr).trim().to_string()
    };
    if !output.status.success() || actual != IC_WASM_VERSION_IDENTITY {
        return Err(IcWasmToolError::VersionMismatch {
            path,
            actual,
            expected: IC_WASM_VERSION_IDENTITY,
        });
    }
    Ok(IcWasmExecutable {
        path,
        version_identity: actual,
    })
}

fn download_archive(
    authority: IcWasmAuthority,
    archive_path: &Path,
) -> Result<(), IcWasmToolError> {
    let mut command = Command::new(DOWNLOAD_TOOL);
    command
        .args([
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--tlsv1.2",
            "-fsSL",
            "-o",
        ])
        .arg(archive_path)
        .arg(authority.archive_url());
    let output =
        output_with_executable_busy_retry(&mut command).map_err(|source| IcWasmToolError::Io {
            operation: "run curl for ic-wasm archive",
            path: archive_path.to_path_buf(),
            source,
        })?;
    if !output.status.success() {
        return Err(IcWasmToolError::ArchiveDownload {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(())
}

fn verify_archive(authority: IcWasmAuthority, archive_path: &Path) -> Result<(), IcWasmToolError> {
    let actual = sha256_file(archive_path)?;
    if actual != authority.archive_sha256() {
        return Err(IcWasmToolError::ArchiveHashMismatch {
            path: archive_path.to_path_buf(),
            actual,
            expected: authority.archive_sha256(),
        });
    }
    Ok(())
}

fn extract_archive(
    authority: IcWasmAuthority,
    archive_path: &Path,
    destination: &Path,
) -> Result<(), IcWasmToolError> {
    let member = format!("{}/{IC_WASM_TOOL}", authority.package_name());
    let mut command = Command::new(EXTRACT_TOOL);
    command
        .arg("-xJf")
        .arg(archive_path)
        .arg("-C")
        .arg(destination)
        .arg(member);
    let output =
        output_with_executable_busy_retry(&mut command).map_err(|source| IcWasmToolError::Io {
            operation: "run tar for ic-wasm archive",
            path: archive_path.to_path_buf(),
            source,
        })?;
    if !output.status.success() {
        return Err(IcWasmToolError::ArchiveExtraction {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(())
}

fn publish_executable(candidate: &Path, destination: &Path) -> Result<(), IcWasmToolError> {
    let parent = destination.parent().ok_or_else(|| IcWasmToolError::Io {
        operation: "select ic-wasm installation directory",
        path: destination.to_path_buf(),
        source: io::Error::new(io::ErrorKind::InvalidInput, "destination has no parent"),
    })?;
    fs::create_dir_all(parent).map_err(|source| IcWasmToolError::Io {
        operation: "create ic-wasm installation directory",
        path: parent.to_path_buf(),
        source,
    })?;
    let expected_sha256 = sha256_file(candidate)?;
    let stage = parent.join(format!(
        ".ic-wasm.canic-install-{}-{}",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut source = File::open(candidate).map_err(|source| IcWasmToolError::Io {
            operation: "open admitted ic-wasm executable",
            path: candidate.to_path_buf(),
            source,
        })?;
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&stage)
            .map_err(|source| IcWasmToolError::Io {
                operation: "create staged ic-wasm executable",
                path: stage.clone(),
                source,
            })?;
        io::copy(&mut source, &mut output).map_err(|source| IcWasmToolError::Io {
            operation: "write staged ic-wasm executable",
            path: stage.clone(),
            source,
        })?;
        #[cfg(unix)]
        fs::set_permissions(&stage, fs::Permissions::from_mode(0o755)).map_err(|source| {
            IcWasmToolError::Io {
                operation: "set staged ic-wasm executable permissions",
                path: stage.clone(),
                source,
            }
        })?;
        output.sync_all().map_err(|source| IcWasmToolError::Io {
            operation: "sync staged ic-wasm executable",
            path: stage.clone(),
            source,
        })?;
        drop(output);
        let actual = sha256_file(&stage)?;
        if actual != expected_sha256 {
            return Err(IcWasmToolError::ExecutableHashMismatch {
                path: stage.clone(),
                actual,
                expected: expected_sha256.clone(),
            });
        }
        admit_ic_wasm_executable(&stage)?;
        fs::rename(&stage, destination).map_err(|source| IcWasmToolError::Io {
            operation: "publish ic-wasm executable",
            path: destination.to_path_buf(),
            source,
        })?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| IcWasmToolError::Io {
                operation: "sync ic-wasm installation directory",
                path: parent.to_path_buf(),
                source,
            })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&stage);
    }
    result
}

fn sha256_file(path: &Path) -> Result<String, IcWasmToolError> {
    let mut file = File::open(path).map_err(|source| IcWasmToolError::Io {
        operation: "open file for SHA-256",
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|source| IcWasmToolError::Io {
                operation: "read file for SHA-256",
                path: path.to_path_buf(),
                source,
            })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_bytes(hasher.finalize()))
}

fn home_is_root() -> bool {
    env::var_os("HOME").is_some_and(|home| Path::new(&home) == Path::new("/"))
}

#[cfg(test)]
pub(crate) fn resolve_test_ic_wasm(command: &str) -> Result<IcWasmExecutable, IcWasmToolError> {
    let path = canonical_executable(Path::new(command))?;
    admit_ic_wasm_executable(&path)
}

struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    fn create() -> Result<Self, IcWasmToolError> {
        let root = env::temp_dir();
        for _ in 0..TEMP_ATTEMPTS {
            let path = root.join(format!(
                "canic-ic-wasm-install-{}-{}",
                std::process::id(),
                TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
                Err(source) => {
                    return Err(IcWasmToolError::Io {
                        operation: "create temporary ic-wasm installation directory",
                        path,
                        source,
                    });
                }
            }
        }
        Err(IcWasmToolError::TempDirectoryExhausted { root })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

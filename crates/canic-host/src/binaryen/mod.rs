//! Module: binaryen
//!
//! Responsibility: own the published Binaryen release-tool authority, installation, and
//! executable admission used by canonical release-Wasm builds.
//! Does not own: Wasm transformation policy, build provenance shape, or release-set publication.
//! Boundary: only the checksum-pinned official platform executable may reach the optimizer.

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

pub const BINARYEN_REPAIR_COMMAND: &str = "canic toolchain install";
pub const BINARYEN_VERSION: &str = "132";
pub const BINARYEN_VERSION_IDENTITY: &str = "wasm-opt version 132 (version_132)";
pub const WASM_OPT_TOOL: &str = "wasm-opt";

const DOWNLOAD_TOOL: &str = "curl";
const EXTRACT_TOOL: &str = "tar";
const TEMP_ATTEMPTS: usize = 64;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

///
/// BinaryenAuthority
///
/// Immutable official archive and executable identities for one supported host platform.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryenAuthority {
    archive_platform: &'static str,
    archive_sha256: &'static str,
    executable_sha256: &'static str,
}

impl BinaryenAuthority {
    #[must_use]
    pub const fn archive_platform(self) -> &'static str {
        self.archive_platform
    }

    #[must_use]
    pub const fn archive_sha256(self) -> &'static str {
        self.archive_sha256
    }

    #[must_use]
    pub const fn executable_sha256(self) -> &'static str {
        self.executable_sha256
    }

    fn archive_name(self) -> String {
        format!(
            "binaryen-version_{BINARYEN_VERSION}-{}.tar.gz",
            self.archive_platform
        )
    }

    fn archive_url(self) -> String {
        format!(
            "https://github.com/WebAssembly/binaryen/releases/download/version_{BINARYEN_VERSION}/{}",
            self.archive_name()
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BinaryenPlatformAuthority {
    os: &'static str,
    arch: &'static str,
    authority: BinaryenAuthority,
}

const SUPPORTED_BINARYEN_AUTHORITIES: [BinaryenPlatformAuthority; 3] = [
    BinaryenPlatformAuthority {
        os: "macos",
        arch: "aarch64",
        authority: BinaryenAuthority {
            archive_platform: "arm64-macos",
            archive_sha256: "98aad827847af7ef990ed7098d885725c8e5b5aae75073403635617ae4e259aa",
            executable_sha256: "a9c8d09d84186e4c8efe937f3de19b887404d24a96e2638f3bd3b476e17b7218",
        },
    },
    BinaryenPlatformAuthority {
        os: "macos",
        arch: "x86_64",
        authority: BinaryenAuthority {
            archive_platform: "x86_64-macos",
            archive_sha256: "40c3de90bb3766bd0282a895e139a6f50253dba49b4f5bb89e66faca162d832e",
            executable_sha256: "c3cbd288eef3402119d8183df1739887ff0e6430caba2e1c801406df725a2bd3",
        },
    },
    BinaryenPlatformAuthority {
        os: "linux",
        arch: "x86_64",
        authority: BinaryenAuthority {
            archive_platform: "x86_64-linux",
            archive_sha256: "195ddc94f9bc89f45abdabb0b9eea86023d727ba90eac8b35b80f2544fc30572",
            executable_sha256: "1014958e6f20d412f1542320b43970214b0fb1ed780595e8f7c0d8761ed53725",
        },
    },
];

///
/// BinaryenExecutable
///
/// Exact admitted optimizer executable consumed by one release-Wasm finalization.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryenExecutable {
    path: PathBuf,
    version_identity: String,
    sha256: String,
}

impl BinaryenExecutable {
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn version_identity(&self) -> &str {
        &self.version_identity
    }

    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

///
/// BinaryenToolError
///
/// Typed setup or admission failure for the mandatory release optimizer.
///

#[derive(Debug, ThisError)]
pub enum BinaryenToolError {
    #[error("Binaryen archive download failed with status {status}: {stderr}")]
    ArchiveDownload { status: String, stderr: String },

    #[error("downloaded Binaryen archive {path} has SHA-256 {actual}; required {expected}")]
    ArchiveHashMismatch {
        path: PathBuf,
        actual: String,
        expected: &'static str,
    },

    #[error("Binaryen archive extraction failed with status {status}: {stderr}")]
    ArchiveExtraction { status: String, stderr: String },

    #[error(
        "selected Binaryen executable {path} has SHA-256 {actual}; required {expected}; run `{BINARYEN_REPAIR_COMMAND}`"
    )]
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

    #[error("HOME is unavailable; `{BINARYEN_REPAIR_COMMAND}` cannot select ~/.local/bin")]
    MissingHome,

    #[error(
        "release Wasm optimization requires Binaryen {BINARYEN_VERSION}; `{WASM_OPT_TOOL}` was not found on PATH or at {canonical_path}; run `{BINARYEN_REPAIR_COMMAND}`"
    )]
    MissingOptimizer { canonical_path: PathBuf },

    #[error(
        "release Wasm optimization requires Binaryen {BINARYEN_VERSION}; `{WASM_OPT_TOOL}` was not found on PATH and HOME is unavailable; run `{BINARYEN_REPAIR_COMMAND}` with HOME set to the intended account home"
    )]
    MissingOptimizerWithoutHome,

    #[error(
        "release Wasm optimization requires Binaryen {BINARYEN_VERSION}; `{WASM_OPT_TOOL}` was not found on PATH or at {canonical_path}; HOME resolves to `/`, so confirm that root-level install location is intentional before running `{BINARYEN_REPAIR_COMMAND}`"
    )]
    MissingOptimizerWithRootHome { canonical_path: PathBuf },

    #[error("installed Binaryen candidate is not executable: {path}")]
    NotExecutable { path: PathBuf },

    #[error(
        "requested Binaryen executable {path} is missing or not executable; release Wasm optimization requires Binaryen {BINARYEN_VERSION}; run `{BINARYEN_REPAIR_COMMAND}`"
    )]
    RequestedExecutableMissing { path: PathBuf },

    #[error("temporary Binaryen installation directory allocation was exhausted under {root}")]
    TempDirectoryExhausted { root: PathBuf },

    #[error("unsupported Binaryen platform: {os} {arch}")]
    UnsupportedPlatform {
        os: &'static str,
        arch: &'static str,
    },

    #[error(
        "selected Binaryen executable {path} reports `{actual}`; required `{expected}`; run `{BINARYEN_REPAIR_COMMAND}`"
    )]
    VersionMismatch {
        path: PathBuf,
        actual: String,
        expected: &'static str,
    },

    #[error("failed to inspect Binaryen version at {path}: {source}")]
    VersionProcess {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Return the checksum authority for the current supported host platform.
pub fn current_binaryen_authority() -> Result<BinaryenAuthority, BinaryenToolError> {
    binaryen_authority_for(env::consts::OS, env::consts::ARCH)
}

fn binaryen_authority_for(
    os: &'static str,
    arch: &'static str,
) -> Result<BinaryenAuthority, BinaryenToolError> {
    SUPPORTED_BINARYEN_AUTHORITIES
        .iter()
        .find(|projection| (projection.os, projection.arch) == (os, arch))
        .map(|projection| projection.authority)
        .ok_or(BinaryenToolError::UnsupportedPlatform { os, arch })
}

/// Resolve and admit the governed install path, falling back to the first optimizer on PATH.
pub fn resolve_required_binaryen() -> Result<BinaryenExecutable, BinaryenToolError> {
    let authority = current_binaryen_authority()?;
    let path = resolve_executable(OsStr::new(WASM_OPT_TOOL))?;
    admit_binaryen_executable(&path, authority.executable_sha256())
}

/// Install the official current-platform optimizer under `~/.local/bin`.
///
/// Canic invokes the returned admitted absolute path directly.
pub fn install_required_binaryen() -> Result<BinaryenExecutable, BinaryenToolError> {
    let authority = current_binaryen_authority()?;
    let install_path = default_binaryen_install_path()?;
    let temp = TempDirectory::create()?;
    let archive_path = temp.path().join(authority.archive_name());
    download_archive(authority, &archive_path)?;
    verify_archive(authority, &archive_path)?;
    extract_archive(&archive_path, temp.path())?;
    let candidate = temp
        .path()
        .join(format!("binaryen-version_{BINARYEN_VERSION}/bin/wasm-opt"));
    admit_binaryen_executable(&candidate, authority.executable_sha256())?;
    publish_executable(&candidate, &install_path, authority.executable_sha256())?;
    admit_binaryen_executable(&install_path, authority.executable_sha256())
}

/// Return the fixed downstream installation path named by repair diagnostics.
pub fn default_binaryen_install_path() -> Result<PathBuf, BinaryenToolError> {
    let home = env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .ok_or(BinaryenToolError::MissingHome)?;
    Ok(PathBuf::from(home).join(".local/bin/wasm-opt"))
}

fn resolve_executable(command: &OsStr) -> Result<PathBuf, BinaryenToolError> {
    let requested = Path::new(command);
    if requested.components().count() > 1 {
        return canonical_executable(requested);
    }
    match default_binaryen_install_path() {
        Ok(path) if is_executable(&path) => return canonical_executable(&path),
        Ok(_) | Err(BinaryenToolError::MissingHome) => {}
        Err(error) => return Err(error),
    }
    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            let candidate = directory.join(requested);
            if is_executable(&candidate) {
                return canonical_executable(&candidate);
            }
        }
    }
    match default_binaryen_install_path() {
        Ok(path) if is_executable(&path) => canonical_executable(&path),
        Ok(canonical_path) if home_is_root() => {
            Err(BinaryenToolError::MissingOptimizerWithRootHome { canonical_path })
        }
        Ok(canonical_path) => Err(BinaryenToolError::MissingOptimizer { canonical_path }),
        Err(BinaryenToolError::MissingHome) => Err(BinaryenToolError::MissingOptimizerWithoutHome),
        Err(error) => Err(error),
    }
}

fn canonical_executable(path: &Path) -> Result<PathBuf, BinaryenToolError> {
    if !is_executable(path) {
        return Err(BinaryenToolError::RequestedExecutableMissing {
            path: path.to_path_buf(),
        });
    }
    fs::canonicalize(path).map_err(|source| BinaryenToolError::Io {
        operation: "resolve Binaryen executable",
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

fn admit_binaryen_executable(
    path: &Path,
    expected_sha256: &str,
) -> Result<BinaryenExecutable, BinaryenToolError> {
    let path = fs::canonicalize(path).map_err(|source| BinaryenToolError::Io {
        operation: "resolve Binaryen executable",
        path: path.to_path_buf(),
        source,
    })?;
    if !is_executable(&path) {
        return Err(BinaryenToolError::NotExecutable { path });
    }
    let sha256 = sha256_file(&path)?;
    if sha256 != expected_sha256 {
        return Err(BinaryenToolError::ExecutableHashMismatch {
            path,
            actual: sha256,
            expected: expected_sha256.to_string(),
        });
    }

    let mut command = Command::new(&path);
    command.arg("--version");
    let output = output_with_executable_busy_retry(&mut command).map_err(|source| {
        BinaryenToolError::VersionProcess {
            path: path.clone(),
            source,
        }
    })?;
    let actual = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        String::from_utf8_lossy(&output.stderr).trim().to_string()
    };
    if !output.status.success() || actual != BINARYEN_VERSION_IDENTITY {
        return Err(BinaryenToolError::VersionMismatch {
            path,
            actual,
            expected: BINARYEN_VERSION_IDENTITY,
        });
    }

    Ok(BinaryenExecutable {
        path,
        version_identity: actual,
        sha256,
    })
}

fn download_archive(
    authority: BinaryenAuthority,
    archive_path: &Path,
) -> Result<(), BinaryenToolError> {
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
    let output = output_with_executable_busy_retry(&mut command).map_err(|source| {
        BinaryenToolError::Io {
            operation: "run curl for Binaryen archive",
            path: archive_path.to_path_buf(),
            source,
        }
    })?;
    if !output.status.success() {
        return Err(BinaryenToolError::ArchiveDownload {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(())
}

fn verify_archive(
    authority: BinaryenAuthority,
    archive_path: &Path,
) -> Result<(), BinaryenToolError> {
    let actual = sha256_file(archive_path)?;
    if actual != authority.archive_sha256() {
        return Err(BinaryenToolError::ArchiveHashMismatch {
            path: archive_path.to_path_buf(),
            actual,
            expected: authority.archive_sha256(),
        });
    }
    Ok(())
}

fn extract_archive(archive_path: &Path, destination: &Path) -> Result<(), BinaryenToolError> {
    let member = format!("binaryen-version_{BINARYEN_VERSION}/bin/wasm-opt");
    let mut command = Command::new(EXTRACT_TOOL);
    command
        .arg("-xzf")
        .arg(archive_path)
        .arg("-C")
        .arg(destination)
        .arg(member);
    let output = output_with_executable_busy_retry(&mut command).map_err(|source| {
        BinaryenToolError::Io {
            operation: "run tar for Binaryen archive",
            path: archive_path.to_path_buf(),
            source,
        }
    })?;
    if !output.status.success() {
        return Err(BinaryenToolError::ArchiveExtraction {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(())
}

fn publish_executable(
    candidate: &Path,
    destination: &Path,
    expected_sha256: &str,
) -> Result<(), BinaryenToolError> {
    let parent = destination.parent().ok_or_else(|| BinaryenToolError::Io {
        operation: "select Binaryen installation directory",
        path: destination.to_path_buf(),
        source: io::Error::new(io::ErrorKind::InvalidInput, "destination has no parent"),
    })?;
    fs::create_dir_all(parent).map_err(|source| BinaryenToolError::Io {
        operation: "create Binaryen installation directory",
        path: parent.to_path_buf(),
        source,
    })?;
    let stage = parent.join(format!(
        ".wasm-opt.canic-install-{}-{}",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut source = File::open(candidate).map_err(|source| BinaryenToolError::Io {
            operation: "open admitted Binaryen executable",
            path: candidate.to_path_buf(),
            source,
        })?;
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&stage)
            .map_err(|source| BinaryenToolError::Io {
                operation: "create staged Binaryen executable",
                path: stage.clone(),
                source,
            })?;
        io::copy(&mut source, &mut output).map_err(|source| BinaryenToolError::Io {
            operation: "write staged Binaryen executable",
            path: stage.clone(),
            source,
        })?;
        #[cfg(unix)]
        fs::set_permissions(&stage, fs::Permissions::from_mode(0o755)).map_err(|source| {
            BinaryenToolError::Io {
                operation: "set staged Binaryen executable permissions",
                path: stage.clone(),
                source,
            }
        })?;
        output.sync_all().map_err(|source| BinaryenToolError::Io {
            operation: "sync staged Binaryen executable",
            path: stage.clone(),
            source,
        })?;
        drop(output);
        admit_binaryen_executable(&stage, expected_sha256)?;
        fs::rename(&stage, destination).map_err(|source| BinaryenToolError::Io {
            operation: "publish Binaryen executable",
            path: destination.to_path_buf(),
            source,
        })?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| BinaryenToolError::Io {
                operation: "sync Binaryen installation directory",
                path: parent.to_path_buf(),
                source,
            })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&stage);
    }
    result
}

fn sha256_file(path: &Path) -> Result<String, BinaryenToolError> {
    let mut file = File::open(path).map_err(|source| BinaryenToolError::Io {
        operation: "open file for SHA-256",
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|source| BinaryenToolError::Io {
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
pub(crate) fn resolve_test_binaryen(
    command: &str,
) -> Result<BinaryenExecutable, BinaryenToolError> {
    let path = resolve_executable(OsStr::new(command))?;
    let expected = sha256_file(&path)?;
    admit_binaryen_executable(&path, &expected)
}

struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    fn create() -> Result<Self, BinaryenToolError> {
        let root = env::temp_dir();
        for _ in 0..TEMP_ATTEMPTS {
            let path = root.join(format!(
                "canic-binaryen-install-{}-{}",
                std::process::id(),
                TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
                Err(source) => {
                    return Err(BinaryenToolError::Io {
                        operation: "create temporary Binaryen installation directory",
                        path,
                        source,
                    });
                }
            }
        }
        Err(BinaryenToolError::TempDirectoryExhausted { root })
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

//! Module: binaryen
//!
//! Responsibility: own the published Binaryen release-tool authority, installation, and
//! executable admission used by canonical release-Wasm builds.
//! Does not own: Wasm transformation policy, build provenance shape, or release-set publication.
//! Boundary: only the checksum-pinned official platform executable may reach the optimizer.

#[cfg(test)]
mod tests;

use crate::{
    output_with_executable_busy_retry,
    tool_install::{self, ArchiveFormat, InstallError, InstallSpec, sha256_file},
    tool_resolution,
};
use std::{
    env,
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
};

use thiserror::Error as ThisError;

pub const BINARYEN_REPAIR_COMMAND: &str = "canic toolchain install";
pub const BINARYEN_VERSION: &str = "132";
pub const BINARYEN_VERSION_IDENTITY: &str = "wasm-opt version 132 (version_132)";
pub const WASM_OPT_TOOL: &str = "wasm-opt";

///
/// BinaryenAuthority
///
/// Official archive, executable and runtime-library identities for one supported host.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryenAuthority {
    archive_platform: &'static str,
    archive_sha256: &'static str,
    executable_sha256: &'static str,
    runtime_library_sha256: Option<&'static str>,
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
            runtime_library_sha256: Some(
                "6627f4f3f3655bfc14b3cd4816b0e7b0cb62ce6a530bab00cdd26855d5f6359b",
            ),
        },
    },
    BinaryenPlatformAuthority {
        os: "macos",
        arch: "x86_64",
        authority: BinaryenAuthority {
            archive_platform: "x86_64-macos",
            archive_sha256: "40c3de90bb3766bd0282a895e139a6f50253dba49b4f5bb89e66faca162d832e",
            executable_sha256: "c3cbd288eef3402119d8183df1739887ff0e6430caba2e1c801406df725a2bd3",
            runtime_library_sha256: Some(
                "f6d540a50c12af1769c10e30775625f70b65327ff8ee438c7059bc076147000a",
            ),
        },
    },
    BinaryenPlatformAuthority {
        os: "linux",
        arch: "x86_64",
        authority: BinaryenAuthority {
            archive_platform: "x86_64-linux",
            archive_sha256: "195ddc94f9bc89f45abdabb0b9eea86023d727ba90eac8b35b80f2544fc30572",
            executable_sha256: "1014958e6f20d412f1542320b43970214b0fb1ed780595e8f7c0d8761ed53725",
            runtime_library_sha256: None,
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

    #[error(
        "selected Binaryen runtime library {path} has SHA-256 {actual}; required {expected}; run `{BINARYEN_REPAIR_COMMAND}`"
    )]
    RuntimeLibraryHashMismatch {
        path: PathBuf,
        actual: String,
        expected: String,
    },

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

impl From<InstallError> for BinaryenToolError {
    fn from(error: InstallError) -> Self {
        match error {
            InstallError::ArchiveDownload { status, stderr } => {
                Self::ArchiveDownload { status, stderr }
            }
            InstallError::ArchiveExtraction { status, stderr } => {
                Self::ArchiveExtraction { status, stderr }
            }
            InstallError::ArchiveHashMismatch {
                path,
                actual,
                expected,
            } => Self::ArchiveHashMismatch {
                path,
                actual,
                expected,
            },
            InstallError::ExecutableHashMismatch {
                path,
                actual,
                expected,
            } => Self::ExecutableHashMismatch {
                path,
                actual,
                expected,
            },
            InstallError::Io {
                operation,
                path,
                source,
            } => Self::Io {
                operation,
                path,
                source,
            },
            InstallError::TempDirectoryExhausted { root } => Self::TempDirectoryExhausted { root },
        }
    }
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
    admit_binaryen_executable(
        &path,
        authority.executable_sha256(),
        authority.runtime_library_sha256,
    )
}

/// Install the official current-platform optimizer under `~/.local/bin`.
///
/// Canic invokes the returned admitted absolute path directly.
pub fn install_required_binaryen() -> Result<BinaryenExecutable, BinaryenToolError> {
    let authority = current_binaryen_authority()?;
    let install_path = default_binaryen_install_path()?;
    let runtime_library = format!("binaryen-version_{BINARYEN_VERSION}/lib/libbinaryen.dylib");
    let spec = InstallSpec {
        tool: WASM_OPT_TOOL,
        archive_name: &authority.archive_name(),
        archive_url: &authority.archive_url(),
        archive_sha256: authority.archive_sha256(),
        member: &format!("binaryen-version_{BINARYEN_VERSION}/bin/wasm-opt"),
        runtime_library: authority
            .archive_platform()
            .ends_with("-macos")
            .then_some(runtime_library.as_str()),
        format: ArchiveFormat::Gzip,
    };
    tool_install::install(&spec, &install_path, |path| {
        admit_binaryen_executable(
            path,
            authority.executable_sha256(),
            authority.runtime_library_sha256,
        )
    })
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
    let canonical = default_binaryen_install_path().ok();
    let mut directories = Vec::new();
    if let Some(parent) = canonical.as_ref().and_then(|path| path.parent()) {
        directories.push(parent.to_path_buf());
    }
    if let Some(path) = env::var_os("PATH") {
        directories.extend(env::split_paths(&path));
    }
    let selected = tool_resolution::resolve(requested, &directories).map_err(|source| {
        BinaryenToolError::Io {
            operation: "resolve Binaryen executable",
            path: requested.to_path_buf(),
            source,
        }
    })?;
    if let Some(path) = selected {
        return Ok(path);
    }
    if command.as_encoded_bytes().contains(&b'/') {
        return Err(BinaryenToolError::RequestedExecutableMissing {
            path: requested.to_path_buf(),
        });
    }
    match canonical {
        Some(canonical_path) if home_is_root() => {
            Err(BinaryenToolError::MissingOptimizerWithRootHome { canonical_path })
        }
        Some(canonical_path) => Err(BinaryenToolError::MissingOptimizer { canonical_path }),
        None => Err(BinaryenToolError::MissingOptimizerWithoutHome),
    }
}

fn admit_binaryen_executable(
    path: &Path,
    expected_sha256: &str,
    expected_runtime_sha256: Option<&str>,
) -> Result<BinaryenExecutable, BinaryenToolError> {
    let path = tool_resolution::resolve(path, &[])
        .map_err(|source| BinaryenToolError::Io {
            operation: "resolve Binaryen executable",
            path: path.to_path_buf(),
            source,
        })?
        .ok_or_else(|| BinaryenToolError::NotExecutable {
            path: path.to_path_buf(),
        })?;
    let sha256 = sha256_file(&path)?;
    if sha256 != expected_sha256 {
        return Err(BinaryenToolError::ExecutableHashMismatch {
            path,
            actual: sha256,
            expected: expected_sha256.to_string(),
        });
    }
    if let Some(expected) = expected_runtime_sha256 {
        admit_runtime_library(&path, expected)?;
    }

    let mut command = crate::build_environment::command(&path);
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

fn admit_runtime_library(executable: &Path, expected: &str) -> Result<(), BinaryenToolError> {
    let library = executable.with_file_name("../lib/libbinaryen.dylib");
    let metadata = std::fs::symlink_metadata(&library).map_err(|source| BinaryenToolError::Io {
        operation: "inspect Binaryen runtime library",
        path: library.clone(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(BinaryenToolError::Io {
            operation: "inspect Binaryen runtime library",
            path: library,
            source: io::Error::new(
                io::ErrorKind::InvalidData,
                "runtime library must be a regular file",
            ),
        });
    }
    let actual = sha256_file(&library)?;
    if actual != expected {
        return Err(BinaryenToolError::RuntimeLibraryHashMismatch {
            path: library,
            actual,
            expected: expected.to_string(),
        });
    }
    Ok(())
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
    admit_binaryen_executable(&path, &expected, None)
}

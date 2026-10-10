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
pub const BINARYEN_VERSION: &str = "133";
pub const BINARYEN_VERSION_IDENTITY: &str = "wasm-opt version 133 (version_133)";
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
            archive_sha256: "ad66da82ac13f163e424b1643f16c6dfcccc98b5966296b43e52d3cab04f84a8",
            executable_sha256: "81041e09f332df94db1c2009a64d8f3b85f0431a2c920ccd014d3ceebb343402",
            runtime_library_sha256: Some(
                "61055e190d84d5db6d1dec63456e0c24dad324ecfd4d2e23740dca217ed89e5a",
            ),
        },
    },
    BinaryenPlatformAuthority {
        os: "macos",
        arch: "x86_64",
        authority: BinaryenAuthority {
            archive_platform: "x86_64-macos",
            archive_sha256: "13a9b90be775c6389ce3d1f879cb8627bea56708ba8c122983941d53a8199b95",
            executable_sha256: "e26344b1d0d0986ac4a1090f58e478470eb2a52ba0625ae9a0f880511bb31d51",
            runtime_library_sha256: Some(
                "26388343133e968f58c18807552b83c43944d11cf512e3920798890adc554f38",
            ),
        },
    },
    BinaryenPlatformAuthority {
        os: "linux",
        arch: "x86_64",
        authority: BinaryenAuthority {
            archive_platform: "x86_64-linux",
            archive_sha256: "2dc9c7813f5375db93d96ead4b78222fcc3e2677bbb832297af4797782a37489",
            executable_sha256: "8f25e9fd5db0fc5f210003aaa432922feb2e52d309e430def2f929e34da9466b",
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
    current_binaryen_authority()?;
    let path = resolve_executable(OsStr::new(WASM_OPT_TOOL))?;
    admit_required_binaryen(&path)
}

/// Admit an explicitly selected optimizer against the current platform authority.
pub(crate) fn admit_required_binaryen(
    path: &Path,
) -> Result<BinaryenExecutable, BinaryenToolError> {
    let authority = current_binaryen_authority()?;
    admit_binaryen_executable(
        path,
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

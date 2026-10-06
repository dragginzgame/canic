//! Module: ic_wasm
//!
//! Responsibility: own the published `ic-wasm` build-tool authority, installation, and
//! executable admission used by canonical Canic artifact builds.
//! Does not own: artifact transformation order, build provenance shape, or Binaryen policy.
//! Boundary: only the repository-pinned official tool version may transform Canic Wasm.

#[cfg(test)]
mod tests;

use crate::{
    output_with_executable_busy_retry,
    tool_install::{self, ArchiveFormat, InstallError, InstallSpec, sha256_file},
    tool_resolution,
};
use std::{
    env, io,
    path::{Path, PathBuf},
};

use thiserror::Error as ThisError;

pub const IC_WASM_REPAIR_COMMAND: &str = "canic toolchain install";
pub const IC_WASM_TOOL: &str = "ic-wasm";
pub const IC_WASM_VERSION: &str = "0.11.1";
pub const IC_WASM_VERSION_IDENTITY: &str = "ic-wasm 0.11.1";

// Exact launcher from the integrity-verified @icp-sdk/ic-wasm 0.11.1 npm archive.
// Bind the executable it selects rather than a script whose payload can change independently.
const NPM_LAUNCHER_SHA256: &str =
    "ff4f9bd1d3734f7aa69078ecd4c5716dbfaf67d094c69d5082b00bac5b8bf936";

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

impl From<InstallError> for IcWasmToolError {
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
    let path = resolve_distribution_executable(&path)?;
    admit_ic_wasm_executable(&path)
}

/// Install the official current-platform `ic-wasm` under `~/.local/bin`.
pub fn install_required_ic_wasm() -> Result<IcWasmExecutable, IcWasmToolError> {
    let authority = current_ic_wasm_authority()?;
    let install_path = default_ic_wasm_install_path()?;
    let spec = InstallSpec {
        tool: IC_WASM_TOOL,
        archive_name: &authority.archive_name(),
        archive_url: &authority.archive_url(),
        archive_sha256: authority.archive_sha256(),
        member: &format!("{}/{}", authority.package_name(), IC_WASM_TOOL),
        format: ArchiveFormat::Xz,
        runtime_library: None,
    };
    tool_install::install(&spec, &install_path, admit_ic_wasm_executable)
}

/// Return the fixed installation path used by setup and fallback resolution.
pub fn default_ic_wasm_install_path() -> Result<PathBuf, IcWasmToolError> {
    let home = env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .ok_or(IcWasmToolError::MissingHome)?;
    Ok(PathBuf::from(home).join(".local/bin/ic-wasm"))
}

fn resolve_distribution_executable(path: &Path) -> Result<PathBuf, IcWasmToolError> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("js")
        || sha256_file(path)? != NPM_LAUNCHER_SHA256
    {
        return Ok(path.to_path_buf());
    }
    let cwd = env::current_dir().map_err(|source| IcWasmToolError::Io {
        operation: "resolve npm ic-wasm invocation directory",
        path: path.to_path_buf(),
        source,
    })?;
    let candidates = npm_binary_candidates(path, &cwd, env::consts::OS, env::consts::ARCH)?;
    for candidate in candidates {
        if candidate.exists() {
            return tool_resolution::resolve(&candidate, &[])
                .map_err(|source| IcWasmToolError::Io {
                    operation: "resolve ic-wasm distribution executable",
                    path: candidate.clone(),
                    source,
                })?
                .ok_or(IcWasmToolError::RequestedExecutableMissing { path: candidate });
        }
    }
    Err(IcWasmToolError::RequestedExecutableMissing {
        path: path.to_path_buf(),
    })
}

fn npm_binary_candidates(
    launcher: &Path,
    cwd: &Path,
    os: &'static str,
    arch: &'static str,
) -> Result<[PathBuf; 3], IcWasmToolError> {
    let platform = match (os, arch) {
        ("linux", "x86_64") => "linux-x64",
        ("linux", "aarch64") => "linux-arm64",
        ("macos", "x86_64") => "darwin-x64",
        ("macos", "aarch64") => "darwin-arm64",
        _ => return Err(IcWasmToolError::UnsupportedPlatform { os, arch }),
    };
    let directory =
        launcher
            .parent()
            .ok_or_else(|| IcWasmToolError::RequestedExecutableMissing {
                path: launcher.to_path_buf(),
            })?;
    let binary = format!("@icp-sdk/ic-wasm-{platform}/bin/ic-wasm");
    Ok([
        directory.join("../../..").join(&binary),
        directory.join("../node_modules").join(&binary),
        cwd.join("node_modules").join(binary),
    ])
}

fn resolve_selected_executable() -> Result<PathBuf, IcWasmToolError> {
    let canonical = default_ic_wasm_install_path().ok();
    let mut directories = Vec::new();
    if let Some(parent) = canonical.as_ref().and_then(|path| path.parent()) {
        directories.push(parent.to_path_buf());
    }
    if let Some(path) = env::var_os("PATH") {
        directories.extend(env::split_paths(&path));
    }
    let selected =
        tool_resolution::resolve(Path::new(IC_WASM_TOOL), &directories).map_err(|source| {
            IcWasmToolError::Io {
                operation: "resolve ic-wasm executable",
                path: PathBuf::from(IC_WASM_TOOL),
                source,
            }
        })?;
    if let Some(path) = selected {
        return Ok(path);
    }
    match canonical {
        Some(canonical_path) if home_is_root() => {
            Err(IcWasmToolError::MissingToolWithRootHome { canonical_path })
        }
        Some(canonical_path) => Err(IcWasmToolError::MissingTool { canonical_path }),
        None => Err(IcWasmToolError::MissingToolWithoutHome),
    }
}

fn admit_ic_wasm_executable(path: &Path) -> Result<IcWasmExecutable, IcWasmToolError> {
    let path = tool_resolution::resolve(path, &[])
        .map_err(|source| IcWasmToolError::Io {
            operation: "resolve ic-wasm executable",
            path: path.to_path_buf(),
            source,
        })?
        .ok_or_else(|| IcWasmToolError::RequestedExecutableMissing {
            path: path.to_path_buf(),
        })?;
    let mut command = crate::build_environment::command(&path);
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

fn home_is_root() -> bool {
    env::var_os("HOME").is_some_and(|home| Path::new(&home) == Path::new("/"))
}

#[cfg(test)]
pub(crate) fn resolve_test_ic_wasm(command: &str) -> Result<IcWasmExecutable, IcWasmToolError> {
    admit_ic_wasm_executable(Path::new(command))
}

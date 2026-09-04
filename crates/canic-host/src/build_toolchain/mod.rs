//! Module: build_toolchain
//!
//! Responsibility: resolve or install one exact external-tool set for a Canic build invocation.
//! Does not own: Wasm transformation behavior, Cargo compilation, or operator shell setup.
//! Boundary: build orchestration receives admitted absolute executables before artifact work starts.

#[cfg(test)]
mod tests;

use crate::{
    binaryen::{
        BinaryenExecutable, BinaryenToolError, current_binaryen_authority,
        install_required_binaryen, resolve_required_binaryen,
    },
    build_profile::CanisterBuildProfile,
    ic_wasm::{
        IcWasmExecutable, IcWasmToolError, current_ic_wasm_authority, install_required_ic_wasm,
        resolve_required_ic_wasm,
    },
};
use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

///
/// BuildToolchain
///
/// Absolute admitted tools retained for every artifact transform in one build invocation.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildToolchain {
    profile: CanisterBuildProfile,
    ic_wasm: IcWasmExecutable,
    binaryen: Option<BinaryenExecutable>,
}

impl BuildToolchain {
    /// Resolve every required tool before Cargo or artifact mutation begins.
    pub fn resolve(profile: CanisterBuildProfile) -> Result<Self, BuildToolchainError> {
        let ic_wasm = resolve_required_ic_wasm()?;
        let binaryen = match profile {
            CanisterBuildProfile::Release => Some(resolve_required_binaryen()?),
            CanisterBuildProfile::Debug | CanisterBuildProfile::Fast => None,
        };
        Ok(Self {
            profile,
            ic_wasm,
            binaryen,
        })
    }

    /// Return concise exact-path diagnostics for the admitted invocation tools.
    #[must_use]
    pub fn diagnostic_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "ic-wasm: {} ({})",
            self.ic_wasm.path().display(),
            self.ic_wasm.version_identity()
        )];
        if let Some(binaryen) = &self.binaryen {
            lines.push(format!(
                "wasm-opt: {} ({})",
                binaryen.path().display(),
                binaryen.version_identity()
            ));
        }
        lines
    }

    pub(crate) fn require_profile(
        &self,
        profile: CanisterBuildProfile,
    ) -> Result<(), BuildToolchainError> {
        if self.profile != profile {
            return Err(BuildToolchainError::ProfileMismatch {
                resolved: self.profile,
                requested: profile,
            });
        }
        Ok(())
    }

    pub(crate) const fn ic_wasm(&self) -> &IcWasmExecutable {
        &self.ic_wasm
    }

    pub(crate) const fn binaryen(&self) -> Option<&BinaryenExecutable> {
        self.binaryen.as_ref()
    }
}

///
/// InstalledBuildToolchain
///
/// Complete checksum-admitted tool set returned by the public installation command.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledBuildToolchain {
    ic_wasm: IcWasmExecutable,
    binaryen: BinaryenExecutable,
    install_directory: PathBuf,
}

impl InstalledBuildToolchain {
    #[must_use]
    pub const fn ic_wasm(&self) -> &IcWasmExecutable {
        &self.ic_wasm
    }

    #[must_use]
    pub const fn binaryen(&self) -> &BinaryenExecutable {
        &self.binaryen
    }

    #[must_use]
    pub fn install_directory(&self) -> &Path {
        self.install_directory.as_path()
    }

    #[must_use]
    pub fn root_home_warning(&self) -> Option<String> {
        let home = env::var_os("HOME");
        root_home_warning_for(home.as_deref(), self.install_directory())
    }
}

fn root_home_warning_for(home: Option<&OsStr>, install_directory: &Path) -> Option<String> {
    home.filter(|home| Path::new(home) == Path::new("/"))
        .map(|_| {
            format!(
                "warning: HOME resolves to `/`; tools were installed under {}. Set HOME to the intended account home and reinstall if this root-level location is accidental.",
                install_directory.display()
            )
        })
}

///
/// BuildToolchainError
///
/// Typed resolution, installation, or profile-binding failure for canonical build tools.
///

#[derive(Debug, ThisError)]
pub enum BuildToolchainError {
    #[error(transparent)]
    Binaryen(#[from] BinaryenToolError),

    #[error(transparent)]
    IcWasm(#[from] IcWasmToolError),

    #[error(
        "build toolchain was resolved for {resolved:?}, but artifact finalization requested {requested:?}"
    )]
    ProfileMismatch {
        resolved: CanisterBuildProfile,
        requested: CanisterBuildProfile,
    },

    #[error(
        "installed Wasm tools resolved different directories: ic-wasm at {ic_wasm}, wasm-opt at {binaryen}"
    )]
    SplitInstallDirectories { ic_wasm: PathBuf, binaryen: PathBuf },
}

/// Install and admit both canonical Wasm tools for the current host.
pub fn install_required_build_toolchain() -> Result<InstalledBuildToolchain, BuildToolchainError> {
    // Fail unsupported platform projections before downloading or publishing either tool.
    current_binaryen_authority()?;
    current_ic_wasm_authority()?;

    let ic_wasm = install_required_ic_wasm()?;
    let binaryen = install_required_binaryen()?;
    let ic_wasm_directory = ic_wasm
        .path()
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| BuildToolchainError::SplitInstallDirectories {
            ic_wasm: ic_wasm.path().to_path_buf(),
            binaryen: binaryen.path().to_path_buf(),
        })?;
    if binaryen.path().parent() != Some(ic_wasm_directory.as_path()) {
        return Err(BuildToolchainError::SplitInstallDirectories {
            ic_wasm: ic_wasm.path().to_path_buf(),
            binaryen: binaryen.path().to_path_buf(),
        });
    }
    Ok(InstalledBuildToolchain {
        ic_wasm,
        binaryen,
        install_directory: ic_wasm_directory,
    })
}

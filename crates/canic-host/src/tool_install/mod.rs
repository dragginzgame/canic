//! Shared mechanics for installing repository-pinned host tools.
//!
//! Tool owners supply archive authority and executable admission. This module owns
//! download, extraction and durable staging, without choosing versions or trust rules.

#[cfg(test)]
mod tests;

use crate::output_with_executable_busy_retry;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    env,
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const TEMP_ATTEMPTS: usize = 64;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

///
/// ArchiveFormat
///
/// Exact archive extraction format selected by the tool's pinned distribution.
///

pub enum ArchiveFormat {
    Gzip,
    Xz,
}

///
/// InstallSpec
///
/// Tool-owned archive identity, executable and optional relative runtime library.
///

pub struct InstallSpec<'a> {
    pub tool: &'static str,
    pub archive_name: &'a str,
    pub archive_url: &'a str,
    pub archive_sha256: &'static str,
    pub member: &'a str,
    pub runtime_library: Option<&'a str>,
    pub format: ArchiveFormat,
}

///
/// InstallError
///
/// Mechanical failures translated into each tool's existing diagnostic type.
///

#[derive(Debug)]
pub enum InstallError {
    ArchiveDownload {
        status: String,
        stderr: String,
    },

    ArchiveExtraction {
        status: String,
        stderr: String,
    },

    ArchiveHashMismatch {
        path: PathBuf,
        actual: String,
        expected: &'static str,
    },

    ExecutableHashMismatch {
        path: PathBuf,
        actual: String,
        expected: String,
    },

    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },

    TempDirectoryExhausted {
        root: PathBuf,
    },
}

/// Admit both candidate and staged executable before publishing, then admit the installed path.
pub fn install<T, E: From<InstallError>>(
    spec: &InstallSpec<'_>,
    destination: &Path,
    admit: impl Fn(&Path) -> Result<T, E>,
) -> Result<T, E> {
    let temp = TempDirectory::create(spec.tool)?;
    let archive = temp.path.join(spec.archive_name);
    download_archive(spec.archive_url, &archive)?;
    verify_archive(&archive, spec.archive_sha256)?;
    extract_archive(spec, &archive, &temp.path)?;
    let candidate = temp.path.join(spec.member);
    admit(&candidate)?;
    if let Some(library) = spec.runtime_library {
        publish_bundle(
            spec.tool,
            &candidate,
            &temp.path.join(library),
            destination,
            |path| admit(path).map(|_| ()),
        )?;
    } else {
        publish_executable(spec.tool, &candidate, destination, |path| {
            admit(path).map(|_| ())
        })?;
    }
    admit(destination)
}

fn download_archive(url: &str, archive: &Path) -> Result<(), InstallError> {
    let mut command = crate::build_environment::command("curl");
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
        .arg(archive)
        .arg(url);
    let output = output_with_executable_busy_retry(&mut command)
        .map_err(|source| io_error("download tool archive", archive, source))?;
    if !output.status.success() {
        return Err(InstallError::ArchiveDownload {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(())
}

fn verify_archive(archive: &Path, expected: &'static str) -> Result<(), InstallError> {
    let actual = sha256_file(archive)?;
    if actual != expected {
        return Err(InstallError::ArchiveHashMismatch {
            path: archive.to_path_buf(),
            actual,
            expected,
        });
    }
    Ok(())
}

fn extract_archive(
    spec: &InstallSpec<'_>,
    archive: &Path,
    destination: &Path,
) -> Result<(), InstallError> {
    let compression = match spec.format {
        ArchiveFormat::Gzip => "-xzf",
        ArchiveFormat::Xz => "-xJf",
    };
    let mut command = crate::build_environment::command("tar");
    command
        .arg(compression)
        .arg(archive)
        .arg("-C")
        .arg(destination)
        .arg(spec.member);
    if let Some(library) = spec.runtime_library {
        command.arg(library);
    }
    let output = output_with_executable_busy_retry(&mut command)
        .map_err(|source| io_error("extract tool archive", archive, source))?;
    if !output.status.success() {
        return Err(InstallError::ArchiveExtraction {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(())
}

/// Publish exact candidate bytes only after closing the writer and admitting the staged path.
pub fn publish_executable<E: From<InstallError>>(
    tool: &str,
    candidate: &Path,
    destination: &Path,
    admit: impl Fn(&Path) -> Result<(), E>,
) -> Result<(), E> {
    let parent = destination.parent().ok_or_else(|| {
        io_error(
            "select tool installation directory",
            destination,
            io::Error::new(io::ErrorKind::InvalidInput, "destination has no parent"),
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|source| io_error("create tool installation directory", parent, source))?;
    let expected = sha256_file(candidate)?;
    let stage = parent.join(format!(
        ".{tool}.canic-install-{}-{}",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        stage_executable(candidate, &stage)?;
        let actual = sha256_file(&stage)?;
        if actual != expected {
            return Err(InstallError::ExecutableHashMismatch {
                path: stage.clone(),
                actual,
                expected,
            }
            .into());
        }
        admit(&stage)?;
        fs::rename(&stage, destination)
            .map_err(|source| io_error("publish tool executable", destination, source))?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| E::from(io_error("sync tool installation directory", parent, source)))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&stage);
    }
    result
}

fn stage_executable(candidate: &Path, stage: &Path) -> Result<(), InstallError> {
    let mut source = File::open(candidate)
        .map_err(|source| io_error("open admitted tool executable", candidate, source))?;
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(stage)
        .map_err(|source| io_error("create staged tool executable", stage, source))?;
    io::copy(&mut source, &mut output)
        .map_err(|source| io_error("write staged tool executable", stage, source))?;
    #[cfg(unix)]
    fs::set_permissions(stage, fs::Permissions::from_mode(0o755))
        .map_err(|source| io_error("set staged tool executable permissions", stage, source))?;
    output
        .sync_all()
        .map_err(|source| io_error("sync staged tool executable", stage, source))?;
    Ok(())
}

/// Qualify a complete immutable bin/lib layout, then atomically select it.
/// Failed admission leaves the previous installation intact. Published bundles
/// remain available to processes holding their canonical executable paths.
#[cfg(unix)]
fn publish_bundle<E: From<InstallError>>(
    tool: &str,
    candidate: &Path,
    library: &Path,
    destination: &Path,
    admit: impl Fn(&Path) -> Result<(), E>,
) -> Result<(), E> {
    let parent = destination.parent().ok_or_else(|| {
        io_error(
            "select tool installation directory",
            destination,
            io::Error::new(io::ErrorKind::InvalidInput, "destination has no parent"),
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|source| io_error("create tool installation directory", parent, source))?;
    let parent = fs::canonicalize(parent)
        .map_err(|source| io_error("resolve tool installation directory", parent, source))?;
    let bundle = parent.join(format!(
        ".canic-{tool}-bundle-{}-{}",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&bundle).map_err(|source| io_error("create tool bundle", &bundle, source))?;
    let link = bundle.join("selected");
    let mut published = false;
    let result = (|| {
        let bin = bundle.join("bin");
        let lib = bundle.join("lib");
        for directory in [&bin, &lib] {
            fs::create_dir(directory)
                .map_err(|source| io_error("create tool bundle directory", directory, source))?;
        }
        let executable = bin.join(tool);
        let staged_library = lib.join("libbinaryen.dylib");
        stage_bundle_member(candidate, &executable)?;
        stage_bundle_member(library, &staged_library)?;
        admit(&executable)?;
        for directory in [&bin, &lib, &bundle] {
            File::open(directory)
                .and_then(|file| file.sync_all())
                .map_err(|source| io_error("sync tool bundle directory", directory, source))?;
        }
        std::os::unix::fs::symlink(&executable, &link)
            .map_err(|source| io_error("stage tool bundle selection", &link, source))?;
        File::open(&bundle)
            .and_then(|file| file.sync_all())
            .map_err(|source| io_error("sync tool bundle selection", &bundle, source))?;
        fs::rename(&link, destination)
            .map_err(|source| io_error("publish tool bundle", destination, source))?;
        published = true;
        File::open(&parent)
            .and_then(|file| file.sync_all())
            .map_err(|source| {
                E::from(io_error(
                    "sync tool installation directory",
                    &parent,
                    source,
                ))
            })
    })();
    if !published {
        let _ = fs::remove_dir_all(&bundle);
    }
    result
}

fn stage_bundle_member(source: &Path, stage: &Path) -> Result<(), InstallError> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| io_error("inspect tool bundle member", source, error))?;
    if !metadata.is_file() {
        return Err(io_error(
            "inspect tool bundle member",
            source,
            io::Error::new(
                io::ErrorKind::InvalidData,
                "bundle member must be a regular file",
            ),
        ));
    }
    let expected = sha256_file(source)?;
    stage_executable(source, stage)?;
    let actual = sha256_file(stage)?;
    if actual != expected {
        return Err(InstallError::ExecutableHashMismatch {
            path: stage.to_path_buf(),
            actual,
            expected,
        });
    }
    Ok(())
}

pub fn sha256_file(path: &Path) -> Result<String, InstallError> {
    // Distribution owners retain their exact digest pins. Hashing streams in
    // constant space and accepts every representable file size.
    ic_host_tools::artifact::hash_file(path, u64::MAX)
        .map(|identity| identity.sha256.to_string())
        .map_err(|source| {
            let source = match source {
                ic_host_tools::artifact::ArtifactError::Io(source) => source,
                source => io::Error::other(source),
            };
            io_error("hash admitted tool file", path, source)
        })
}

fn io_error(operation: &'static str, path: &Path, source: io::Error) -> InstallError {
    InstallError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    fn create(tool: &str) -> Result<Self, InstallError> {
        let root = env::temp_dir();
        for _ in 0..TEMP_ATTEMPTS {
            let path = root.join(format!(
                "canic-{tool}-install-{}-{}",
                std::process::id(),
                TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
                Err(source) => {
                    return Err(io_error(
                        "create temporary tool installation directory",
                        &path,
                        source,
                    ));
                }
            }
        }
        Err(InstallError::TempDirectoryExhausted { root })
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

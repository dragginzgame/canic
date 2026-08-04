#[cfg(test)]
use crate::durable_io::write_bytes;
use crate::{
    durable_io::{
        RegularFileReadError, create_new_bytes_with_parents, read_optional_regular_bytes,
    },
    fleet_install_plan::PlannedCanisterCreationFunding,
    icp::{self, LocalReplicaTarget},
};
#[cfg(test)]
use candid::CandidType;
use canic_core::{cdk::types::Principal, ids::SubnetId};
use serde_json::Value as JsonValue;
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
    process::Command,
};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub(super) enum CreationResultReadError {
    #[error("Canister creation result is not a regular no-follow file: {path}")]
    Unsafe { path: PathBuf },

    #[error("Canister creation result is invalid: {path}")]
    Invalid { path: PathBuf },

    #[error("failed to read Canister creation result {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[cfg(not(unix))]
    #[error("Canister creation result reads are unsupported: {path}")]
    UnsupportedPlatform { path: PathBuf },
}

fn parse_created_canister_id(output: &str) -> Option<Principal> {
    if let Ok(value) = serde_json::from_str::<JsonValue>(output) {
        return parse_canister_id_json(&value);
    }

    output
        .lines()
        .map(str::trim)
        .find_map(|line| Principal::from_text(line).ok())
}

pub(super) fn read_created_canister(
    path: &Path,
) -> Result<Option<Principal>, CreationResultReadError> {
    let bytes = match read_optional_regular_bytes(path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Ok(None),
        Err(RegularFileReadError::NotRegular) => {
            return Err(CreationResultReadError::Unsafe {
                path: path.to_path_buf(),
            });
        }
        Err(RegularFileReadError::Io(source)) => {
            return Err(CreationResultReadError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(CreationResultReadError::UnsupportedPlatform {
                path: path.to_path_buf(),
            });
        }
    };
    if bytes.is_empty() {
        return Ok(None);
    }
    let output = std::str::from_utf8(&bytes).map_err(|_| CreationResultReadError::Invalid {
        path: path.to_path_buf(),
    })?;
    let principal =
        parse_created_canister_id(output).ok_or_else(|| CreationResultReadError::Invalid {
            path: path.to_path_buf(),
        })?;
    Ok(Some(principal))
}

fn parse_canister_id_json(value: &JsonValue) -> Option<Principal> {
    match value {
        JsonValue::String(text) => Principal::from_text(text).ok(),
        JsonValue::Array(values) => values.iter().find_map(parse_canister_id_json),
        JsonValue::Object(object) => ["canister_id", "id", "principal"]
            .iter()
            .filter_map(|key| object.get(*key))
            .find_map(parse_canister_id_json),
        _ => None,
    }
}

#[cfg(test)]
pub(super) fn write_candid_args<T: CandidType>(
    path: &Path,
    args: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    write_bytes(path, &candid::encode_one(args)?)?;
    Ok(())
}

pub(super) fn prepare_creation_result(path: &Path, subject: &str) -> io::Result<()> {
    match create_new_bytes_with_parents(path, &[]) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            match read_optional_regular_bytes(path) {
                Ok(Some(bytes)) if bytes.is_empty() => Ok(()),
                Ok(Some(_)) => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{subject} creation result exists before creation intent"),
                )),
                Ok(None) => Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("{subject} creation result disappeared"),
                )),
                Err(RegularFileReadError::NotRegular) => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{subject} creation result is not a regular file"),
                )),
                Err(RegularFileReadError::Io(source)) => Err(source),
                #[cfg(not(unix))]
                Err(RegularFileReadError::UnsupportedPlatform) => Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!("{subject} creation result reads are unsupported"),
                )),
            }
        }
        Err(source) => Err(source),
    }
}

#[cfg(unix)]
pub(super) fn open_creation_result_for_effect(path: &Path, subject: &str) -> io::Result<File> {
    use rustix::{
        fd::OwnedFd,
        fs::{FileType, Mode, OFlags, fstat, open},
    };

    let bytes = match read_optional_regular_bytes(path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("{subject} creation result is missing"),
            ));
        }
        Err(RegularFileReadError::NotRegular) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{subject} creation result is not a regular file"),
            ));
        }
        Err(RegularFileReadError::Io(source)) => return Err(source),
    };
    if !bytes.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{subject} creation result already contains evidence"),
        ));
    }
    let fd: OwnedFd = open(
        path,
        OFlags::WRONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|source| io::Error::from_raw_os_error(source.raw_os_error()))?;
    let metadata =
        fstat(&fd).map_err(|source| io::Error::from_raw_os_error(source.raw_os_error()))?;
    if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{subject} creation result is not a regular file"),
        ));
    }
    if metadata.st_size != 0 {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{subject} creation result already contains evidence"),
        ));
    }
    Ok(File::from(fd))
}

#[cfg(not(unix))]
pub(super) fn open_creation_result_for_effect(_path: &Path, subject: &str) -> io::Result<File> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        format!("{subject} creation result capture is unsupported"),
    ))
}

pub(super) fn icp_canister_command(icp_root: &Path) -> Command {
    let mut command = icp::default_command_in(icp_root);
    command.arg("canister");
    command
}

pub(super) fn icp_canister_create_command(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    subnet: SubnetId,
    funding: &PlannedCanisterCreationFunding,
    controllers: &[Principal],
) -> Command {
    let mut command = icp_canister_command(icp_root);
    command.args(["create", "--detached", "--json", "--subnet"]);
    command.arg(subnet.to_string());
    match funding {
        PlannedCanisterCreationFunding::Cycles { cycles } => {
            command.args(["--cycles", &cycles.to_string()]);
        }
        PlannedCanisterCreationFunding::Icp { e8s } => {
            command.args(["--with-icp", &icp_e8s_text(*e8s)]);
        }
    }
    for controller in controllers {
        command.args(["--controller", &controller.to_text()]);
    }
    add_icp_environment_target(&mut command, environment, local_replica);
    command
}

pub(super) fn icp_canister_install_binary_args_command(
    icp_root: &Path,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
    canister: Principal,
    wasm_path: &Path,
    args_path: &Path,
) -> Command {
    let mut command = icp_canister_command(icp_root);
    command.args([
        "install",
        &canister.to_text(),
        "--mode=install",
        "-y",
        "--wasm",
    ]);
    command.arg(wasm_path);
    command.arg("--args-file");
    command.arg(args_path);
    command.args(["--args-format", "bin"]);
    add_icp_environment_target(&mut command, environment, local_replica);
    command
}

pub(super) fn add_icp_environment_target(
    command: &mut Command,
    environment: &str,
    local_replica: Option<&LocalReplicaTarget>,
) {
    icp::add_target_args(command, Some(environment), local_replica);
}

pub(super) fn icp_e8s_text(e8s: u64) -> String {
    const E8S_PER_ICP: u64 = 100_000_000;
    let whole = e8s / E8S_PER_ICP;
    let remainder = e8s % E8S_PER_ICP;
    if remainder == 0 {
        return whole.to_string();
    }
    let fractional = format!("{remainder:08}");
    format!("{whole}.{}", fractional.trim_end_matches('0'))
}

//! Module: canister_protocol
//!
//! Responsibility: invoke typed Canic Candid methods through the maintained ICP CLI adapter.
//! Does not own: domain sequencing, endpoint authorization, or management-Canister effects.
//! Boundary: domain workflows supply exact Canister, method, arguments, and query/update intent.

use crate::icp::{IcpCli, IcpCommandError, IcpJsonResponseError, decode_json_result_response};
use candid::{CandidType, Principal};
use canic_core::dto::error::ErrorCode;
use serde::de::DeserializeOwned;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use thiserror::Error as ThisError;

const ICP_JSON_OUTPUT: &str = "json";
const MAX_ARGUMENT_FILE_ATTEMPTS: usize = 32;
static NEXT_ARGUMENT_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, ThisError)]
pub enum CanisterProtocolError {
    #[error("failed to encode Candid arguments for {method} on Canister {canister}: {source}")]
    ArgumentEncoding {
        canister: Principal,
        method: &'static str,
        #[source]
        source: candid::Error,
    },

    #[error(
        "failed to manage binary Candid arguments for {method} on Canister {canister}: {source}"
    )]
    ArgumentFile {
        canister: Principal,
        method: &'static str,
        #[source]
        source: io::Error,
    },

    #[error("failed to invoke {method} on Canister {canister}: {source}")]
    Invocation {
        canister: Principal,
        method: &'static str,
        #[source]
        source: IcpCommandError,
    },

    #[error("invalid {method} response from Canister {canister}: {source}")]
    Response {
        canister: Principal,
        method: &'static str,
        #[source]
        source: IcpJsonResponseError,
    },
}

impl CanisterProtocolError {
    pub(crate) fn is_rejected_with(&self, code: ErrorCode) -> bool {
        matches!(
            self,
            Self::Response {
                source: IcpJsonResponseError::Rejected(error),
                ..
            } if error.code == code
        )
    }
}

pub fn call_no_arg<O>(
    icp: &IcpCli,
    canister: Principal,
    method: &'static str,
) -> Result<O, CanisterProtocolError>
where
    O: CandidType + DeserializeOwned,
{
    let output = icp
        .canister_call_arg_output_with_candid(
            &canister.to_text(),
            method,
            "()",
            Some(ICP_JSON_OUTPUT),
            None,
        )
        .map_err(|source| CanisterProtocolError::Invocation {
            canister,
            method,
            source,
        })?;
    decode_response(canister, method, &output)
}

pub fn call_with_arg<I, O>(
    icp: &IcpCli,
    canister: Principal,
    method: &'static str,
    input: &I,
    query: bool,
) -> Result<O, CanisterProtocolError>
where
    I: CandidType,
    O: CandidType + DeserializeOwned,
{
    let bytes =
        candid::encode_one(input).map_err(|source| CanisterProtocolError::ArgumentEncoding {
            canister,
            method,
            source,
        })?;
    let args_path =
        write_argument_file(&bytes).map_err(|source| CanisterProtocolError::ArgumentFile {
            canister,
            method,
            source,
        })?;
    let output = if query {
        icp.canister_query_binary_args_output_with_candid(
            &canister.to_text(),
            method,
            &args_path,
            Some(ICP_JSON_OUTPUT),
            None,
        )
    } else {
        icp.canister_call_binary_args_output_with_candid(
            &canister.to_text(),
            method,
            &args_path,
            Some(ICP_JSON_OUTPUT),
            None,
        )
    };
    let cleanup = fs::remove_file(&args_path);
    let output = output.map_err(|source| CanisterProtocolError::Invocation {
        canister,
        method,
        source,
    })?;
    cleanup.map_err(|source| CanisterProtocolError::ArgumentFile {
        canister,
        method,
        source,
    })?;
    decode_response(canister, method, &output)
}

fn write_argument_file(bytes: &[u8]) -> io::Result<PathBuf> {
    let directory = std::env::temp_dir();
    for _ in 0..MAX_ARGUMENT_FILE_ATTEMPTS {
        let sequence = NEXT_ARGUMENT_FILE.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!(
            "canic-candid-args-{}-{sequence}.bin",
            std::process::id()
        ));
        match create_argument_file(&path, bytes) {
            Ok(()) => return Ok(path),
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
            Err(source) => return Err(source),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique Candid argument file",
    ))
}

fn create_argument_file(path: &std::path::Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path)?;
    if let Err(source) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(source);
    }
    Ok(())
}

pub fn query_no_arg<O>(
    icp: &IcpCli,
    canister: Principal,
    method: &'static str,
) -> Result<O, CanisterProtocolError>
where
    O: CandidType + DeserializeOwned,
{
    let output = icp
        .canister_query_output_with_candid(&canister.to_text(), method, Some(ICP_JSON_OUTPUT), None)
        .map_err(|source| CanisterProtocolError::Invocation {
            canister,
            method,
            source,
        })?;
    decode_response(canister, method, &output)
}

fn decode_response<O>(
    canister: Principal,
    method: &'static str,
    output: &str,
) -> Result<O, CanisterProtocolError>
where
    O: CandidType + DeserializeOwned,
{
    decode_json_result_response(output).map_err(|source| CanisterProtocolError::Response {
        canister,
        method,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(CandidType, Debug, Deserialize, Eq, PartialEq)]
    struct EmptyVectorArgument {
        values: Vec<u64>,
    }

    #[test]
    fn typed_arguments_are_written_to_private_binary_files() {
        let argument = EmptyVectorArgument { values: Vec::new() };
        let path = write_argument_file(&candid::encode_one(&argument).expect("encode argument"))
            .expect("write argument file");

        let decoded: EmptyVectorArgument =
            candid::decode_one(&fs::read(&path).expect("read argument file"))
                .expect("decode argument");
        assert_eq!(decoded, argument);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(
                fs::metadata(&path)
                    .expect("argument metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        fs::remove_file(path).expect("remove argument file");
    }
}

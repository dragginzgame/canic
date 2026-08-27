//! Module: icp::candid
//!
//! Responsibility: perform one typed Candid update through the maintained ICP CLI process.
//! Does not own: domain retry policy, authority, or durable effect intent.
//! Boundary: callers persist authority before invoking this mechanical transport adapter.

use crate::icp::{IcpCli, IcpCommandError, IcpJsonResponseError};
use candid::CandidType;
use serde::de::DeserializeOwned;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use thiserror::Error as ThisError;

const MAX_ARGUMENT_FILE_ATTEMPTS: usize = 32;
static NEXT_ARGUMENT_FILE: AtomicU64 = AtomicU64::new(0);

/// Typed transport or codec failure for a raw Candid update.

#[derive(Debug, ThisError)]
pub enum IcpCandidCallError {
    #[error("failed to encode Candid call argument: {0}")]
    Encode(#[source] candid::Error),

    #[error("failed to manage Candid call argument file: {0}")]
    File(#[source] io::Error),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error("failed to decode Candid call response: {0}")]
    Response(#[source] IcpJsonResponseError),
}

impl IcpCli {
    /// Perform one typed Candid update and return its exact decoded response.
    pub fn canister_call_candid<I, O>(
        &self,
        canister: &str,
        method: &str,
        input: &I,
        candid_path: Option<&Path>,
    ) -> Result<O, IcpCandidCallError>
    where
        I: CandidType,
        O: CandidType + DeserializeOwned,
    {
        self.canister_candid(canister, method, input, candid_path, false)
    }

    /// Perform one typed Candid query and return its exact decoded response.
    pub fn canister_query_candid<I, O>(
        &self,
        canister: &str,
        method: &str,
        input: &I,
        candid_path: Option<&Path>,
    ) -> Result<O, IcpCandidCallError>
    where
        I: CandidType,
        O: CandidType + DeserializeOwned,
    {
        self.canister_candid(canister, method, input, candid_path, true)
    }

    fn canister_candid<I, O>(
        &self,
        canister: &str,
        method: &str,
        input: &I,
        candid_path: Option<&Path>,
        query: bool,
    ) -> Result<O, IcpCandidCallError>
    where
        I: CandidType,
        O: CandidType + DeserializeOwned,
    {
        let bytes = candid::encode_one(input).map_err(IcpCandidCallError::Encode)?;
        let path = write_argument_file(&bytes).map_err(IcpCandidCallError::File)?;
        let output = if query {
            self.canister_query_binary_args_output_with_candid(
                canister,
                method,
                &path,
                Some("json"),
                candid_path,
            )
        } else {
            self.canister_call_binary_args_output_with_candid(
                canister,
                method,
                &path,
                Some("json"),
                candid_path,
            )
        };
        let cleanup = fs::remove_file(&path);
        let output = output?;
        cleanup.map_err(IcpCandidCallError::File)?;
        crate::icp::response::decode_json_response(&output).map_err(IcpCandidCallError::Response)
    }
}

fn write_argument_file(bytes: &[u8]) -> io::Result<PathBuf> {
    for _ in 0..MAX_ARGUMENT_FILE_ATTEMPTS {
        let sequence = NEXT_ARGUMENT_FILE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "canic-raw-candid-{}-{sequence}.bin",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        match options.open(&path) {
            Ok(mut file) => {
                if let Err(source) = file.write_all(bytes).and_then(|()| file.sync_all()) {
                    drop(file);
                    let _ = fs::remove_file(&path);
                    return Err(source);
                }
                return Ok(path);
            }
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
            Err(source) => return Err(source),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique Candid argument file",
    ))
}

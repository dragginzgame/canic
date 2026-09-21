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
        let path = write_candid_argument_file(&bytes).map_err(IcpCandidCallError::File)?;
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

/// Write private invocation scratch, closing it before the child opens it.
///
/// Completed writes provide process visibility without a durable disk flush.
/// Recovery recreates these arguments from caller-owned durable intent.
pub fn write_candid_argument_file(bytes: &[u8]) -> io::Result<PathBuf> {
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
                if let Err(source) = file.write_all(bytes) {
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use canic_core::cdk::utils::hash::hex_bytes;
    use std::{collections::BTreeSet, os::unix::fs::PermissionsExt};

    #[test]
    fn child_reads_complete_arguments_and_transport_always_removes_scratch() {
        let root = crate::test_support::temp_dir("candid-child-arguments");
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("icp");
        fs::write(
            &executable,
            crate::test_support::tool_script(
                r#"#!/bin/sh
if [ "$1" = --version ]; then echo 'icp @ICP_VERSION@'; exit 0; fi
query=false
while [ "$#" -gt 0 ]; do
  case "$1" in
    --args-file) shift; argument=$1 ;;
    --query) query=true ;;
  esac
  shift
done
printf '%s\n' "$argument" > argument-path
printf '%s\n' "$query" > query-mode
cp "$argument" received || exit 91
cat response
exit "$(cat exit-code)"
"#,
            ),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let icp = IcpCli::new(executable.to_str().unwrap(), None).with_cwd(&root);
        let payload = vec![0xa5_u8; 1024 * 1024];
        let encoded = candid::encode_one(&payload).unwrap();
        let response = serde_json::json!({
            "response_bytes": hex_bytes(candid::encode_one(42_u64).unwrap()),
        });
        let mut paths = BTreeSet::new();
        for query in [false, true] {
            for outcome in 0..3 {
                fs::write(root.join("exit-code"), if outcome == 1 { "1" } else { "0" }).unwrap();
                fs::write(
                    root.join("response"),
                    if outcome == 2 {
                        "{}".to_string()
                    } else {
                        response.to_string()
                    },
                )
                .unwrap();
                let result =
                    icp.canister_candid::<_, u64>("aaaaa-aa", "probe", &payload, None, query);
                match outcome {
                    0 => assert_eq!(result.unwrap(), 42),
                    1 => assert!(matches!(result, Err(IcpCandidCallError::Icp(_)))),
                    _ => assert!(matches!(result, Err(IcpCandidCallError::Response(_)))),
                }
                assert_eq!(fs::read(root.join("received")).unwrap(), encoded);
                assert_eq!(
                    fs::read_to_string(root.join("query-mode")).unwrap().trim(),
                    query.to_string()
                );
                let path = PathBuf::from(
                    fs::read_to_string(root.join("argument-path"))
                        .unwrap()
                        .trim(),
                );
                assert!(!path.exists());
                assert!(paths.insert(path));
            }
        }
        fs::remove_dir_all(root).unwrap();
    }
}

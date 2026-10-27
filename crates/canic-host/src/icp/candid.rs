//! Module: icp::candid
//!
//! Responsibility: perform one typed Candid update through the maintained ICP CLI process.
//! Does not own: domain retry policy, authority, or durable effect intent.
//! Boundary: callers persist authority before invoking this mechanical transport adapter.

use super::binary_arguments::{cleanup_binary_argument_file, write_binary_argument_file};
use crate::icp::{IcpCli, IcpCommandError, IcpJsonResponseError};
use ic_platform::candid::contract::CandidType;
use serde::de::DeserializeOwned;
use std::{io, path::Path};
use thiserror::Error as ThisError;

/// Typed transport or codec failure for a raw Candid update.

#[derive(Debug, ThisError)]
pub enum IcpCandidCallError {
    #[error("failed to encode Candid call argument: {0}")]
    Encode(#[source] ic_platform::candid::contract::Error),

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
        let bytes =
            ic_platform::candid::contract::encode_one(input).map_err(IcpCandidCallError::Encode)?;
        let path = write_binary_argument_file(&bytes).map_err(IcpCandidCallError::File)?;
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
        cleanup_binary_argument_file(&path);
        let output = output?;
        crate::icp::response::decode_json_response(&output).map_err(IcpCandidCallError::Response)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use canic_core::cdk::utils::hash::hex_bytes;
    use std::{collections::BTreeSet, fs, os::unix::fs::PermissionsExt, path::PathBuf};

    #[test]
    fn child_reads_complete_arguments_and_cleanup_preserves_transport_outcomes() {
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
case "$(cat cleanup-mode)" in
  missing) rm "$argument" ;;
  directory) rm "$argument"; mkdir "$argument" ;;
esac
cat response
exit "$(cat exit-code)"
"#,
            ),
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let icp = IcpCli::new(executable.to_str().unwrap(), None).with_cwd(&root);
        let payload = vec![0xa5_u8; 1024 * 1024];
        let encoded = ic_platform::candid::contract::encode_one(&payload).unwrap();
        let response = serde_json::json!({
            "response_bytes": hex_bytes(ic_platform::candid::contract::encode_one(42_u64).unwrap()),
        });
        let mut paths = BTreeSet::new();
        for (query, cleanup_mode) in [
            (false, "normal"),
            (true, "normal"),
            (false, "missing"),
            (true, "missing"),
            (false, "directory"),
            (true, "directory"),
        ] {
            fs::write(root.join("cleanup-mode"), cleanup_mode).unwrap();
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
                if cleanup_mode == "directory" {
                    assert!(path.is_dir());
                    fs::remove_dir(&path).unwrap();
                } else {
                    assert!(!path.exists());
                }
                assert!(paths.insert(path));
            }
        }
        fs::remove_dir_all(root).unwrap();
    }
}

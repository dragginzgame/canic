//! Module: install_root::operations::installation
//!
//! Responsibility: execute or observe one journal-authorized initial Canister install effect.
//! Does not own: role init authority or journal transitions.
//! Boundary: callers supply typed init authority and commit the reconciled module hash.

use crate::{
    durable_io::{
        RegularFileReadError, create_new_bytes_with_parents, read_optional_regular_bytes,
        write_bytes,
    },
    icp::{self, IcpCommandError, IcpDiagnostic},
    install_root::{
        commands::icp_canister_install_binary_args_command, icp_context::InstallIcpContext,
    },
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

use super::{EffectAction, activation::observe_module_hash, module_hash_text};

const INSTALL_REJECTION_RECEIPT_SCHEMA_VERSION: u32 = 1;
const MAX_INSTALL_REJECTION_RECEIPT_BYTES: usize = 4 * 1024;

#[derive(Debug, ThisError)]
#[error("{subject} install reconciliation for {canister} failed: {detail}")]
struct InstallEffectError {
    subject: &'static str,
    canister: Principal,
    detail: String,
}

/// Exact authority for retrying one install the replica explicitly rejected before application.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct InstallRejectionReceipt {
    schema_version: u32,
    fresh_fleet_plan_digest: String,
    canister: Principal,
    expected_module_hash: [u8; 32],
    args_sha256: [u8; 32],
    rejection: InstallTerminalRejection,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum InstallTerminalRejection {
    OutOfCycles,
}

#[derive(Debug, ThisError)]
enum InstallRejectionReceiptError {
    #[error("install rejection receipt is not a regular no-follow file: {path}")]
    Unsafe { path: PathBuf },

    #[error("invalid install rejection receipt {path}: {reason}")]
    Invalid { path: PathBuf, reason: String },

    #[error("install rejection receipt conflicts with current install authority: {path}")]
    Conflict { path: PathBuf },

    #[error("failed to access install rejection receipt {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[cfg(not(unix))]
    #[error("install rejection receipt reads are unsupported: {path}")]
    UnsupportedPlatform { path: PathBuf },
}

pub(in crate::install_root) struct InstallEffectRequest<'a> {
    pub icp: &'a InstallIcpContext,
    pub subject: &'static str,
    pub canister: Principal,
    pub wasm_path: &'a Path,
    pub args_path: &'a Path,
    pub expected_module_hash: [u8; 32],
    pub fresh_fleet_plan_digest: &'a str,
    pub action: EffectAction,
}

pub(in crate::install_root) fn execute_or_observe_install<T, F>(
    request: InstallEffectRequest<'_>,
    resolve_args: F,
) -> Result<[u8; 32], Box<dyn std::error::Error>>
where
    T: CandidType,
    F: FnOnce() -> Result<T, Box<dyn std::error::Error>>,
{
    let icp = request.icp.cli();
    let prior_rejection = match observe_module_hash(icp, request.canister)? {
        Some(module_hash) if module_hash == request.expected_module_hash => {
            return Ok(module_hash);
        }
        Some(module_hash) => {
            return Err(effect_error(
                &request,
                format!("unexpected module hash {}", module_hash_text(module_hash)),
            ));
        }
        None if matches!(request.action, EffectAction::ObserveOnly) => {
            let receipt_path = install_rejection_receipt_path(request.args_path);
            let Some(receipt) = load_optional_install_rejection_receipt(&receipt_path)? else {
                return Err(effect_error(
                    &request,
                    "outcome is unknown; no second install was attempted because the journal is already install_in_flight and the expected module is not observable"
                        .to_string(),
                ));
            };
            Some((receipt_path, receipt))
        }
        None => None,
    };

    let args = candid::encode_one(resolve_args()?)?;
    let expected_rejection = InstallRejectionReceipt {
        schema_version: INSTALL_REJECTION_RECEIPT_SCHEMA_VERSION,
        fresh_fleet_plan_digest: request.fresh_fleet_plan_digest.to_string(),
        canister: request.canister,
        expected_module_hash: request.expected_module_hash,
        args_sha256: Sha256::digest(&args).into(),
        rejection: InstallTerminalRejection::OutOfCycles,
    };
    if let Some((receipt_path, receipt)) = prior_rejection
        && receipt != expected_rejection
    {
        return Err(InstallRejectionReceiptError::Conflict { path: receipt_path }.into());
    }
    write_bytes(request.args_path, &args)?;
    let mut command = icp_canister_install_binary_args_command(
        request.icp,
        request.canister,
        request.wasm_path,
        request.args_path,
    );
    let command_result = icp::run_status(&mut command);
    match observe_module_hash(icp, request.canister) {
        Ok(Some(module_hash)) if module_hash == request.expected_module_hash => Ok(module_hash),
        Ok(Some(module_hash)) => Err(effect_error(
            &request,
            format!("unexpected module hash {}", module_hash_text(module_hash)),
        )),
        Ok(None) => {
            if persist_known_rejection(&request, &expected_rejection, &command_result)? {
                return Err(explicit_rejection_error(&request, None));
            }
            Err(effect_error(
                &request,
                format!(
                    "outcome is unknown; no second install was attempted: {}",
                    command_result.err().map_or_else(
                        || "install command completed but no module is observable".to_string(),
                        |error| error.to_string(),
                    )
                ),
            ))
        }
        Err(observation) => {
            if persist_known_rejection(&request, &expected_rejection, &command_result)? {
                return Err(explicit_rejection_error(
                    &request,
                    Some(observation.to_string()),
                ));
            }
            Err(effect_error(
                &request,
                format!(
                    "outcome is unknown; no second install was attempted: {}",
                    match command_result {
                        Ok(()) => format!("post-install observation failed: {observation}"),
                        Err(command) => {
                            format!(
                                "install command failed: {command}; reconciliation failed: {observation}"
                            )
                        }
                    }
                ),
            ))
        }
    }
}

fn persist_known_rejection(
    request: &InstallEffectRequest<'_>,
    receipt: &InstallRejectionReceipt,
    command_result: &Result<(), IcpCommandError>,
) -> Result<bool, InstallRejectionReceiptError> {
    let Err(command_error) = command_result else {
        return Ok(false);
    };
    if !matches!(
        command_error.diagnostic(),
        Some(IcpDiagnostic::CanisterOutOfCycles)
    ) {
        return Ok(false);
    }
    persist_install_rejection_receipt(&install_rejection_receipt_path(request.args_path), receipt)?;
    Ok(true)
}

fn explicit_rejection_error(
    request: &InstallEffectRequest<'_>,
    observation_error: Option<String>,
) -> Box<dyn std::error::Error> {
    let observation = observation_error.map_or_else(String::new, |error| {
        format!("; post-rejection observation also failed: {error}")
    });
    effect_error(
        request,
        format!(
            "the replica explicitly rejected the install with IC0207 before application; the exact rejection receipt is durable, so top up the Canister and retry{observation}"
        ),
    )
}

fn install_rejection_receipt_path(args_path: &Path) -> PathBuf {
    let mut path = args_path.as_os_str().to_os_string();
    path.push(".known-rejection.json");
    PathBuf::from(path)
}

fn load_optional_install_rejection_receipt(
    path: &Path,
) -> Result<Option<InstallRejectionReceipt>, InstallRejectionReceiptError> {
    let bytes = match read_optional_regular_bytes(path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Ok(None),
        Err(RegularFileReadError::NotRegular) => {
            return Err(InstallRejectionReceiptError::Unsafe {
                path: path.to_path_buf(),
            });
        }
        Err(RegularFileReadError::Io(source)) => {
            return Err(InstallRejectionReceiptError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => {
            return Err(InstallRejectionReceiptError::UnsupportedPlatform {
                path: path.to_path_buf(),
            });
        }
    };
    if bytes.len() > MAX_INSTALL_REJECTION_RECEIPT_BYTES {
        return Err(invalid_receipt(path, "receipt exceeds its byte bound"));
    }
    let receipt: InstallRejectionReceipt = serde_json::from_slice(&bytes)
        .map_err(|error| invalid_receipt(path, &error.to_string()))?;
    validate_install_rejection_receipt(path, &receipt)?;
    let canonical = encode_install_rejection_receipt(path, &receipt)?;
    if canonical != bytes {
        return Err(invalid_receipt(path, "receipt bytes are not canonical"));
    }
    Ok(Some(receipt))
}

fn persist_install_rejection_receipt(
    path: &Path,
    receipt: &InstallRejectionReceipt,
) -> Result<(), InstallRejectionReceiptError> {
    let bytes = encode_install_rejection_receipt(path, receipt)?;
    match create_new_bytes_with_parents(path, &bytes) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            match load_optional_install_rejection_receipt(path)? {
                Some(observed) if observed == *receipt => Ok(()),
                Some(_) => Err(InstallRejectionReceiptError::Conflict {
                    path: path.to_path_buf(),
                }),
                None => Err(InstallRejectionReceiptError::Io {
                    path: path.to_path_buf(),
                    source: io::Error::new(
                        io::ErrorKind::NotFound,
                        "install rejection receipt disappeared",
                    ),
                }),
            }
        }
        Err(source) => Err(InstallRejectionReceiptError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn encode_install_rejection_receipt(
    path: &Path,
    receipt: &InstallRejectionReceipt,
) -> Result<Vec<u8>, InstallRejectionReceiptError> {
    validate_install_rejection_receipt(path, receipt)?;
    let bytes =
        serde_json::to_vec(receipt).map_err(|error| invalid_receipt(path, &error.to_string()))?;
    if bytes.len() > MAX_INSTALL_REJECTION_RECEIPT_BYTES {
        return Err(invalid_receipt(path, "receipt exceeds its byte bound"));
    }
    Ok(bytes)
}

fn validate_install_rejection_receipt(
    path: &Path,
    receipt: &InstallRejectionReceipt,
) -> Result<(), InstallRejectionReceiptError> {
    if receipt.schema_version != INSTALL_REJECTION_RECEIPT_SCHEMA_VERSION {
        return Err(invalid_receipt(path, "unsupported schema version"));
    }
    if receipt.canister == Principal::anonymous() {
        return Err(invalid_receipt(path, "Canister principal is anonymous"));
    }
    if !is_canonical_sha256(&receipt.fresh_fleet_plan_digest) {
        return Err(invalid_receipt(
            path,
            "fresh-Fleet plan digest is not canonical SHA-256 text",
        ));
    }
    Ok(())
}

fn is_canonical_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn invalid_receipt(path: &Path, reason: &str) -> InstallRejectionReceiptError {
    InstallRejectionReceiptError::Invalid {
        path: path.to_path_buf(),
        reason: reason.to_string(),
    }
}

fn effect_error(request: &InstallEffectRequest<'_>, detail: String) -> Box<dyn std::error::Error> {
    InstallEffectError {
        subject: request.subject,
        canister: request.canister,
        detail,
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    fn receipt(args_sha256: [u8; 32]) -> InstallRejectionReceipt {
        InstallRejectionReceipt {
            schema_version: INSTALL_REJECTION_RECEIPT_SCHEMA_VERSION,
            fresh_fleet_plan_digest: "ab".repeat(32),
            canister: Principal::from_text("rrkah-fqaaa-aaaaa-aaaaq-cai")
                .expect("Canister principal"),
            expected_module_hash: [7; 32],
            args_sha256,
            rejection: InstallTerminalRejection::OutOfCycles,
        }
    }

    #[test]
    fn known_rejection_receipt_is_exact_and_idempotent() {
        let root = temp_dir("canic-install-known-rejection");
        let path = root.join("coordinator-install-args.bin.known-rejection.json");
        let expected = receipt([9; 32]);

        persist_install_rejection_receipt(&path, &expected).expect("persist rejection receipt");
        persist_install_rejection_receipt(&path, &expected).expect("persist exact retry");

        assert_eq!(
            load_optional_install_rejection_receipt(&path).expect("load rejection receipt"),
            Some(expected)
        );
        std::fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn known_rejection_receipt_rejects_changed_install_authority() {
        let root = temp_dir("canic-install-known-rejection-conflict");
        let path = root.join("root-install-args.bin.known-rejection.json");
        persist_install_rejection_receipt(&path, &receipt([1; 32]))
            .expect("persist rejection receipt");

        let error = persist_install_rejection_receipt(&path, &receipt([2; 32]))
            .expect_err("changed authority rejected");

        assert!(matches!(
            error,
            InstallRejectionReceiptError::Conflict { .. }
        ));
        let mut changed_plan = receipt([1; 32]);
        changed_plan.fresh_fleet_plan_digest = "cd".repeat(32);
        assert!(matches!(
            persist_install_rejection_receipt(&path, &changed_plan),
            Err(InstallRejectionReceiptError::Conflict { .. })
        ));
        std::fs::remove_dir_all(root).expect("remove temp root");
    }

    #[test]
    fn known_rejection_receipt_rejects_noncanonical_bytes() {
        let root = temp_dir("canic-install-known-rejection-noncanonical");
        let path = root.join("wasm-store-install-args.bin.known-rejection.json");
        let mut bytes =
            encode_install_rejection_receipt(&path, &receipt([3; 32])).expect("encode receipt");
        bytes.push(b'\n');
        std::fs::create_dir_all(&root).expect("create temp root");
        std::fs::write(&path, bytes).expect("write noncanonical receipt");

        let error = load_optional_install_rejection_receipt(&path)
            .expect_err("noncanonical receipt rejected");

        assert!(matches!(
            error,
            InstallRejectionReceiptError::Invalid { .. }
        ));
        std::fs::remove_dir_all(root).expect("remove temp root");
    }
}

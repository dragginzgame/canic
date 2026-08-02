//! Module: install_root::operations::activation
//!
//! Responsibility: parse and render exact installed Canister module hashes.
//! Does not own: install commands, activation transitions, or Fleet authority.
//! Boundary: journalled Coordinator and Fleet Subnet Root workflows share this strict projection.

use crate::icp::{IcpCli, IcpCommandError};
use candid::Principal;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub(in crate::install_root) enum ModuleHashObservationError {
    #[error("failed to query status for Canister {canister}: {source}")]
    Invocation {
        canister: Principal,
        #[source]
        source: IcpCommandError,
    },

    #[error("Canister status returned {observed} for expected principal {expected}")]
    PrincipalMismatch {
        expected: Principal,
        observed: String,
    },

    #[error("Canister {canister} status returned invalid module hash {observed}")]
    InvalidHash {
        canister: Principal,
        observed: String,
    },
}

#[derive(Debug, ThisError)]
pub(in crate::install_root) enum CreatedCanisterStateError {
    #[error("{subject} {canister} already has its expected module before install intent")]
    ExpectedModulePresent {
        subject: &'static str,
        canister: Principal,
    },

    #[error("{subject} {canister} already has unexpected module hash {observed}")]
    UnexpectedModule {
        subject: &'static str,
        canister: Principal,
        observed: String,
    },

    #[error(transparent)]
    Observation(#[from] ModuleHashObservationError),
}

pub(in crate::install_root) fn observe_module_hash(
    icp: &IcpCli,
    canister: Principal,
) -> Result<Option<[u8; 32]>, ModuleHashObservationError> {
    let report = icp
        .canister_status_report(&canister.to_text())
        .map_err(|source| ModuleHashObservationError::Invocation { canister, source })?;
    if report.id != canister.to_text() {
        return Err(ModuleHashObservationError::PrincipalMismatch {
            expected: canister,
            observed: report.id,
        });
    }
    report
        .module_hash
        .as_deref()
        .map(|value| {
            parse_module_hash(value).ok_or_else(|| ModuleHashObservationError::InvalidHash {
                canister,
                observed: value.to_string(),
            })
        })
        .transpose()
}

pub(in crate::install_root) fn require_uninstalled_created_canister(
    icp: &IcpCli,
    canister: Principal,
    expected_module_hash: [u8; 32],
    subject: &'static str,
) -> Result<(), CreatedCanisterStateError> {
    match observe_module_hash(icp, canister)? {
        None => Ok(()),
        Some(observed) if observed == expected_module_hash => {
            Err(CreatedCanisterStateError::ExpectedModulePresent { subject, canister })
        }
        Some(observed) => Err(CreatedCanisterStateError::UnexpectedModule {
            subject,
            canister,
            observed: module_hash_text(observed),
        }),
    }
}

fn parse_module_hash(value: &str) -> Option<[u8; 32]> {
    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if value.len() != 64 {
        return None;
    }
    let mut bytes = [0; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Some(bytes)
}

const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(in crate::install_root) fn module_hash_text(bytes: [u8; 32]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(64), |mut text, byte| {
            use std::fmt::Write as _;
            let _ = write!(text, "{byte:02x}");
            text
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_hash_projection_is_exact_and_prefix_tolerant() {
        let bytes = [0xab; 32];
        let text = "ab".repeat(32);

        assert_eq!(module_hash_text(bytes), text);
        assert_eq!(parse_module_hash(&text), Some(bytes));
        assert_eq!(parse_module_hash(&format!("0x{text}")), Some(bytes));
        assert_eq!(parse_module_hash("not-a-module"), None);
    }
}

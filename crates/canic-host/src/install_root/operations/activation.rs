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
pub(in crate::install_root) enum CanisterControllerObservationError {
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

    #[error("Canister {canister} status omitted controller settings")]
    MissingSettings { canister: Principal },

    #[error("Canister {canister} returned invalid installation controller {controller}")]
    InvalidController {
        canister: Principal,
        controller: String,
    },
}

#[derive(Debug, ThisError)]
pub(in crate::install_root) enum InstallationControllerObservationError {
    #[error(
        "failed to resolve the active ICP installation identity: {source}; for an encrypted identity in non-interactive execution, set CANIC_ICP_IDENTITY_PASSWORD_FILE to an absolute operator-owned password file"
    )]
    Invocation {
        #[source]
        source: IcpCommandError,
    },

    #[error("ICP CLI returned invalid installation identity {observed:?}")]
    InvalidPrincipal { observed: String },

    #[error("ICP CLI returned the anonymous Principal as the installation identity")]
    Anonymous,

    #[error(
        "active ICP installation identity {observed} differs from Fleet input operator principal {expected}"
    )]
    #[cfg(test)]
    OperatorMismatch {
        expected: String,
        observed: Principal,
    },
}

#[derive(Debug, ThisError)]
pub(in crate::install_root) enum ExpectedCanisterControllersError {
    #[error("{subject} creation authority must contain at least one controller")]
    Empty { subject: &'static str },

    #[error("{subject} creation authority contains the anonymous controller")]
    Anonymous { subject: &'static str },

    #[error(transparent)]
    Observation(#[from] CanisterControllerObservationError),

    #[error(
        "{subject} controllers differ from exact creation authority: expected {expected:?}, observed {observed:?}"
    )]
    Mismatch {
        subject: &'static str,
        expected: Vec<Principal>,
        observed: Vec<Principal>,
    },
}

#[derive(Debug, ThisError)]
pub(in crate::install_root) enum CanisterModuleStateError {
    #[error("{subject} {canister} already has its expected module before install intent")]
    ExpectedModulePresent {
        subject: &'static str,
        canister: Principal,
    },

    #[error("{subject} {canister} has no installed module")]
    MissingModule {
        subject: &'static str,
        canister: Principal,
    },

    #[error(transparent)]
    Observation(#[from] ModuleHashObservationError),

    #[error("{subject} {canister} already has unexpected module hash {observed}")]
    UnexpectedModule {
        subject: &'static str,
        canister: Principal,
        observed: String,
    },
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

pub(in crate::install_root) fn observe_controllers(
    icp: &IcpCli,
    canister: Principal,
) -> Result<Vec<Principal>, CanisterControllerObservationError> {
    let report = icp
        .canister_status_report(&canister.to_text())
        .map_err(|source| CanisterControllerObservationError::Invocation { canister, source })?;
    if report.id != canister.to_text() {
        return Err(CanisterControllerObservationError::PrincipalMismatch {
            expected: canister,
            observed: report.id,
        });
    }
    let settings = report
        .settings
        .ok_or(CanisterControllerObservationError::MissingSettings { canister })?;
    let mut controllers = Vec::with_capacity(settings.controllers.len());
    for text in settings.controllers {
        let controller = Principal::from_text(&text).map_err(|_| {
            CanisterControllerObservationError::InvalidController {
                canister,
                controller: text.clone(),
            }
        })?;
        if controller == Principal::anonymous() {
            return Err(CanisterControllerObservationError::InvalidController {
                canister,
                controller: text,
            });
        }
        controllers.push(controller);
    }
    controllers.sort();
    controllers.dedup();
    Ok(controllers)
}

pub(in crate::install_root) fn active_installation_controller(
    icp: &IcpCli,
) -> Result<Principal, InstallationControllerObservationError> {
    let observed = icp
        .identity_principal_text()
        .map_err(|source| InstallationControllerObservationError::Invocation { source })?;
    let controller = Principal::from_text(observed.trim()).map_err(|_| {
        InstallationControllerObservationError::InvalidPrincipal {
            observed: observed.clone(),
        }
    })?;
    if controller == Principal::anonymous() {
        return Err(InstallationControllerObservationError::Anonymous);
    }
    Ok(controller)
}

#[cfg(test)]
pub(in crate::install_root) fn require_planned_installation_controller(
    icp: &IcpCli,
    expected: &str,
) -> Result<Principal, InstallationControllerObservationError> {
    let observed = active_installation_controller(icp)?;
    if observed.to_text() != expected {
        return Err(InstallationControllerObservationError::OperatorMismatch {
            expected: expected.to_string(),
            observed,
        });
    }
    Ok(observed)
}

pub(in crate::install_root) fn require_expected_controllers(
    icp: &IcpCli,
    canister: Principal,
    expected: &[Principal],
    subject: &'static str,
) -> Result<(), ExpectedCanisterControllersError> {
    if expected.is_empty() {
        return Err(ExpectedCanisterControllersError::Empty { subject });
    }
    if expected.contains(&Principal::anonymous()) {
        return Err(ExpectedCanisterControllersError::Anonymous { subject });
    }
    let mut expected = expected.to_vec();
    expected.sort();
    expected.dedup();
    let observed = observe_controllers(icp, canister)?;
    if observed != expected {
        return Err(ExpectedCanisterControllersError::Mismatch {
            subject,
            expected,
            observed,
        });
    }
    Ok(())
}

pub(in crate::install_root) fn require_uninstalled_created_canister(
    icp: &IcpCli,
    canister: Principal,
    expected_module_hash: [u8; 32],
    subject: &'static str,
) -> Result<(), CanisterModuleStateError> {
    match observe_module_hash(icp, canister)? {
        None => Ok(()),
        Some(observed) if observed == expected_module_hash => {
            Err(CanisterModuleStateError::ExpectedModulePresent { subject, canister })
        }
        Some(observed) => Err(CanisterModuleStateError::UnexpectedModule {
            subject,
            canister,
            observed: module_hash_text(observed),
        }),
    }
}

pub(in crate::install_root) fn require_expected_module_hash(
    icp: &IcpCli,
    canister: Principal,
    expected_module_hash: [u8; 32],
    subject: &'static str,
) -> Result<(), CanisterModuleStateError> {
    match observe_module_hash(icp, canister)? {
        Some(observed) if observed == expected_module_hash => Ok(()),
        Some(observed) => Err(CanisterModuleStateError::UnexpectedModule {
            subject,
            canister,
            observed: module_hash_text(observed),
        }),
        None => Err(CanisterModuleStateError::MissingModule { subject, canister }),
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

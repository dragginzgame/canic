//! Module: install_root::operations::installation
//!
//! Responsibility: execute or observe one journal-authorized initial Canister install effect.
//! Does not own: role init authority or journal transitions.
//! Boundary: callers supply typed init authority and commit the reconciled module hash.

use crate::{
    icp::LocalReplicaTarget,
    install_root::{
        commands::{icp_canister_install_binary_args_command, run_command, write_candid_args},
        install_icp,
    },
};
use candid::{CandidType, Principal};
use std::path::Path;
use thiserror::Error as ThisError;

use super::{EffectAction, activation::observe_module_hash, module_hash_text};

#[derive(Debug, ThisError)]
#[error("{subject} install reconciliation for {canister} failed: {detail}")]
struct InstallEffectError {
    subject: &'static str,
    canister: Principal,
    detail: String,
}

pub(in crate::install_root) struct InstallEffectRequest<'a> {
    pub icp_root: &'a Path,
    pub environment: &'a str,
    pub local_replica: Option<&'a LocalReplicaTarget>,
    pub subject: &'static str,
    pub canister: Principal,
    pub wasm_path: &'a Path,
    pub args_path: &'a Path,
    pub expected_module_hash: [u8; 32],
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
    let icp = install_icp(request.icp_root, request.environment, request.local_replica);
    match observe_module_hash(&icp, request.canister)? {
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
            return Err(effect_error(
                &request,
                "outcome is unknown; no second install was attempted because the journal is already install_in_flight and the expected module is not observable"
                    .to_string(),
            ));
        }
        None => {}
    }

    write_candid_args(request.args_path, &resolve_args()?)?;
    let mut command = icp_canister_install_binary_args_command(
        request.icp_root,
        request.environment,
        request.local_replica,
        request.canister,
        request.wasm_path,
        request.args_path,
    );
    let command_result = run_command(&mut command);
    match observe_module_hash(&icp, request.canister) {
        Ok(Some(module_hash)) if module_hash == request.expected_module_hash => Ok(module_hash),
        Ok(Some(module_hash)) => Err(effect_error(
            &request,
            format!("unexpected module hash {}", module_hash_text(module_hash)),
        )),
        Ok(None) => Err(effect_error(
            &request,
            format!(
                "outcome is unknown; no second install was attempted: {}",
                command_result.err().map_or_else(
                    || "install command completed but no module is observable".to_string(),
                    |error| error.to_string(),
                )
            ),
        )),
        Err(observation) => Err(effect_error(
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
        )),
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

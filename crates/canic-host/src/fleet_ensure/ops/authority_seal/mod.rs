//! Module: fleet_ensure::ops::authority_seal
//!
//! Responsibility: issue and observe the existing authority-snapshot seal.
//! Does not own: reset intent, sequencing, inventory admission or retries.
//! Boundary: one exact controller command or protected status read per call.

pub(in crate::fleet_ensure) mod contract;
#[cfg(test)]
mod tests;

use crate::{
    canister_protocol::{call_with_candid, query_with_candid},
    fleet_ensure::{
        model::EnsureAction,
        ops::{
            EffectObservation, EffectOutcome, EffectRetry, current_protocol::CurrentProtocolError,
        },
    },
    icp::IcpCli,
};
use candid::{CandidType, Principal};
use canic_core::dto::authority_restore::{
    AuthorityRestoreFencePhase, AuthorityRestoreFenceStatusResponse, AuthoritySnapshotRequest,
};
use serde::Deserialize;
use std::path::Path;

#[derive(CandidType)]
enum Command {
    PrepareAuthoritySnapshot(AuthoritySnapshotRequest),
}

#[derive(CandidType, Deserialize)]
enum CommandResponse {
    PrepareAuthoritySnapshot(AuthorityRestoreFenceStatusResponse),
}

#[derive(CandidType)]
enum Status {
    AuthorityRestore,
}

#[derive(CandidType, Deserialize)]
enum StatusResponse {
    AuthorityRestore(AuthorityRestoreFenceStatusResponse),
}

struct Authority<'a> {
    candid: std::path::PathBuf,
    command: &'static str,
    status: &'static str,
    principal: Principal,
    operation: [u8; 32],
    name: &'a str,
}

fn authority<'a>(
    root: &Path,
    operation_id: &str,
    action: &'a EnsureAction,
) -> Result<Authority<'a>, CurrentProtocolError> {
    let EnsureAction::SealAuthority {
        candid,
        candid_sha256,
        authority_kind,
        name,
        principal,
    } = action
    else {
        return Err(CurrentProtocolError::ResponseMismatch);
    };
    let path = root.join(candid);
    let bytes = crate::durable_io::read_regular_bytes(&path, 1024 * 1024)
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    if canic_core::cdk::utils::hash::sha256_hex(&bytes) != *candid_sha256 {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    let invalid_contract = || CurrentProtocolError::AuthoritySealContract { name: name.clone() };
    let text = std::str::from_utf8(&bytes).map_err(|_| invalid_contract())?;
    contract::verify(text, *authority_kind).ok_or_else(invalid_contract)?;
    let (command, status) = contract::methods(*authority_kind).ok_or_else(invalid_contract)?;
    Ok(Authority {
        candid: path,
        command,
        status,
        principal: Principal::from_text(principal)
            .map_err(|_| CurrentProtocolError::ResponseMismatch)?,
        operation: super::current_protocol::operation_bytes(operation_id)?,
        name,
    })
}

pub(in crate::fleet_ensure) fn apply(
    icp: &IcpCli,
    root: &Path,
    operation_id: &str,
    action: &EnsureAction,
) -> Result<EffectOutcome, CurrentProtocolError> {
    let authority = authority(root, operation_id, action)?;
    let CommandResponse::PrepareAuthoritySnapshot(status) = call_with_candid(
        icp,
        &authority.candid,
        authority.principal,
        authority.command,
        &Command::PrepareAuthoritySnapshot(AuthoritySnapshotRequest {
            operation_id: authority.operation,
        }),
    )?;
    validate_status(authority.principal, &status)?;
    if !sealed(&authority, &status) {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    Ok(EffectOutcome {
        created_principal: None,
        post_cycles: None,
        receipt: Some(operation_id.to_string()),
    })
}

pub(in crate::fleet_ensure) fn observe(
    icp: &IcpCli,
    root: &Path,
    operation_id: &str,
    action: &EnsureAction,
) -> Result<EffectObservation, CurrentProtocolError> {
    let authority = authority(root, operation_id, action)?;
    let StatusResponse::AuthorityRestore(status) = query_with_candid(
        icp,
        &authority.candid,
        authority.principal,
        authority.status,
        &Status::AuthorityRestore,
    )?;
    observation(&authority, &status)
}

fn observation(
    authority: &Authority<'_>,
    status: &AuthorityRestoreFenceStatusResponse,
) -> Result<EffectObservation, CurrentProtocolError> {
    validate_status(authority.principal, status)?;
    if status.phase == AuthorityRestoreFencePhase::Sealed
        && status.operation_id != Some(authority.operation)
    {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    Ok(EffectObservation {
        provisioning_progress: None,
        provisioning_failure: None,
        applied: sealed(authority, status),
        estate_funding_required: None,
        post_cycles: None,
        progress_identity: format!("authority-seal:{}:{:?}", authority.name, status.phase),
        retry: EffectRetry::ReplayExactIssuedCommand,
    })
}

/// A fence receipt is complete or absent; partial history cannot reconcile a lost reply.
fn validate_status(
    principal: Principal,
    status: &AuthorityRestoreFenceStatusResponse,
) -> Result<(), CurrentProtocolError> {
    let receipt_fields = [
        status.operation_id.is_some(),
        status.history_total_num_changes.is_some(),
        status.changed_at_ns.is_some(),
    ];
    let complete = receipt_fields.into_iter().all(|present| present);
    let absent = receipt_fields.into_iter().all(|present| !present);
    let valid = match status.phase {
        AuthorityRestoreFencePhase::Open => absent || complete,
        AuthorityRestoreFencePhase::Sealed => complete,
    };
    if principal != status.authority_canister || !valid {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    Ok(())
}

fn sealed(authority: &Authority<'_>, status: &AuthorityRestoreFenceStatusResponse) -> bool {
    status.authority_canister == authority.principal
        && status.phase == AuthorityRestoreFencePhase::Sealed
        && status.operation_id == Some(authority.operation)
}

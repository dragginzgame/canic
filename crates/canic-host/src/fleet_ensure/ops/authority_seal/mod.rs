//! Module: fleet_ensure::ops::authority_seal
//!
//! Responsibility: issue and observe the existing authority-snapshot seal.
//! Does not own: reset intent, sequencing, inventory admission or retries.
//! Boundary: one exact controller command or protected status read per call.

use crate::{
    canister_protocol::{call_with_candid, query_with_candid},
    fleet_ensure::{
        model::{DesiredCanisterKind, EnsureAction},
        ops::{
            EffectObservation, EffectOutcome, EffectRetry, current_protocol::CurrentProtocolError,
        },
    },
    icp::IcpCli,
};
use candid::{CandidType, Principal};
use canic_core::{
    dto::authority_restore::{
        AuthorityRestoreFencePhase, AuthorityRestoreFenceStatusResponse, AuthoritySnapshotRequest,
    },
    protocol,
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
    let bytes = std::fs::read(&path)
        .map_err(|error| CurrentProtocolError::Configuration(error.to_string()))?;
    if canic_core::cdk::utils::hash::sha256_hex(&bytes) != *candid_sha256 {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    let (command, status) = match authority_kind {
        DesiredCanisterKind::Root => (protocol::CANIC_ROOT_COMMAND, protocol::CANIC_ROOT_STATUS),
        DesiredCanisterKind::Coordinator => (
            protocol::CANIC_COORDINATOR_COMMAND,
            protocol::CANIC_OBSERVABILITY,
        ),
        _ => return Err(CurrentProtocolError::ResponseMismatch),
    };
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

pub(super) fn apply(
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
    if !sealed(&authority, &status) {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    Ok(EffectOutcome {
        created_principal: None,
        post_cycles: None,
        receipt: Some(operation_id.to_string()),
    })
}

pub(super) fn observe(
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
    if status.authority_canister != authority.principal
        || (status.phase == AuthorityRestoreFencePhase::Sealed
            && status.operation_id != Some(authority.operation))
    {
        return Err(CurrentProtocolError::ResponseMismatch);
    }
    Ok(EffectObservation {
        provisioning_failure: None,
        applied: sealed(&authority, &status),
        estate_funding_required: None,
        post_cycles: None,
        progress_identity: format!("authority-seal:{}:{:?}", authority.name, status.phase),
        retry: EffectRetry::ReplayExactIssuedCommand,
    })
}

fn sealed(authority: &Authority<'_>, status: &AuthorityRestoreFenceStatusResponse) -> bool {
    status.authority_canister == authority.principal
        && status.phase == AuthorityRestoreFencePhase::Sealed
        && status.operation_id == Some(authority.operation)
}

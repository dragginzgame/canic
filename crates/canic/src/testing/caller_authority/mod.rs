//! Module: testing::caller_authority
//!
//! Protected caller publication for synthetic managed fixtures.
//!
//! Production membership uses Root's journal; fixtures retain exact receipts in their receivers.

use super::managed_app::ManagedAppQualificationError as FixtureError;
use candid::Principal;
use canic_contracts::dto::wire::projection::caller_authority::Command;
use canic_contracts::dto::wire::projection::caller_authority::CommandResponse as Response;
use canic_contracts::dto::wire::projection::caller_authority::StatusRequest;
use canic_contracts::dto::wire::projection::caller_authority::StatusResponse;
use canic_contracts::{
    dto::{caller_authority::*, error::Error, role::OperationStatusRequest},
    ids::CallerReceiverAuthority,
    protocol::{CANIC_COMMAND, CANIC_CONTROL_STATUS},
};
use canic_core::{
    api::caller_authority::CallerAuthorityApi, bootstrap::compiled::CompiledCallerPolicy,
    control_plane_support::policy::caller_authority::matches_permission,
};
use ic_testkit::pic::{CandidCallExt, PocketIc};

pub(super) fn publish(
    pic: &PocketIc,
    root: Principal,
    receivers: &[(CallerReceiverAuthority, CompiledCallerPolicy)],
) -> Result<bool, FixtureError> {
    for (authority, _) in receivers {
        if status(pic, root, authority, [0; 32])?.readiness
            == CallerAuthorityReadiness::FrameworkPending
        {
            return Ok(false);
        }
    }
    let mut releases = Vec::new();
    for (authority, policy) in receivers {
        for (source, _) in receivers {
            if policy.configuration.as_ref().is_some_and(|config| {
                config.permissions.keys().any(|name| {
                    matches_permission(policy, name, &authority.receiver, &source.receiver)
                })
            }) {
                apply(
                    pic,
                    root,
                    authority,
                    CallerAuthorityChange::StageSource(source.receiver.clone()),
                )?;
                apply(
                    pic,
                    root,
                    authority,
                    CallerAuthorityChange::Grant(source.receiver.clone()),
                )?;
            }
        }
        releases.push(apply(
            pic,
            root,
            authority,
            CallerAuthorityChange::OpenReceiver,
        )?);
    }
    let mut ready = true;
    for release in releases {
        let authority = &release.authority;
        if status(pic, root, authority, release.operation_id)?.readiness
            != CallerAuthorityReadiness::ApplicationReady
        {
            ready = false;
            let operation = release.operation_id;
            let response = command(
                pic,
                root,
                authority.receiver.canister(),
                Command::ReleaseApplicationStartup(release),
            )?;
            if !matches!(response, Response::OperationAccepted(receipt) if receipt.operation_id == operation)
            {
                return Err(FixtureError::UnexpectedResponse(
                    "application startup release",
                ));
            }
        }
    }
    Ok(ready)
}

fn status(
    pic: &PocketIc,
    root: Principal,
    authority: &CallerReceiverAuthority,
    operation_id: [u8; 32],
) -> Result<CallerAuthorityStatus, FixtureError> {
    let response: Result<StatusResponse, Error> = pic.query_candid_as(
        authority.receiver.canister(),
        root,
        CANIC_CONTROL_STATUS,
        (StatusRequest::CallerAuthority(OperationStatusRequest {
            operation_id,
        }),),
    )?;
    let StatusResponse::CallerAuthority(status) = response.map_err(FixtureError::Canic)?;
    if status.authority != *authority {
        return Err(FixtureError::Authority(
            "caller receiver installation or policy differs".into(),
        ));
    }
    Ok(status)
}

fn apply(
    pic: &PocketIc,
    root: Principal,
    authority: &CallerReceiverAuthority,
    change: CallerAuthorityChange,
) -> Result<CallerAuthorityPublication, FixtureError> {
    let encoded = candid::encode_args((authority.clone(), change.clone()))
        .map_err(|error| FixtureError::Candid(error.to_string()))?;
    let operation: [u8; 32] = canic_core::cdk::utils::hash::sha256_bytes(&encoded)
        .try_into()
        .map_err(|_| FixtureError::Authority("publication identity length".into()))?;
    let observed = status(pic, root, authority, operation)?;
    let publication = if let Some(receipt) = observed.receipt {
        if receipt.publication.change != change {
            return Err(FixtureError::Authority("changed publication replay".into()));
        }
        receipt.publication
    } else {
        CallerAuthorityApi::publication(authority.clone(), operation, observed.generation, change)
            .map_err(FixtureError::Canic)?
    };
    for request in [
        CallerAuthorityCommand::Prepare(publication.clone()),
        CallerAuthorityCommand::Commit(publication.clone()),
        CallerAuthorityCommand::Complete(publication.clone()),
    ] {
        let Response::CallerAuthority(receipt) = command(
            pic,
            root,
            authority.receiver.canister(),
            Command::CallerAuthority(request),
        )?
        else {
            return Err(FixtureError::UnexpectedResponse("caller publication"));
        };
        if receipt.publication != publication {
            return Err(FixtureError::Authority(
                "publication receipt differs".into(),
            ));
        }
    }
    Ok(publication)
}

fn command(
    pic: &PocketIc,
    root: Principal,
    target: Principal,
    command: Command,
) -> Result<Response, FixtureError> {
    let response: Result<Response, Error> =
        pic.update_candid_as(target, root, CANIC_COMMAND, (command,))?;
    response.map_err(FixtureError::Canic)
}

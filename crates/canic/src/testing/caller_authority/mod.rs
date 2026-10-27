//! Module: testing::caller_authority
//!
//! Protected caller publication for synthetic managed fixtures.
//!
//! Production membership uses Root's journal; fixtures retain exact receipts in their receivers.

use super::managed_app::ManagedAppQualificationError as FixtureError;
use canic_contracts::{
    dto::{caller_authority::*, error::Error, role::OperationStatusRequest},
    ids::CallerReceiverAuthority,
};
use canic_core::{
    api::caller_authority::CallerAuthorityApi, bootstrap::compiled::CompiledCallerPolicy,
    control_plane_support::policy::caller_authority::matches_permission,
};
use ic_platform::pocket_ic::PocketIc;
use ic_platform::principal::ExternalPrincipal as Principal;

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
            let response = release_startup(pic, root, authority.receiver.canister(), release)?;
            if response.operation_id != operation {
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
    use canic_contracts::protocol::caller_authority_status as protocol;
    let response: Result<
        canic_contracts::dto::caller_authority_status::CallerAuthorityStatus,
        Error,
    > = ic_testkit::pic::query_wire(
        pic,
        authority.receiver.canister(),
        root,
        ic_testkit::pic::WireCallSpec {
            method: protocol::METHOD,
            method_id: protocol::METHOD_ID,
            request_limits: protocol::REQUEST_LIMITS,
            response_limits: protocol::RESPONSE_LIMITS,
        },
        &OperationStatusRequest { operation_id },
    )
    .map_err(FixtureError::Wire)?;
    let status: CallerAuthorityStatus = response.map_err(FixtureError::Canic)?.into();
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
    let encoded = canic_contracts::protocol::authority::encode((authority.clone(), change.clone()))
        .map_err(|error| FixtureError::Encoding(error.to_string()))?;
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
        let receipt = command(pic, root, authority.receiver.canister(), request)?;
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
    command: CallerAuthorityCommand,
) -> Result<CallerAuthorityReceipt, FixtureError> {
    let request = canic_contracts::ops::caller_authority_status::publication_request(command);
    let response: Result<
        canic_contracts::dto::caller_authority_status::CallerAuthorityReceipt,
        Error,
    > = ic_testkit::pic::update_wire(
        pic,
        target,
        root,
        ic_testkit::pic::WireCallSpec {
            method: request.method,
            method_id: request.method_id,
            request_limits: canic_contracts::protocol::caller_authority::REQUEST_LIMITS,
            response_limits: canic_contracts::protocol::caller_authority::RESPONSE_LIMITS,
        },
        &request.publication,
    )
    .map_err(FixtureError::Wire)?;
    response.map(Into::into).map_err(FixtureError::Canic)
}

fn release_startup(
    pic: &PocketIc,
    root: Principal,
    target: Principal,
    publication: CallerAuthorityPublication,
) -> Result<canic_contracts::dto::role::OperationReceipt, FixtureError> {
    use canic_contracts::protocol::caller_authority as protocol;
    let publication: canic_contracts::dto::caller_authority_status::CallerAuthorityPublication =
        publication.into();
    let response: Result<_, Error> = ic_testkit::pic::update_wire(
        pic,
        target,
        root,
        ic_testkit::pic::WireCallSpec {
            method: protocol::RELEASE,
            method_id: protocol::RELEASE_ID,
            request_limits: protocol::REQUEST_LIMITS,
            response_limits: protocol::RESPONSE_LIMITS,
        },
        &publication,
    )
    .map_err(FixtureError::Wire)?;
    response.map_err(FixtureError::Canic)
}

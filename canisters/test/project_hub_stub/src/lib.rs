//! Minimal placement-index hub canister for keyed instance placement tests.

#![expect(clippy::unused_async)]

use candid::Principal;
use canic::{
    Error,
    api::auth::AuthApi,
    api::call::Call,
    api::canister::placement::PlacementIndexApi,
    api::rpc::RpcApi,
    dto::{
        auth::{DelegatedToken, SignedRoleAttestation},
        component_registry::{
            RootComponentAllocationResponse, RootComponentCommitRequest,
            RootComponentCreationRequest, RootComponentDirectoryPreparationRequest,
            RootComponentInstallRequest, RootComponentMembershipActivationRequest,
            RootComponentMembershipActivationResponse, RootComponentRuntimeActivationRequest,
            RootPeerComponentAllocationRequest,
        },
        placement::index::{PlacementIndexRecoveryResponse, PlacementIndexStatusResponse},
        rpc::CreateCanisterParent,
    },
    ids::cap,
    prelude::*,
    protocol::{
        CANIC_ROOT_PEER_COMPONENT_ALLOCATE, CANIC_ROOT_PEER_COMPONENT_COMMIT,
        CANIC_ROOT_PEER_COMPONENT_CREATE, CANIC_ROOT_PEER_COMPONENT_DIRECTORY_PREPARE,
        CANIC_ROOT_PEER_COMPONENT_INSTALL, CANIC_ROOT_PEER_COMPONENT_MEMBERSHIP_ACTIVATE,
        CANIC_ROOT_PEER_COMPONENT_RUNTIME_ACTIVATE,
    },
};

const PROJECTS_POOL: &str = "projects";

canic::start!();

// Keep the test hub setup hook empty.
async fn canic_setup() {}

// Keep the test hub install hook empty.
async fn canic_install(_args: Option<Vec<u8>>) {}

// Keep the test hub upgrade hook empty.
async fn canic_upgrade() {}

#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn verifier_verify_token(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;
    Ok(())
}

#[canic_update(requires(auth::authenticated()))]
async fn verifier_verify_token_any(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;
    Ok(())
}

#[canic_update(public)]
async fn verifier_clear_delegated_session() -> Result<(), Error> {
    AuthApi::clear_delegated_session();
    Ok(())
}

#[canic_query(public)]
async fn verifier_delegated_session_subject() -> Result<Option<Principal>, Error> {
    Ok(AuthApi::delegated_session_subject())
}

#[canic_update(public)]
async fn verifier_verify_role_attestation(
    attestation: SignedRoleAttestation,
    min_accepted_epoch: u64,
) -> Result<(), Error> {
    AuthApi::verify_role_attestation(&attestation, min_accepted_epoch).await
}

#[canic_update(requires(auth::attested_local_subnet()))]
async fn verifier_require_attested_local_subnet(
    attestation: SignedRoleAttestation,
) -> Result<(), Error> {
    let _ = attestation;
    Ok(())
}

/// Provision one peer Component through another Fleet Subnet Root as this exact service caller.
#[canic_update(public)]
async fn provision_cross_root_peer(
    fleet_subnet_root: Principal,
    allocation: RootPeerComponentAllocationRequest,
) -> Result<
    (
        RootComponentAllocationResponse,
        RootComponentAllocationResponse,
        RootComponentMembershipActivationResponse,
    ),
    Error,
> {
    let operation_id = allocation.operation_id;
    let reserved: Result<RootComponentAllocationResponse, Error> =
        Call::bounded_wait(fleet_subnet_root, CANIC_ROOT_PEER_COMPONENT_ALLOCATE)
            .with_arg(allocation.clone())?
            .execute_candid()
            .await?;
    let reserved = reserved?;
    let retried: Result<RootComponentAllocationResponse, Error> =
        Call::bounded_wait(fleet_subnet_root, CANIC_ROOT_PEER_COMPONENT_ALLOCATE)
            .with_arg(allocation)?
            .execute_candid()
            .await?;
    let retried = retried?;
    let created: Result<RootComponentAllocationResponse, Error> =
        Call::bounded_wait(fleet_subnet_root, CANIC_ROOT_PEER_COMPONENT_CREATE)
            .with_arg(RootComponentCreationRequest { operation_id })?
            .execute_candid()
            .await?;
    let _created = created?;
    let installed: Result<RootComponentAllocationResponse, Error> =
        Call::bounded_wait(fleet_subnet_root, CANIC_ROOT_PEER_COMPONENT_INSTALL)
            .with_arg(RootComponentInstallRequest { operation_id })?
            .execute_candid()
            .await?;
    let _installed = installed?;
    let committed: Result<canic::dto::component_registry::RootComponentCommitResponse, Error> =
        Call::bounded_wait(fleet_subnet_root, CANIC_ROOT_PEER_COMPONENT_COMMIT)
            .with_arg(RootComponentCommitRequest { operation_id })?
            .execute_candid()
            .await?;
    let _committed = committed?;
    let prepared: Result<
        canic::dto::component_registry::RootComponentDirectoryPreparationResponse,
        Error,
    > = Call::bounded_wait(
        fleet_subnet_root,
        CANIC_ROOT_PEER_COMPONENT_DIRECTORY_PREPARE,
    )
    .with_arg(RootComponentDirectoryPreparationRequest { operation_id })?
    .execute_candid()
    .await?;
    let _prepared = prepared?;
    let activated: Result<
        canic::dto::component_registry::RootComponentRuntimeActivationResponse,
        Error,
    > = Call::bounded_wait(
        fleet_subnet_root,
        CANIC_ROOT_PEER_COMPONENT_RUNTIME_ACTIVATE,
    )
    .with_arg(RootComponentRuntimeActivationRequest { operation_id })?
    .execute_candid()
    .await?;
    let _activated = activated?;
    let membership: Result<RootComponentMembershipActivationResponse, Error> = Call::bounded_wait(
        fleet_subnet_root,
        CANIC_ROOT_PEER_COMPONENT_MEMBERSHIP_ACTIVATE,
    )
    .with_arg(RootComponentMembershipActivationRequest { operation_id })?
    .execute_candid()
    .await?;

    Ok((reserved, retried, membership?))
}

/// Resolve one logical project key to a dedicated instance, creating it when absent.
#[canic_update(public)]
async fn resolve_project(project_key: String) -> Result<PlacementIndexStatusResponse, Error> {
    PlacementIndexApi::resolve_or_create(PROJECTS_POOL, project_key).await
}

/// Repair or release one placement index after partial failure.
#[canic_update(public)]
async fn recover_project(project_key: String) -> Result<PlacementIndexRecoveryResponse, Error> {
    PlacementIndexApi::recover_entry(PROJECTS_POOL, project_key).await
}

/// Look up the currently bound instance pid for one project key.
#[canic_query(public)]
async fn lookup_project(project_key: String) -> Result<Option<Principal>, Error> {
    Ok(PlacementIndexApi::lookup_key(PROJECTS_POOL, &project_key))
}

/// Return the full placement-index state for one project key.
#[canic_query(public)]
async fn lookup_project_entry(
    project_key: String,
) -> Result<Option<PlacementIndexStatusResponse>, Error> {
    Ok(PlacementIndexApi::lookup_entry(PROJECTS_POOL, &project_key))
}

/// Attempt a Ledger request as the Hub so tests prove the parent-role grant rejects it.
#[canic_update(public)]
async fn attempt_project_ledger(operation_id: [u8; 32]) -> Result<Principal, Error> {
    let response = RpcApi::create_canister_request(
        operation_id,
        &CanisterRole::new("project_ledger"),
        CreateCanisterParent::ThisCanister,
        Option::<()>::None,
    )
    .await?;
    Ok(response.new_canister_pid)
}

canic::finish!();

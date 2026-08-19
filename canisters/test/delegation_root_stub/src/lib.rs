//! Minimal Fleet Subnet Root stub for Registry lifecycle tests.

#![expect(clippy::unused_async)]

use canic::{
    Error,
    api::auth::AuthApi,
    dto::auth::{DelegatedToken, SignedRoleAttestation},
    prelude::*,
};
use std::cell::Cell;

thread_local! {
    static LIFECYCLE_INIT_EXECUTIONS: Cell<u64> = const { Cell::new(0) };
    static LIFECYCLE_POST_UPGRADE_EXECUTIONS: Cell<u64> = const { Cell::new(0) };
}

canic::start!(lifecycle_participant(
    init = lifecycle_participant_init,
    post_upgrade = lifecycle_participant_post_upgrade,
),);

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

fn lifecycle_participant_init() {
    assert_restored_root_lifecycle_context();
    assert_eq!(
        (
            LIFECYCLE_INIT_EXECUTIONS.get(),
            LIFECYCLE_POST_UPGRADE_EXECUTIONS.get()
        ),
        (0, 0),
        "the Root init participant must run exactly once"
    );
    LIFECYCLE_INIT_EXECUTIONS.set(LIFECYCLE_INIT_EXECUTIONS.get().saturating_add(1));
}

fn lifecycle_participant_post_upgrade() {
    assert_restored_root_lifecycle_context();
    assert_eq!(
        (
            LIFECYCLE_INIT_EXECUTIONS.get(),
            LIFECYCLE_POST_UPGRADE_EXECUTIONS.get()
        ),
        (0, 0),
        "the Root post-upgrade participant must run exactly once on the fresh heap"
    );
    LIFECYCLE_POST_UPGRADE_EXECUTIONS
        .set(LIFECYCLE_POST_UPGRADE_EXECUTIONS.get().saturating_add(1));
}

fn assert_restored_root_lifecycle_context() {
    let role = canic::api::env::EnvQuery::snapshot()
        .canister_role
        .expect("Canic must restore the Root role before the lifecycle participant");
    assert_eq!(role, canic::api::canister::CanisterRole::ROOT);

    let inventory = ic_timers::timer_inventory()
        .expect("Canic must initialize the shared timer provider before the lifecycle participant");
    for (subsystem, name) in [
        ("async_job_recovery", "watchdog"),
        ("canister_pool", "maintain"),
    ] {
        assert!(
            inventory.timers().iter().any(|timer| {
                let identity = timer.identity();
                identity.owner() == "canic"
                    && identity.subsystem() == subsystem
                    && identity.name() == name
            }),
            "Canic must declare Root timer canic:{subsystem}:{name} before the lifecycle participant"
        );
    }
}

#[canic_update(public)]
async fn root_verify_role_attestation(
    attestation: SignedRoleAttestation,
    min_accepted_epoch: u64,
) -> Result<(), Error> {
    AuthApi::verify_role_attestation(&attestation, min_accepted_epoch).await
}

#[canic_query(public)]
async fn root_now_secs() -> Result<u64, Error> {
    Ok(ic_cdk::api::time() / 1_000_000_000)
}

#[canic_update(requires(caller::is_controller()))]
async fn test_provision_chain_key_delegation_proof_for_issuer(
    issuer_pid: candid::Principal,
) -> Result<(), Error> {
    AuthApi::provision_chain_key_delegation_proof_for_issuer_root(issuer_pid).await
}

#[canic_update(public)]
async fn root_bootstrap_delegated_session(
    token: DelegatedToken,
    delegated_subject: candid::Principal,
    requested_ttl_ns: Option<u64>,
) -> Result<(), Error> {
    AuthApi::set_delegated_session_subject(delegated_subject, token, requested_ttl_ns)
}

#[canic_update(public)]
async fn root_clear_delegated_session() -> Result<(), Error> {
    AuthApi::clear_delegated_session();
    Ok(())
}

#[canic_query(public)]
async fn root_delegated_session_subject() -> Result<Option<candid::Principal>, Error> {
    Ok(AuthApi::delegated_session_subject())
}

canic::finish!();

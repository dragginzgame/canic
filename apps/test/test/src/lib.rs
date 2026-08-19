#![expect(clippy::unused_async)]

use candid::Principal;
use canic::{Error, api::auth::AuthApi, dto::auth::DelegatedToken, ids::cap, prelude::*};
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

async fn canic_install(_: Option<Vec<u8>>) {}

async fn canic_upgrade() {}

fn lifecycle_participant_init() {
    assert_restored_lifecycle_context();
    assert_eq!(
        (
            LIFECYCLE_INIT_EXECUTIONS.get(),
            LIFECYCLE_POST_UPGRADE_EXECUTIONS.get()
        ),
        (0, 0),
        "the managed init participant must run exactly once"
    );
    LIFECYCLE_INIT_EXECUTIONS.set(LIFECYCLE_INIT_EXECUTIONS.get().saturating_add(1));
}

fn lifecycle_participant_post_upgrade() {
    assert_restored_lifecycle_context();
    assert_eq!(
        (
            LIFECYCLE_INIT_EXECUTIONS.get(),
            LIFECYCLE_POST_UPGRADE_EXECUTIONS.get()
        ),
        (0, 0),
        "the managed post-upgrade participant must run exactly once on the fresh heap"
    );
    if option_env!("CANIC_TEST_LIFECYCLE_PARTICIPANT_TRAP").is_some() {
        ic_cdk::trap("managed lifecycle participant requested a test trap");
    }
    LIFECYCLE_POST_UPGRADE_EXECUTIONS
        .set(LIFECYCLE_POST_UPGRADE_EXECUTIONS.get().saturating_add(1));
}

fn assert_restored_lifecycle_context() {
    let role = canic::api::env::EnvQuery::snapshot()
        .canister_role
        .expect("Canic must restore the managed role before the lifecycle participant");
    assert_eq!(role.as_str(), "test");
    ic_timers::timer_inventory()
        .expect("Canic must initialize the shared timer provider before the lifecycle participant");
}

#[canic_update(requires(auth::authenticated(cap::VERIFY)))]
async fn test_verify_delegated_token(token: DelegatedToken) -> Result<(), Error> {
    let _ = token;

    if canic::access::env::build_network_local().is_err() {
        return Err(Error::from_registered(
            canic::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }

    Ok(())
}

#[canic_update(public)]
async fn test_set_delegated_session_subject(
    delegated_subject: Principal,
    bootstrap_token: DelegatedToken,
    requested_ttl_secs: Option<u64>,
) -> Result<(), Error> {
    AuthApi::set_delegated_session_subject(delegated_subject, bootstrap_token, requested_ttl_secs)
}

#[canic_query(public)]
async fn test_delegated_session_subject() -> Result<Option<Principal>, Error> {
    Ok(AuthApi::delegated_session_subject())
}

canic::finish!();

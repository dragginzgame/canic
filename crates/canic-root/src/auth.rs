//! Optional, volatile per-receiver authority under Root's existing controller guard.
use canic::auth::enrollment::{
    self as enrollment, Authority, Error, Grant, Request, Snapshot, transport,
};
use enrollment::types::Principal;
use ic_platform_runtime::{self as runtime, system as api};
use std::{cell::RefCell, collections::BTreeMap};

thread_local! {
    static STATE: RefCell<Option<BTreeMap<Principal, Authority>>> = const { RefCell::new(None) };
}

pub(crate) fn fresh() {
    STATE.with_borrow_mut(|state| *state = Some(BTreeMap::new()));
}

// None after upgrade is closed, not an empty configuration to refill.
/// Require a current controller of this Root.
fn controller<const ID: u64>() -> Result<(), String> {
    if !api::is_controller(&api::caller()) {
        return Err("controller required".into());
    }
    transport::admit::<ID>()
}

/// Approve one receiver grant or confirm an exact retry.
#[runtime::update(
    runtime = runtime,
    guard = controller::<{enrollment::CONFIGURE}>,
    decode = transport::decode::<Grant, {enrollment::CONFIGURE}>,
    max_bytes = 8192,
    on_error = endpoint_error,
    name = "configure_auth",
    encode = transport::encode::<Result<(), Error>, {enrollment::CONFIGURE}>,
)]
fn configure_auth(grant: Grant) -> Result<(), Error> {
    STATE.with_borrow_mut(|state| {
        let grants = state.as_mut().ok_or(Error::Closed)?;
        if let Some(authority) = grants.get(&grant.receiver) {
            return authority.matches(&grant);
        }
        if grants.len() >= enrollment::MAX_RECEIVERS {
            return Err(Error::Capacity);
        }
        grants.insert(grant.receiver, Authority::new(grant, api::time())?);
        Ok(())
    })
}

/// Close the matching receiver grant while preserving grants for other receivers.
#[runtime::update(
    runtime = runtime,
    guard = controller::<{enrollment::REVOKE}>,
    decode = transport::decode::<Grant, {enrollment::REVOKE}>,
    max_bytes = 8192,
    on_error = endpoint_error,
    name = "revoke_auth",
    encode = transport::encode::<Result<(), Error>, {enrollment::REVOKE}>,
)]
#[allow(clippy::needless_pass_by_value)]
fn revoke_auth(grant: Grant) -> Result<(), Error> {
    STATE.with_borrow_mut(|state| {
        let authority = state
            .as_mut()
            .and_then(|g| g.get_mut(&grant.receiver))
            .ok_or(Error::Closed)?;
        authority.matches(&grant)?;
        authority.revoke(api::time())
    })
}

/// Return the snapshot for the calling receiver and its requested binding.
#[runtime::update(
    runtime = runtime,
    guard = transport::admit::<{enrollment::SNAPSHOT}>,
    decode = transport::decode::<Request, {enrollment::SNAPSHOT}>,
    max_bytes = 8192,
    on_error = endpoint_error,
    name = "auth_enrollment",
    encode = transport::encode::<Result<Snapshot, Error>, {enrollment::SNAPSHOT}>,
)]
fn auth_enrollment(request: Request) -> Result<Snapshot, Error> {
    let caller = api::caller();
    STATE.with_borrow_mut(|state| {
        let grants = state.as_mut().ok_or(Error::Closed)?;
        if grants.is_empty() {
            return Err(Error::Closed);
        }
        grants
            .get_mut(&caller)
            .ok_or(Error::Binding)?
            .snapshot(caller, request, api::time())
    })
}

/// Ask a receiver controlled by this Root to pull its current grant.
#[runtime::update(
    runtime = runtime,
    guard = controller::<{enrollment::TRIGGER_REFRESH}>,
    decode = transport::decode::<Principal, {enrollment::TRIGGER_REFRESH}>,
    max_bytes = 8192,
    on_error = endpoint_error,
    name = "refresh_auth_receiver",
    encode = transport::encode::<Result<(), Error>, {enrollment::TRIGGER_REFRESH}>,
)]
async fn refresh_auth_receiver(receiver: Principal) -> Result<(), Error> {
    STATE.with_borrow(|state| {
        let grants = state.as_ref().ok_or(Error::Closed)?;
        grants
            .contains_key(&receiver)
            .then_some(())
            .ok_or(Error::Binding)
    })?;
    transport::trigger_refresh(receiver).await
}

fn endpoint_error(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}

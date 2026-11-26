//! Root controllers approve per-receiver grants. Root keeps the grant details in memory.
use canic::auth::enrollment::{
    self as enrollment, Authority, Error, Grant, Request, Snapshot, transport,
};
use enrollment::types::Principal;
use ic_platform_runtime::{self as runtime, system as api};
use std::{cell::RefCell, collections::BTreeMap};

thread_local! {
    // After upgrade, None fences grants until their history can be recovered.
    static STATE: RefCell<Option<BTreeMap<Principal, Authority>>> = const { RefCell::new(None) };
}

pub(crate) fn fresh() {
    STATE.with_borrow_mut(|state| *state = Some(BTreeMap::new()));
}

/// Require a current controller of this Root.
fn controller<const ID: u64>() -> Result<(), String> {
    canic::guard::require_controller()?;
    transport::admit::<ID>()
}

// grant_auth_access
// Approve one receiver grant or confirm an exact retry.
// guard  : Root controller
// limits : enrollment::LIMITS (8,192 bytes each way)
canic::__canic_inspect!(wire, "grant_auth_access", update, enrollment::CONFIGURE, Grant, Result<(), Error>, enrollment::LIMITS, enrollment::LIMITS);
#[runtime::update(
    runtime = runtime,
    guard = controller::<{enrollment::CONFIGURE}>,
    decode = transport::decode::<Grant, {enrollment::CONFIGURE}>,
    max_bytes = 8192,
    on_error = endpoint_error,
    name = "grant_auth_access",
    encode = transport::encode::<Result<(), Error>, {enrollment::CONFIGURE}>,
)]
fn grant_auth_access(grant: Grant) -> Result<(), Error> {
    STATE.with_borrow_mut(|state| {
        let grants = state.as_mut().ok_or(Error::Closed)?;
        if let Some(authority) = grants.get(&grant.receiver) {
            return authority.matches(&grant);
        }
        if !canic_management::sharding::standby::permits_enrollment(grant.receiver) {
            return Err(Error::Binding);
        }
        if grants.len() >= enrollment::MAX_RECEIVERS {
            return Err(Error::Capacity);
        }
        let receiver = grant.receiver;
        let authority = Authority::new(grant, api::time())?;
        canic_management::record_auth_grant();
        grants.insert(receiver, authority);
        Ok(())
    })
}

// revoke_auth_access
// Close the matching receiver grant while preserving grants for other receivers.
// guard  : Root controller
// limits : enrollment::LIMITS (8,192 bytes each way)
canic::__canic_inspect!(wire, "revoke_auth_access", update, enrollment::REVOKE, Grant, Result<(), Error>, enrollment::LIMITS, enrollment::LIMITS);
#[runtime::update(
    runtime = runtime,
    guard = controller::<{enrollment::REVOKE}>,
    decode = transport::decode::<Grant, {enrollment::REVOKE}>,
    max_bytes = 8192,
    on_error = endpoint_error,
    name = "revoke_auth_access",
    encode = transport::encode::<Result<(), Error>, {enrollment::REVOKE}>,
)]
#[allow(clippy::needless_pass_by_value)]
fn revoke_auth_access(grant: Grant) -> Result<(), Error> {
    STATE.with_borrow_mut(|state| {
        let authority = state
            .as_mut()
            .and_then(|g| g.get_mut(&grant.receiver))
            .ok_or(Error::Closed)?;
        authority.matches(&grant)?;
        authority.revoke(api::time())
    })
}

// fetch_auth_enrollment
// Return the snapshot for the calling receiver and its requested binding.
// guard  : transport::admit (enrollment frame)
// limits : enrollment::LIMITS (8,192 bytes each way)
canic::__canic_inspect!(wire, "fetch_auth_enrollment", update, enrollment::SNAPSHOT, Request, Result<Snapshot, Error>, enrollment::LIMITS, enrollment::LIMITS);
#[runtime::update(
    runtime = runtime,
    guard = transport::admit::<{enrollment::SNAPSHOT}>,
    decode = transport::decode::<Request, {enrollment::SNAPSHOT}>,
    max_bytes = 8192,
    on_error = endpoint_error,
    name = "fetch_auth_enrollment",
    encode = transport::encode::<Result<Snapshot, Error>, {enrollment::SNAPSHOT}>,
)]
fn fetch_auth_enrollment(request: Request) -> Result<Snapshot, Error> {
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

// refresh_auth_receiver
// Ask a receiver controlled by this Root to pull its current grant.
// guard  : Root controller
// limits : enrollment::LIMITS (8,192 bytes each way)
canic::__canic_inspect!(wire, "refresh_auth_receiver", update, enrollment::TRIGGER_REFRESH, Principal, Result<(), Error>, enrollment::LIMITS, enrollment::LIMITS);
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

/// Check that this worker has no retained receiver authority before recycling it.
pub(crate) fn require_no_worker_grant(worker: Principal) -> Result<(), String> {
    STATE.with_borrow(|s| match s {
        Some(grants) if !grants.contains_key(&worker) => Ok(()),
        _ => Err("worker Auth grant history is occupied or unavailable".into()),
    })
}

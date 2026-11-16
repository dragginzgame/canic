//! Optional, volatile single-grant authority under Root's existing controller guard.
use canic::auth::enrollment::{
    self as enrollment, Authority, Error, Grant, Request, Snapshot, transport,
};
use ic_platform_runtime::{self as runtime, system as api};
use std::cell::RefCell;

enum State {
    Fresh,
    Configured(Box<Authority>),
}
thread_local! { static STATE: RefCell<Option<State>> = const { RefCell::new(None) }; }
pub(crate) fn fresh() {
    STATE.with_borrow_mut(|s| *s = Some(State::Fresh));
}
// None after upgrade is closed, not an empty configuration to refill.
fn controller<const ID: u64>() -> Result<(), String> {
    if !api::is_controller(&api::caller()) {
        return Err("controller required".into());
    }
    transport::admit::<ID>()
}
#[runtime::update(runtime = runtime, guard = controller::<{enrollment::CONFIGURE}>, decode = transport::decode::<Grant, {enrollment::CONFIGURE}>, max_bytes = 8192, on_error = endpoint_error, name = "configure_auth", encode = transport::encode::<Result<(), Error>, {enrollment::CONFIGURE}>)]
fn configure_auth(grant: Grant) -> Result<(), Error> {
    STATE.with_borrow_mut(|state| match state {
        None => Err(Error::Closed),
        Some(State::Configured(authority)) => authority.matches(&grant),
        Some(State::Fresh) => {
            let authority = Authority::new(grant, api::time())?;
            *state = Some(State::Configured(Box::new(authority)));
            Ok(())
        }
    })
}
#[runtime::update(runtime = runtime, guard = controller::<{enrollment::REVOKE}>, decode = transport::decode::<Grant, {enrollment::REVOKE}>, max_bytes = 8192, on_error = endpoint_error, name = "revoke_auth", encode = transport::encode::<Result<(), Error>, {enrollment::REVOKE}>)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "the IC entrypoint owns its decoded request"
)]
fn revoke_auth(grant: Grant) -> Result<(), Error> {
    STATE.with_borrow_mut(|state| match state {
        Some(State::Configured(authority)) => {
            authority.matches(&grant)?;
            authority.revoke(api::time())
        }
        _ => Err(Error::Closed),
    })
}
#[runtime::update(runtime = runtime, guard = transport::admit::<{enrollment::SNAPSHOT}>, decode = transport::decode::<Request, {enrollment::SNAPSHOT}>, max_bytes = 8192, on_error = endpoint_error, name = "auth_enrollment", encode = transport::encode::<Result<Snapshot, Error>, {enrollment::SNAPSHOT}>)]
fn auth_enrollment(request: Request) -> Result<Snapshot, Error> {
    STATE.with_borrow_mut(|state| match state {
        Some(State::Configured(authority)) => {
            authority.snapshot(api::caller(), request, api::time())
        }
        _ => Err(Error::Closed),
    })
}

fn endpoint_error(error: impl std::fmt::Debug) -> ! {
    ic_platform_runtime::system::trap(&format!("endpoint refused: {error:?}"))
}

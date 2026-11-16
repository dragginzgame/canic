//! Optional, volatile single-grant authority under Root's existing controller guard.
use canic::auth::enrollment::{
    self as enrollment, Authority, Error, Grant, Request, Snapshot, transport,
};
use ic_platform_runtime::{cdk, cdk::api};
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
    if !api::is_controller(&api::msg_caller()) {
        return Err("controller required".into());
    }
    transport::admit::<ID>()
}
#[cdk::update(
    guard = "controller::<{enrollment::CONFIGURE}>",
    decode_with = "transport::decode::<Grant, {enrollment::CONFIGURE}>",
    encode_with = "transport::encode::<Result<(), Error>, {enrollment::CONFIGURE}>"
)]
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
#[cdk::update(
    guard = "controller::<{enrollment::REVOKE}>",
    decode_with = "transport::decode::<Grant, {enrollment::REVOKE}>",
    encode_with = "transport::encode::<Result<(), Error>, {enrollment::REVOKE}>"
)]
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
#[cdk::update(
    guard = "transport::admit::<{enrollment::SNAPSHOT}>",
    decode_with = "transport::decode::<Request, {enrollment::SNAPSHOT}>",
    encode_with = "transport::encode::<Result<Snapshot, Error>, {enrollment::SNAPSHOT}>"
)]
fn auth_enrollment(request: Request) -> Result<Snapshot, Error> {
    STATE.with_borrow_mut(|state| match state {
        Some(State::Configured(authority)) => {
            authority.snapshot(api::msg_caller(), request, api::time())
        }
        _ => Err(Error::Closed),
    })
}

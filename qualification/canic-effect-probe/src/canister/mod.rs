//! Installed probes sharing only authentication and dispatch fault injection.
mod creation;
mod funding;
mod maintenance;

use ic_platform_types::Principal;

use crate::{Fault, LIMITS};
use ic_platform_runtime::{
    call::{Call, Completion, SubmitError, Wait},
    system as api,
};
use std::{future::Future, pin::pin, task::Poll};

/// Require a controller and reject oversized frames before decoding.
fn controller() -> Result<(), String> {
    if !api::is_controller(&api::caller()) {
        return Err("controller required".into());
    }
    if ic_platform_runtime::system::argument_size() > LIMITS.bytes {
        return Err("request too large".into());
    }
    Ok(())
}

/// Allow only a continuation dispatched by this canister.
fn private_continuation() -> Result<(), String> {
    if api::caller() == api::canister_self() {
        Ok(())
    } else {
        Err("self call required".into())
    }
}

async fn dispatch(
    method: &str,
    request: Vec<u8>,
    cycles: u128,
    fault: Fault,
) -> Result<Completion, SubmitError> {
    let call = Call::new(Principal::management_canister(), method, Wait::Unbounded)
        .owned_args(request)
        .cycles(cycles)
        .execute();
    let mut call = pin!(call);
    if fault == Fault::Dispatch {
        std::future::poll_fn::<(), _>(|cx| {
            assert!(
                matches!(call.as_mut().poll(cx), Poll::Pending),
                "call must enqueue"
            );
            api::trap("injected dispatch trap after enqueue");
        })
        .await;
    }
    call.await
}

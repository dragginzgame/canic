//! Installed probes sharing only authentication and dispatch fault injection.

mod creation;
mod funding;
mod maintenance;

use crate::{Fault, LIMITS};
use ic_platform_runtime::{
    Principal,
    call::{Call, CallFailed, Response, WaitPolicy},
    cdk::api,
};
use std::{future::Future, pin::pin, task::Poll};

fn controller() -> Result<(), String> {
    if !api::is_controller(&api::msg_caller()) {
        return Err("controller required".into());
    }
    if ic_platform_runtime::ic0::msg_arg_data_size() > LIMITS.bytes {
        return Err("request too large".into());
    }
    Ok(())
}

fn private_continuation() -> Result<(), String> {
    if api::msg_caller() == api::canister_self() {
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
) -> Result<Response, CallFailed> {
    let call = Call::new(
        Principal::management_canister(),
        method,
        WaitPolicy::Unbounded,
    )
    .take_raw_args(request)
    .with_cycles(cycles)
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

//! One controller-authorized Store call. No catalog, payload persistence or retry owner.
use canic_management_protocol::publication::{Command, Error, MAX_WINDOW_NS, Reply, Request};
use canic_wasm_store_protocol as store;
use ic_platform_runtime::{
    Principal,
    call::{Call, WaitPolicy},
    cdk::api,
};

/// Forward one typed publication command under the captured controller's authority.
/// A callback/encoding trap cannot undo an already committed Store mutation.
/// The caller retains its exact command and uses Store observation to recover.
/// # Errors
/// Refuses invalid/expired dispatch, propagates Store refusals, or reports uncertainty.
/// # Panics
/// Traps unless the actual caller currently controls this Root.
pub async fn execute(input: Request) -> Reply {
    let target = admit(&input.store, input.expires_at_ns)?;
    let (method, id, bytes, release) = match &input.command {
        Command::Prepare(value) => (
            "prepare",
            store::PREPARE,
            ic_wire::native::encode_request(store::PREPARE, value, store::LIMITS),
            &value.release,
        ),
        Command::Put(value) => (
            "put",
            store::PUT,
            ic_wire::native::encode_request(store::PUT, value, store::CHUNK_LIMITS),
            &value.release,
        ),
        Command::Publish(value) => (
            "publish",
            store::PUBLISH,
            ic_wire::native::encode_request(store::PUBLISH, value, store::LIMITS),
            value,
        ),
        Command::Observe(value) => (
            "observe",
            store::STATUS,
            ic_wire::native::encode_request(store::STATUS, value, store::LIMITS),
            value,
        ),
    };
    // No await separates controller/deadline admission from this one dispatch.
    let bytes = bytes.map_err(|_| Error::Invalid)?;
    let reply = Call::new(target, method, WaitPolicy::Bounded { timeout_seconds: 5 })
        .take_raw_args(bytes)
        .execute()
        .await
        .map_err(|_| Error::Uncertain)?;
    let reply: Result<store::Status, store::Error> =
        ic_wire::native::decode_response(id, &reply, store::LIMITS)
            .map_err(|_| Error::Uncertain)?;
    let status = reply.map_err(Error::Store)?;
    if &status.manifest.release != release
        || matches!(&input.command, Command::Prepare(expected) if expected != &status.manifest)
    {
        return Err(Error::Uncertain);
    }
    Ok(status)
}

fn admit(store: &str, expires_at_ns: u64) -> Result<Principal, Error> {
    assert!(
        api::is_controller(&api::msg_caller()),
        "controller required"
    );
    let target = Principal::from_text(store).map_err(|_| Error::Invalid)?;
    if target.to_text() != store
        || target == Principal::anonymous()
        || target == Principal::management_canister()
        || target == api::canister_self()
    {
        return Err(Error::Invalid);
    }
    let now = api::time();
    if expires_at_ns <= now {
        return Err(Error::Expired);
    }
    if expires_at_ns - now > MAX_WINDOW_NS {
        return Err(Error::Invalid);
    }
    Ok(target)
}

/// Read Store state after handover through the actual controller canister.
/// This performs no storage write, timer registration or management effect.
/// # Errors
/// Refuses invalid/expired targets or reports an unavailable/invalid Store reply.
/// # Panics
/// Traps unless the actual caller currently controls Root.
pub async fn observe_info(
    input: canic_management_protocol::publication::InfoRequest,
) -> canic_management_protocol::publication::InfoReply {
    let target = admit(&input.store, input.expires_at_ns)?;
    let bytes = ic_wire::native::encode_request(store::INFO, &(), store::LIMITS)
        .map_err(|_| Error::Invalid)?;
    let reply = Call::new(
        target,
        "observe_info",
        WaitPolicy::Bounded { timeout_seconds: 5 },
    )
    .take_raw_args(bytes)
    .execute()
    .await
    .map_err(|_| Error::Uncertain)?;
    ic_wire::native::decode_response(store::INFO, &reply, store::LIMITS)
        .map_err(|_| Error::Uncertain)
}

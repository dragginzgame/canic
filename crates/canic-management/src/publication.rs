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
    assert!(
        api::is_controller(&api::msg_caller()),
        "controller required"
    );
    let target = Principal::from_text(&input.store).map_err(|_| Error::Invalid)?;
    if target.to_text() != input.store
        || target == Principal::anonymous()
        || target == Principal::management_canister()
        || target == api::canister_self()
    {
        return Err(Error::Invalid);
    }
    let now = api::time();
    if input.expires_at_ns <= now {
        return Err(Error::Expired);
    }
    if input.expires_at_ns - now > MAX_WINDOW_NS {
        return Err(Error::Invalid);
    }
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

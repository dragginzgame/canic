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
    if crate::engine::store_maintenance::active() && !matches!(input.command, Command::Observe(_)) {
        return Err(Error::Maintenance);
    }
    let (method, id, bytes, release) = match &input.command {
        Command::Prepare(value) => (
            "prepare",
            store::PREPARE,
            ic_wire::encode(
                value,
                ic_wire::Encoding::Frame {
                    method: store::PREPARE,
                    direction: ic_wire::Direction::Request,
                },
                store::LIMITS,
            ),
            &value.release,
        ),
        Command::Put(value) => (
            "put",
            store::PUT,
            ic_wire::encode(
                value,
                ic_wire::Encoding::Frame {
                    method: store::PUT,
                    direction: ic_wire::Direction::Request,
                },
                store::CHUNK_LIMITS,
            ),
            &value.release,
        ),
        Command::Publish(value) => (
            "publish",
            store::PUBLISH,
            ic_wire::encode(
                value,
                ic_wire::Encoding::Frame {
                    method: store::PUBLISH,
                    direction: ic_wire::Direction::Request,
                },
                store::LIMITS,
            ),
            value,
        ),
        Command::Observe(value) => (
            "observe",
            store::STATUS,
            ic_wire::encode(
                value,
                ic_wire::Encoding::Frame {
                    method: store::STATUS,
                    direction: ic_wire::Direction::Request,
                },
                store::LIMITS,
            ),
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
    let reply: Result<store::Status, store::Error> = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Response,
        },
        store::LIMITS,
    )
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
    metadata_call(target, "observe_info", store::INFO, &()).await
}

/// Read one release and all retention obligations on its artifact through Root.
/// No stable state changes, payload reads, pin creation or maintenance effects occur.
/// # Errors
/// Refuses invalid/expired admission, propagates Store refusals, or reports an invalid reply.
/// # Panics
/// Traps unless the actual caller currently controls Root.
pub async fn observe_custody(
    input: canic_management_protocol::publication::CustodyRequest,
) -> canic_management_protocol::publication::CustodyReply {
    let target = admit(&input.store, input.expires_at_ns)?;
    if !input.release.valid() {
        return Err(Error::Invalid);
    }
    let reply: Result<store::Custody, store::Error> =
        metadata_call(target, "observe_custody", store::CUSTODY, &input.release).await?;
    let custody = reply.map_err(Error::Store)?;
    if custody.status.manifest.release != input.release || custody.pins.len() > store::MAX_PINS {
        return Err(Error::Uncertain);
    }
    Ok(custody)
}

/// Close, inspect or activate Store mutation admission under the current controller.
/// Store retains its exact attempt. Root owns no installation journal through this call.
/// # Errors
/// Refuses invalid/expired targets, propagates Store refusals or reports transport uncertainty.
/// # Panics
/// Traps unless the actual caller currently controls Root.
pub async fn admission(
    input: canic_management_protocol::publication::AdmissionRequest,
) -> canic_management_protocol::publication::AdmissionReply {
    let target = admit(&input.store, input.expires_at_ns)?;
    if crate::engine::store_maintenance::active()
        && input.command != store::AdmissionCommand::Observe
    {
        return Err(Error::Maintenance);
    }
    let reply: Result<Option<store::Admission>, store::Error> =
        metadata_call(target, "admission", store::ADMISSION, &input.command).await?;
    reply.map_err(Error::Store)
}

// Small typed calls share transport and codec bounds; no arbitrary forwarding or Root mirror.
async fn metadata_call<T: serde::Serialize, R: serde::de::DeserializeOwned>(
    target: Principal,
    method: &'static str,
    id: u64,
    input: &T,
) -> Result<R, Error> {
    let bytes = ic_wire::encode(
        input,
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Request,
        },
        store::LIMITS,
    )
    .map_err(|_| Error::Invalid)?;
    let reply = Call::new(target, method, WaitPolicy::Bounded { timeout_seconds: 5 })
        .take_raw_args(bytes)
        .execute()
        .await
        .map_err(|_| Error::Uncertain)?;
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Response,
        },
        store::LIMITS,
    )
    .map_err(|_| Error::Uncertain)
}

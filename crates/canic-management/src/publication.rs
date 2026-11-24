//! One controller-authorized Store call. No catalog, payload persistence or retry owner.
use canic_management_protocol::{
    publication::{Command, Error, MAX_WINDOW_NS, Reply, Request},
    stored::{STORE_CHUNK_LIMITS, STORE_LIMITS},
};
use ic_platform_runtime::{
    call::{Call, Wait},
    system as api,
};
use ic_platform_types::Principal;
use ic_wasm_store_protocol as store;

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
                STORE_LIMITS,
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
                STORE_CHUNK_LIMITS,
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
                STORE_LIMITS,
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
                STORE_LIMITS,
            ),
            value,
        ),
    };
    // No await separates controller/deadline admission from this one dispatch.
    let bytes = bytes.map_err(|_| Error::Invalid)?;
    let reply = Call::new(target, method, Wait::Bounded { seconds: 5 })
        .owned_args(bytes)
        .execute()
        .await
        .map_err(|_| Error::Uncertain)?;
    let reply = match reply.into_parts().0 {
        ic_platform_runtime::call::Outcome::Reply(bytes) => bytes,
        _ => return Err(Error::Uncertain),
    };
    let reply: Result<store::Status, store::Error> = ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Response,
        },
        STORE_LIMITS,
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
    assert!(api::is_controller(&api::caller()), "controller required");
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

/// Dispatch one exact removal, waiting for Store's response without a best-effort timeout.
/// The caller owns durable signed-attempt custody. Only a matching successful reply
/// settles this call; observation, expiry or a callback trap cannot substitute for it.
/// Before restoring a label, settle all removals and expire their dispatch authority.
/// This endpoint does not deduplicate newly signed ingress or automatically retry.
/// # Errors
/// Refuses invalid, expired or maintenance-fenced calls; propagates supplier refusal.
/// Missing or invalid responses retain uncertainty and cannot authorize restoration.
/// # Panics
/// Traps unless the actual caller controls Root, or if a qualification reply fault is selected.
pub async fn remove(
    input: canic_management_protocol::publication::RemovalRequest,
) -> canic_management_protocol::publication::RemovalReply {
    use canic_management_protocol::publication::Removal;
    let target = admit(&input.store, input.expires_at_ns)?;
    if crate::engine::store_maintenance::active() {
        return Err(Error::Maintenance);
    }
    if !input.manifest.valid() {
        return Err(Error::Invalid);
    }
    let (method, id) = match input.action {
        Removal::Delete => ("delete", store::DELETE),
        Removal::Cancel => ("cancel", store::CANCEL),
    };
    // A stalled Store may retain this call indefinitely; the caller stays blocked.
    let reply: Result<store::Manifest, store::Error> =
        store_call(target, method, id, &input.manifest, Wait::Unbounded).await?;
    let removed = reply.map_err(Error::Store)?;
    if removed != input.manifest {
        return Err(Error::Uncertain);
    }
    Ok(removed)
}

/// Dispatch one read-only Store operation as the actual Root controller.
/// # Errors
/// Propagates admission, transport and Store refusals.
pub async fn observe(
    input: canic_management_protocol::publication::ObservationRequest,
) -> canic_management_protocol::publication::ObservationReply {
    use canic_management_protocol::publication::{Observation, ObservationRequest};
    match input {
        ObservationRequest::Info(request) => observe_info(request).await.map(Observation::Info),
        ObservationRequest::Custody(request) => {
            observe_custody(request).await.map(Observation::Custody)
        }
    }
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
    store_call(
        target,
        "observe_info",
        store::INFO,
        &(),
        Wait::Bounded { seconds: 5 },
    )
    .await
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
    let reply: Result<store::Custody, store::Error> = store_call(
        target,
        "observe_custody",
        store::CUSTODY,
        &input.release,
        Wait::Bounded { seconds: 5 },
    )
    .await?;
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
    let reply: Result<Option<store::Admission>, store::Error> = store_call(
        target,
        "admission",
        store::ADMISSION,
        &input.command,
        Wait::Bounded { seconds: 5 },
    )
    .await?;
    reply.map_err(Error::Store)
}

// Small typed calls share transport and codec bounds; no arbitrary forwarding or Root mirror.
async fn store_call<T: serde::Serialize, R: serde::de::DeserializeOwned>(
    target: Principal,
    method: &'static str,
    id: u64,
    input: &T,
    wait: Wait,
) -> Result<R, Error> {
    let bytes = ic_wire::encode(
        input,
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Request,
        },
        STORE_LIMITS,
    )
    .map_err(|_| Error::Invalid)?;
    let reply = Call::new(target, method, wait)
        .owned_args(bytes)
        .execute()
        .await
        .map_err(|_| Error::Uncertain)?;
    let reply = match reply.into_parts().0 {
        ic_platform_runtime::call::Outcome::Reply(bytes) => bytes,
        _ => return Err(Error::Uncertain),
    };
    ic_wire::decode(
        &reply,
        ic_wire::Encoding::Frame {
            method: id,
            direction: ic_wire::Direction::Response,
        },
        STORE_LIMITS,
    )
    .map_err(|_| Error::Uncertain)
}

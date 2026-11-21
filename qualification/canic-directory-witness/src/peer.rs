//! Endpoint-owned project sync policy on a directory-backed accounts hub.
use super::{DIRECTORY, Principal};
use canic::method::{Contract, wire};
use canic::topology::require_canister;
use ic_platform_runtime::{call, system};
use std::cell::Cell;

thread_local! {
    static WRITES: Cell<u64> = const { Cell::new(0) };
    static PAUSED: Cell<bool> = const { Cell::new(false) };
    static EPOCH: Cell<u64> = const { Cell::new(0) };
}

fn writable() -> Result<(), &'static str> {
    (!PAUSED.get()).then_some(()).ok_or("writes paused")
}

pub(super) fn controller() -> Result<(), &'static str> {
    system::is_controller(&system::caller())
        .then_some(())
        .ok_or("controller required")
}

#[canic::update(guard = controller)]
fn pause_writes(paused: bool) {
    if PAUSED.replace(paused) != paused {
        EPOCH.update(|n| n.checked_add(1).expect("write epoch"));
    }
}

struct ProjectSync;
impl Contract for ProjectSync {
    const NAME: &'static str = "project_sync";
    const ID: u64 = wire::method_id(Self::NAME);
    type Input = Option<Principal>;
    type Output = Result<u64, String>;
}

#[canic::update(contract = ProjectSync, guard = require_canister("project_hub"))]
async fn project_sync(barrier: Option<Principal>) -> Result<u64, String> {
    writable()?;
    let epoch = EPOCH.get();
    if let Some(barrier) = barrier {
        let reply = call::Call::new(barrier, "hold", call::Wait::Bounded { seconds: 300 })
            .args(&[])
            .execute()
            .await
            .map_err(|_| "barrier unavailable")?;
        if !matches!(reply.into_parts().0, call::Outcome::Reply(bytes) if bytes.is_empty()) {
            return Err("barrier refused".into());
        }
        // The directory binding is immutable. The application's write policy can change.
        if EPOCH.get() != epoch {
            return Err("write permission changed".into());
        }
    }
    WRITES.update(|n| n + 1);
    Ok(WRITES.get())
}

// Both the service and its controller may inspect this witness counter.
fn may_read_count() -> Result<(), &'static str> {
    require_canister("project_hub").or_else(|_| controller())
}

#[canic::query(guard = may_read_count)]
fn sync_count() -> u64 {
    WRITES.get()
}

#[canic::update(guard = controller)]
async fn send_sync(barrier: Option<Principal>) -> Result<u64, String> {
    let target = DIRECTORY
        .with_borrow(|s| s.as_ref()?.current()?.by_name("accounts_hub"))
        .ok_or("accounts hub unavailable")?;
    canic::method::call::<ProjectSync>(target, 300, &barrier)
        .await
        .map_err(|_| "sync unconfirmed")?
}

// Legacy proof retrieval requires a data certificate in the signing canister.
struct CertificateAvailable;
impl Contract for CertificateAvailable {
    const NAME: &'static str = "certificate_available";
    const ID: u64 = wire::method_id(Self::NAME);
    type Input = ();
    type Output = bool;
}

// Platform Runtime 0.23.23 exposes this system primitive only on IC Wasm.
#[cfg(target_arch = "wasm32")]
#[canic::query(contract = CertificateAvailable)]
fn certificate_available() -> bool {
    system::data_certificate(65536)
        .expect("certificate budget")
        .is_some()
}

#[canic::update(guard = controller)]
async fn probe_certificate() -> bool {
    let receiver = DIRECTORY
        .with_borrow(|s| s.as_ref()?.current()?.by_name("accounts_hub"))
        .expect("published accounts hub");
    canic::method::call::<CertificateAvailable>(receiver, 5, &())
        .await
        .expect("replicated query call")
}

// The installed receiver reports whether it observed an IC message deadline.
struct HasDeadline;
impl Contract for HasDeadline {
    const NAME: &'static str = "has_deadline";
    const ID: u64 = 0x4341_4e49_4300_0067;
    type Input = ();
    type Output = bool;
}

#[canic::update(guard = controller)]
async fn probe_wait((target, seconds): (Principal, Option<u32>)) -> bool {
    match seconds {
        Some(seconds) => canic::method::call::<HasDeadline>(target, seconds, &()).await,
        None => canic::method::call_unbounded::<HasDeadline>(target, &()).await,
    }
    .expect("typed wait probe")
}

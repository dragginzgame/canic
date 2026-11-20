//! A named project hub calls a single permitted method on its accounts hub.
use super::{DefaultMemoryImpl, INDEX, Principal, RefCell};
use canic::method::{Contract, wire};
use canic::topology::peer::{ApplyPeer, Decision, Permissions};
use ic_platform_runtime::{call, system};
use std::cell::Cell;

pub(super) const KEY: &str = "hub.peers.v1";
thread_local! {
    static PEERS: RefCell<Option<Permissions<DefaultMemoryImpl>>> = const { RefCell::new(None) };
    static WRITES: Cell<u64> = const { Cell::new(0) };
}

pub(super) fn open(runtime: &ic_storage::MemoryRuntime<DefaultMemoryImpl>, fresh: bool) {
    PEERS.set(Some(Permissions::attach(runtime, KEY, fresh)));
}

#[canic::update(contract = ApplyPeer)]
fn apply_peer(request: Decision) -> Result<Decision, String> {
    PEERS.with_borrow_mut(|s| {
        s.as_mut()
            .expect("open permissions")
            .receive(request)
            .map_err(String::from)
    })
}

struct ProjectSync;
impl Contract for ProjectSync {
    const NAME: &'static str = "project_sync";
    const ID: u64 = wire::method_id(Self::NAME);
    type Input = Option<Principal>;
    type Output = Result<u64, String>;
}

#[canic::update(contract = ProjectSync)]
async fn project_sync(barrier: Option<Principal>) -> Result<u64, String> {
    let ticket =
        PEERS.with_borrow(|s| s.as_ref().expect("open permissions").admit::<ProjectSync>())?;
    if let Some(barrier) = barrier {
        let reply = call::Call::new(barrier, "hold", call::Wait::Bounded { seconds: 300 })
            .args(&[])
            .execute()
            .await
            .map_err(|_| "barrier unavailable")?;
        if !matches!(reply.into_parts().0, call::Outcome::Reply(bytes) if bytes.is_empty()) {
            return Err("barrier refused".into());
        }
    }
    PEERS.with_borrow(|s| s.as_ref().expect("open permissions").revalidate(&ticket))?;
    WRITES.update(|n| n + 1);
    Ok(WRITES.get())
}

#[canic::update]
async fn send_sync(barrier: Option<Principal>) -> Result<u64, String> {
    // Qualification ingress represents the application's authorized operation.
    if !system::is_controller(&system::caller()) {
        return Err("controller required".into());
    }
    let target = INDEX
        .with_borrow(|s| s.as_ref()?.current()?.by_name("accounts_hub"))
        .ok_or("accounts hub unavailable")?;
    canic::method::call::<ProjectSync>(target, 300, &barrier)
        .await
        .map_err(|_| "sync unconfirmed")?
}

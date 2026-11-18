use canic::sharding::{self, Placement};
use ic_platform_runtime as runtime;
use ic_platform_types::Principal;
use sha2::{Digest, Sha256};
use std::cell::Cell;
thread_local! {static ROOT: Cell<Option<Principal>> = const {Cell::new(None)};}
const KEY: &str = "hub.root.v1";
fn open(initial: Option<Principal>) {
    let fresh = initial.is_some();
    let storage = canic::storage::bootstrap(
        ic_storage::DefaultMemoryImpl::default(),
        fresh,
        "hub",
        &[KEY],
        4,
    );
    let cell = canic::storage::cell(&storage, KEY, 29, initial.map(|p| p.as_slice().to_vec()));
    ROOT.set(Some(
        Principal::try_from_slice(cell.get()).expect("retained Root"),
    ));
}
#[runtime::init(runtime = runtime, guard = allow, decode = principal, max_bytes = 29, on_error = fail)]
fn init(root: Principal) {
    open(Some(root));
}
#[runtime::post_upgrade(runtime = runtime, guard = allow, decode = empty, max_bytes = 0, on_error = fail)]
fn reopen((): ()) {
    open(None);
}
#[runtime::update(runtime = runtime, name = "signup", guard = user, decode = empty, encode = reply, max_bytes = 0, on_error = fail)]
async fn signup((): ()) -> Placement {
    sharding::assign(ROOT.get().unwrap(), key())
        .await
        .expect("inspect original key")
}
#[runtime::update(runtime = runtime, name = "lookup", guard = user, decode = empty, encode = reply, max_bytes = 0, on_error = fail)]
async fn lookup((): ()) -> Placement {
    sharding::lookup(ROOT.get().unwrap(), key())
        .await
        .expect("lookup transport")
}
fn key() -> sharding::Key {
    Sha256::digest(runtime::system::caller().as_slice()).into()
}
fn user() -> Result<(), &'static str> {
    (runtime::system::caller() != Principal::anonymous())
        .then_some(())
        .ok_or("authenticated user")
}
fn principal(raw: &[u8]) -> Result<Principal, &'static str> {
    Principal::try_from_slice(raw).map_err(|_| "root principal")
}
fn empty(raw: &[u8]) -> Result<(), &'static str> {
    raw.is_empty()
        .then_some(())
        .ok_or("empty argument required")
}
fn allow() -> Result<(), std::convert::Infallible> {
    Ok(())
}
#[expect(
    clippy::needless_pass_by_value,
    reason = "Platform passes the owned reply"
)]
fn reply(value: Placement) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(&value, ic_wire::Encoding::Payload, sharding::LIMITS)
}
fn fail(error: impl std::fmt::Debug) -> ! {
    runtime::system::trap(&format!("hub: {error:?}"))
}

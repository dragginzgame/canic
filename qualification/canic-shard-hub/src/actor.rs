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

canic::mount_lifecycle!(
    input = Principal,
    max_bytes = 29,
    decode = principal,
    install = install,
    reopen = reopen
);

fn install(root: Principal) {
    open(Some(root));
}

fn reopen() {
    open(None);
}

/// Assign the authenticated caller's key through the configured Root.
#[runtime::update(
    runtime = runtime,
    name = "signup",
    guard = user,
    decode = empty,
    encode = reply,
    max_bytes = 0,
    on_error = fail,
)]
async fn signup((): ()) -> Placement {
    sharding::assign(ROOT.get().unwrap(), "user_shard", key())
        .await
        .unwrap_or_else(|reason| fail(reason))
}

/// Look up the authenticated caller's existing assignment.
#[runtime::update(
    runtime = runtime,
    name = "lookup",
    guard = user,
    decode = empty,
    encode = reply,
    max_bytes = 0,
    on_error = fail,
)]
async fn lookup((): ()) -> Placement {
    sharding::lookup(ROOT.get().unwrap(), "user_shard", key())
        .await
        .unwrap_or_else(|reason| fail(reason))
}

/// Assign one authenticated user's project key through the configured Root.
#[runtime::update(
    runtime = runtime,
    name = "project",
    guard = user,
    decode = project_request,
    encode = reply,
    max_bytes = 32,
    on_error = fail,
)]
async fn project(key: sharding::Key) -> Placement {
    sharding::assign_instance(ROOT.get().unwrap(), key)
        .await
        .unwrap_or_else(|reason| fail(reason))
}

/// Look up one authenticated user's original project.
#[runtime::update(
    runtime = runtime,
    name = "lookup_project",
    guard = user,
    decode = project_request,
    encode = reply,
    max_bytes = 32,
    on_error = fail,
)]
async fn lookup_project(key: sharding::Key) -> Placement {
    sharding::lookup_instance(ROOT.get().unwrap(), key)
        .await
        .unwrap_or_else(|reason| fail(reason))
}

fn project_request(raw: &[u8]) -> Result<sharding::Key, &'static str> {
    let request: [u8; 32] = raw.try_into().map_err(|_| "project request key")?;
    Ok(Sha256::new()
        .chain_update(key())
        .chain_update(request)
        .finalize()
        .into())
}

fn key() -> sharding::Key {
    Sha256::digest(runtime::system::caller().as_slice()).into()
}

/// Require an identified caller before deriving its placement key.
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

#[allow(clippy::needless_pass_by_value)]
fn reply(value: Placement) -> Result<Vec<u8>, ic_wire::Error> {
    ic_wire::encode(&value, ic_wire::Encoding::Payload, sharding::LIMITS)
}

fn fail(error: impl std::fmt::Debug) -> ! {
    runtime::system::trap(&format!("hub: {error:?}"))
}

// pay
// Exercise real requester-attached cycles from this installed hub.
// guard  : fixture controller
// limits : 1 KiB request and reply
#[canic::update(guard = controller, limits = sharding::LIMITS)]
async fn pay(
    input: (sharding::payment::Demand, u128, u128),
) -> Result<sharding::payment::Payment, String> {
    sharding::payment::fund(ROOT.get().unwrap(), input.0, input.1, input.2).await
}

fn controller() -> Result<(), &'static str> {
    runtime::system::is_controller(&runtime::system::caller())
        .then_some(())
        .ok_or("controller required")
}

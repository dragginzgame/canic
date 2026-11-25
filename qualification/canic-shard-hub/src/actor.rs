use canic::sharding::{self, Placement};
use ic_platform_runtime as runtime;
use ic_platform_types::Principal;
use sha2::{Digest, Sha256};
use std::cell::Cell;

thread_local! {pub(super) static ROOT: Cell<Option<Principal>> = const {Cell::new(None)};}
const KEY: &str = "hub.root.v1";

fn open(initial: Option<(Principal, Option<Principal>)>) {
    let fresh = initial.is_some();
    let storage = canic::storage::bootstrap(
        ic_storage::DefaultMemoryImpl::default(),
        fresh,
        "hub",
        &[
            KEY,
            super::routes::KEY,
            "hub.shards.v1",
            "hub.owners.v1",
            "hub.projects.v1",
        ],
        256,
    );
    sharding::routes::open(&storage, "hub.shards.v1", "hub.owners.v1", initial);
    sharding::registry::open(&storage, "hub.projects.v1", sharding::routes::root(), fresh);
    super::routes::open(&storage, fresh);
    let cell = canic::storage::cell(
        &storage,
        KEY,
        29,
        initial.map(|(p, _)| p.as_slice().to_vec()),
    );
    ROOT.set(Some(
        Principal::try_from_slice(cell.get()).expect("retained Root"),
    ));
}

canic::mount_lifecycle!(
    input = (Principal, Option<Principal>),
    max_bytes = 128,
    decode = principal,
    install = install,
    reopen = reopen
);

fn install(root: (Principal, Option<Principal>)) {
    open(Some(root));
}

fn reopen() {
    open(None);
}

/// Assign the authenticated caller's key in the installed hub.
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
    sharding::routes::assign_shard("user_shard", runtime::system::caller(), user)
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
fn lookup((): ()) -> Placement {
    sharding::routes::get_shard("user_shard", runtime::system::caller())
        .map_or(Placement::Unknown, |t| {
            Placement::Assigned(t.allocation, t.shard.to_text())
        })
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
    sharding::registry::assign_instance(key)
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
    sharding::registry::lookup_instance(key)
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
pub(super) fn user() -> Result<(), &'static str> {
    (runtime::system::caller() != Principal::anonymous())
        .then_some(())
        .ok_or("authenticated user")
}

fn principal(raw: &[u8]) -> Result<(Principal, Option<Principal>), &'static str> {
    if let Ok((root, operator)) =
        ic_wire::decode::<(Principal, Principal)>(raw, ic_wire::Encoding::Payload, sharding::LIMITS)
    {
        Ok((root, Some(operator)))
    } else {
        Principal::try_from_slice(raw)
            .map(|root| (root, None))
            .map_err(|_| "root principal")
    }
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
#[canic::update(guard = canic::guard::require_controller, limits = sharding::LIMITS)]
async fn pay(
    input: (sharding::payment::Demand, u128, u128),
) -> Result<sharding::payment::Payment, canic::sharding::payment::Refusal> {
    sharding::payment::acquire_placement(ROOT.get().unwrap(), input.0, input.1, input.2)
        .await
        .map_err(|error| match error {
            canic::sharding::payment::AcquireError::Refused(reason) => reason,
            other => panic!("funded placement call failed: {other:?}"),
        })
}

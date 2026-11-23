//! The app commits account records. Canic supplies each durable route.
use crate::actor::{ROOT, user};
use canic::{
    method::Contract,
    sharding::{self, Placement, routes},
};
use canic_profile_protocol::ProfileError;
use ic_platform_runtime::system;
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, MemoryRuntime, adapter::StableCell};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::BTreeSet};

pub(super) const KEY: &str = "hub.records.v1";
type Cell = StableCell<Vec<u8>, DefaultMemoryImpl>;

#[derive(Default, Serialize, Deserialize)]
struct Records {
    records: BTreeSet<Principal>,
}

thread_local! { static CELL: RefCell<Option<Cell>> = const { RefCell::new(None) }; }
const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 16384,
    allocation: 65536,
    nodes: 4096,
    depth: 16,
};

pub(super) fn open(store: &MemoryRuntime<DefaultMemoryImpl>, fresh: bool) {
    let cell = canic::storage::cell(
        store,
        KEY,
        LIMITS.bytes,
        fresh.then(|| encode(&Records::default())),
    );
    CELL.with_borrow_mut(|c| *c = Some(cell));
    read(|_| ());
}

fn encode(routes: &Records) -> Vec<u8> {
    ic_wire::encode(routes, ic_wire::Encoding::Payload, LIMITS).unwrap()
}
fn read<T>(f: impl FnOnce(&Records) -> T) -> T {
    CELL.with_borrow(|c| {
        f(&ic_wire::decode(
            c.as_ref().unwrap().get(),
            ic_wire::Encoding::Payload,
            LIMITS,
        )
        .unwrap())
    })
}
fn change<T>(f: impl FnOnce(&mut Records) -> T) -> T {
    CELL.with_borrow_mut(|c| {
        let cell = c.as_mut().unwrap();
        let mut routes = ic_wire::decode(cell.get(), ic_wire::Encoding::Payload, LIMITS).unwrap();
        let result = f(&mut routes);
        cell.set(encode(&routes));
        result
    })
}

fn parent() -> Result<(), &'static str> {
    (Some(system::caller()) == ROOT.get())
        .then_some(())
        .ok_or("installed parent required")
}

struct Insert;
impl Contract for Insert {
    const NAME: &'static str = "insert_account";
    const ID: u64 = ic_wire::method_id(Self::NAME);
    type Input = Principal;
    type Output = Principal;
}

// insert_account
// Commit this account once and acknowledge the original subject on replay.
// guard  : installed parent
// limits : default Wire limits
#[canic::update(contract = Insert, guard = parent)]
fn insert_account(subject: Principal) -> Principal {
    if !read(|r| r.records.contains(&subject)) {
        change(|r| r.records.insert(subject));
    }
    subject
}

// record_count
// Count records at the installed shard for the qualification witness.
// guard  : public
// limits : default Wire limits
#[canic::query]
fn record_count() -> u32 {
    read(|r| u32::try_from(r.records.len()).unwrap())
}

fn readiness() -> canic::readiness::State {
    read(|_| canic::readiness::State::Ready)
}
canic::mount_readiness_endpoint!(crate::routes::readiness);

thread_local! { static DISABLED: RefCell<Option<Principal>> = const { RefCell::new(None) }; }

// disable_registration
// Change the qualification hub's registration permission for one subject.
// guard  : controller
// limits : default Wire limits
#[canic::update(guard = canic::guard::require_controller)]
fn disable_registration(subject: Option<Principal>) {
    DISABLED.set(subject);
}

fn authorize(subject: Principal) -> Result<(), ProfileError> {
    (system::caller() == subject && DISABLED.with_borrow(|disabled| *disabled != Some(subject)))
        .then_some(())
        .ok_or(ProfileError::Forbidden)
}

// local_route
// Read the caller's durable assignment from the hub.
// guard  : identified caller
// limits : default Wire limits
#[canic::query(guard = user)]
fn local_route() -> Option<Principal> {
    let subject = system::caller();
    if !read(|r| r.records.contains(&subject)) {
        return None;
    }
    routes::get_shard("user_shard", subject).map(|target| target.shard)
}

// register_local
// Reserve a local route, grow on exhaustion and commit after the original shard acknowledges.
// guard  : identified caller, rechecked after each await
// limits : sharding Wire limits
#[canic::update(guard = user, limits = sharding::LIMITS)]
async fn register_local() -> Result<Placement, ProfileError> {
    let subject = system::caller();
    let placed = routes::assign_shard("user_shard", subject, || authorize(subject))
        .await
        .map_err(|error| match error {
            routes::ReservationError::Authorization(refusal) => refusal,
            routes::ReservationError::Placement(reason) => panic!("retry capacity: {reason}"),
        })?;
    if let Some((_, shard)) = placed.assigned_target().unwrap()
        && !read(|r| r.records.contains(&subject))
    {
        let ack = canic::method::call::<Insert>(shard, 10, &subject)
            .await
            .expect("retry original write");
        authorize(subject)?;
        assert_eq!(ack, subject);
        change(|r| r.records.insert(subject));
    }
    Ok(placed)
}

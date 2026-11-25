//! One account payload per subject in the installed evacuation witness.
use canic::{
    method::{self, Contract},
    sharding::{Placement, ShardTarget, routes},
};
use ic_platform_runtime::system;
use ic_platform_types::Principal;
use ic_storage::{DefaultMemoryImpl, MemoryRuntime, adapter::StableBTreeMap};
use std::cell::RefCell;

pub(super) const KEYS: [&str; 3] = ["hub.roster.v2", "hub.routes.v2", "hub.accounts.v1"];
type Accounts = StableBTreeMap<Vec<u8>, (u64, u8)>;
thread_local! { static ACCOUNTS: RefCell<Option<Accounts>> = const { RefCell::new(None) }; }

pub(super) fn open(
    storage: &MemoryRuntime<DefaultMemoryImpl>,
    root: Option<(Principal, Option<Principal>)>,
) {
    routes::open(storage, KEYS[0], KEYS[1], root);
    let lease = storage.open_memory(&KEYS[2].parse().unwrap()).unwrap();
    ACCOUNTS.set(Some(if root.is_some() {
        StableBTreeMap::create(lease)
    } else {
        StableBTreeMap::load(lease)
    }));
}

canic::mount_hub_routes!();

// Reserve the actual caller's stable route under this witness pool.
#[canic::update(limits = canic::sharding::LIMITS)]
async fn local_place() -> Placement {
    routes::assign_shard("users", system::caller(), || {
        Ok::<_, std::convert::Infallible>(())
    })
    .await
    .unwrap()
}

// Edit the actual caller's payload unless the app has fenced it for evacuation.
#[canic::update]
fn write_value(value: u64) -> Result<(), String> {
    let key = system::caller().as_slice().to_vec();
    ACCOUNTS.with_borrow_mut(|accounts| {
        let rows = accounts.as_mut().unwrap();
        if rows.get(&key).is_some_and(|(_, fenced)| fenced != 0) {
            return Err("account fenced".into());
        }
        rows.insert(key, (value, 0));
        Ok(())
    })
}

// Read the caller's application payload at this shard.
#[canic::query]
fn read_value() -> Option<u64> {
    ACCOUNTS.with_borrow(|accounts| {
        accounts
            .as_ref()
            .unwrap()
            .get(&system::caller().as_slice().to_vec())
            .map(|(value, _)| value)
    })
}

struct Fence;
impl Contract for Fence {
    const NAME: &'static str = "fence_account";
    const ID: u64 = method::wire::method_id(Self::NAME);
    type Input = Principal;
    type Output = u64;
}
struct CopyAccount;
impl Contract for CopyAccount {
    const NAME: &'static str = "copy_account";
    const ID: u64 = method::wire::method_id(Self::NAME);
    type Input = (Principal, u64);
    type Output = u64;
}

fn parent() -> Result<(), &'static str> {
    let parent = crate::DIRECTORY.with_borrow(|d| d.as_ref().unwrap().publisher());
    if parent == system::caller() {
        Ok(())
    } else {
        Err("parent required")
    }
}

// Freeze the source before returning the value to copy.
#[canic::update(contract = Fence, guard = parent)]
fn fence_account(subject: Principal) -> u64 {
    ACCOUNTS.with_borrow_mut(|accounts| {
        let rows = accounts.as_mut().unwrap();
        let key = subject.as_slice().to_vec();
        let (value, _) = rows.get(&key).unwrap();
        rows.insert(key, (value, 1));
        value
    })
}

// Retain the first copied payload for the account and acknowledge an exact retry.
#[canic::update(contract = CopyAccount, guard = parent)]
fn copy_account((subject, value): (Principal, u64)) -> u64 {
    ACCOUNTS.with_borrow_mut(|accounts| {
        let rows = accounts.as_mut().unwrap();
        let key = subject.as_slice().to_vec();
        if let Some((existing, _)) = rows.get(&key) {
            assert_eq!(existing, value);
        } else {
            rows.insert(key, (value, 0));
        }
        value
    })
}

// The owner retries the same move; a committed move only acknowledges its receipt.
#[canic::update(limits = canic::sharding::LIMITS)]
async fn move_account((source, destination): (ShardTarget, ShardTarget)) -> Result<(), String> {
    let subject = system::caller();
    if routes::get_shard("users", subject).as_ref() == Some(&destination) {
        return routes::confirm_move("users", subject, &source, &destination)
            .map_err(str::to_owned);
    }
    let placed = routes::prepare_move("users", subject, &source, || {
        Ok::<_, std::convert::Infallible>(())
    })
    .await
    .map_err(|e| match e {
        routes::ReservationError::Placement(reason) => reason,
        routes::ReservationError::Authorization(never) => match never {},
    })?;
    assert_eq!(
        placed.assigned_target()?.unwrap(),
        (&destination.allocation, destination.shard)
    );
    let value = method::call::<Fence>(source.shard, 10, &subject)
        .await
        .unwrap();
    let confirmed = method::call::<CopyAccount>(destination.shard, 10, &(subject, value))
        .await
        .unwrap();
    assert_eq!(confirmed, value);
    routes::confirm_move("users", subject, &source, &destination)?;
    Ok(())
}

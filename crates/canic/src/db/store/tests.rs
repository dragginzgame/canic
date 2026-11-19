use super::*;
use ic_db::{Entity, Error};
use ic_storage::{StorageConfig, VectorMemory};
use std::panic::{AssertUnwindSafe, catch_unwind};

///
/// Item
/// Identifies the admitted schema when reopening an existing store.
///

#[ic_db::entity(
    pk(field = "id"),
    fields(field(name = "id", value(item(prim = "Nat64"))))
)]
struct Item {}

fn schema() -> Schema {
    Schema::new(&[Item::SCHEMA]).unwrap()
}

#[test]
fn open_requires_existing_database_and_preserves_neighbor_and_incarnation() {
    const KEY: &str = "app.shard.items.v1";
    const HUB: &str = "app.shard.hub.v1";
    let backing = VectorMemory::default();
    let bootstrap = |fresh| {
        crate::storage::bootstrap_with_prefix(
            backing.clone(),
            fresh,
            "shard",
            "app.shard.",
            &[HUB, KEY],
            StorageConfig::default(),
        )
    };
    let host = bootstrap(true);
    let store = Store::new(KEY, schema);
    assert!(catch_unwind(AssertUnwindSafe(|| store.attach(&host, None))).is_err());
    drop(crate::storage::cell(&host, HUB, 29, Some(vec![3, 1])));
    let id = host.allocation_id(&StableKey::parse(KEY).unwrap()).unwrap();
    store.attach(&host, Some([9; 16]));
    assert!(catch_unwind(AssertUnwindSafe(|| store.attach(&host, Some([2; 16])))).is_err());
    drop(store);
    drop(host);
    let host = bootstrap(false);
    let store = Store::new(KEY, schema);
    store.attach(&host, None);
    assert_eq!(store.with(|db| db.incarnation()), [9; 16]);
    assert_eq!(
        host.allocation_id(&StableKey::parse(KEY).unwrap()),
        Some(id)
    );
    assert_eq!(crate::storage::cell(&host, HUB, 29, None).get(), &[3, 1]);
}

///
/// Other
/// Supplies a different schema to test admission on reopen.
///

#[ic_db::entity(
    pk(field = "id"),
    fields(field(name = "id", value(item(prim = "Nat32"))))
)]
struct Other {}

#[test]
fn supplier_created_rows_and_parent_binding_survive_strict_open() {
    use ic_db::Batch;
    const KEY: &str = "toko.user.shard.user.v1";
    const HUB: &str = "toko.user.shard.hub.v1";
    let db_key = StableKey::parse(KEY).unwrap();
    let hub_key = StableKey::parse(HUB).unwrap();
    let backing = VectorMemory::default();
    let bootstrap = |fresh| {
        crate::storage::bootstrap_with_prefix(
            backing.clone(),
            fresh,
            "user_shard",
            "toko.user.shard.",
            &[KEY, HUB],
            StorageConfig::default(),
        )
    };
    let host = bootstrap(true);
    let ids = [
        host.allocation_id(&db_key).unwrap(),
        host.allocation_id(&hub_key).unwrap(),
    ];
    {
        let mut db =
            Database::create(host.open_memory(&db_key).unwrap(), schema(), [7; 16]).unwrap();
        let mut batch = Batch::new();
        batch.insert(&Item { id: 42 }).unwrap();
        db.commit(batch).unwrap();
    }
    drop(crate::storage::cell(&host, HUB, 29, Some(vec![8, 1])));
    drop(host);
    let retained = backing.borrow().clone();

    let host = bootstrap(false);
    let incompatible = Store::new(KEY, || Schema::new(&[Other::SCHEMA]).unwrap());
    assert!(catch_unwind(AssertUnwindSafe(|| incompatible.attach(&host, None))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| incompatible.with(|_| ()))).is_err());
    let store = Store::new(KEY, schema);
    store.attach(&host, None);
    assert_eq!(
        store.with(|db| db.get::<Item>(42_u64).unwrap().unwrap().id),
        42
    );
    assert_eq!(store.with(|db| db.incarnation()), [7; 16]);
    assert_eq!(
        [
            host.allocation_id(&db_key).unwrap(),
            host.allocation_id(&hub_key).unwrap()
        ],
        ids
    );
    assert_eq!(crate::storage::cell(&host, HUB, 29, None).get(), &[8, 1]);
    assert_eq!(*backing.borrow(), retained);
}

#[test]
fn independent_handles_share_typed_rows_and_refuse_conflicting_access() {
    use ic_db::Batch;
    const KEY: &str = "app.db.v1";
    let store = Store::new(KEY, || Schema::new(&[Item::SCHEMA, Other::SCHEMA]).unwrap());
    assert!(catch_unwind(AssertUnwindSafe(|| store.session())).is_err());
    let host = crate::storage::bootstrap_with_prefix(
        VectorMemory::default(),
        true,
        "app",
        "app.",
        &[KEY],
        StorageConfig::default(),
    );
    store.attach(&host, Some([7; 16]));
    let outer = store.session();
    let helper = store.session();
    let read = helper.read().unwrap();
    assert!(read.get::<Item>(1).unwrap().is_none());
    assert!(matches!(
        outer.commit(Batch::new()),
        Err(Error::BorrowConflict)
    ));
    drop(read);
    let mut batch = Batch::new();
    batch.insert(&Item { id: 1 }).unwrap();
    batch.insert(&Other { id: 2 }).unwrap();
    outer.commit(batch).unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| store.with(|_| ()))).is_err());
    assert_eq!(helper.get::<Item>(1).unwrap().unwrap().id, 1);
    assert_eq!(helper.get::<Other>(2).unwrap().unwrap().id, 2);
    drop((outer, helper));
    assert!(catch_unwind(AssertUnwindSafe(|| store.with(|_| store.session()))).is_err());
    assert_eq!(store.session().get::<Item>(1).unwrap().unwrap().id, 1);
}

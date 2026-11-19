use super::{
    rows::{accept, confirm, reserve, ticket},
    *,
};
use ic_db::{Database, Entity, Schema};
use ic_storage::{StableKey, VectorMemory};

use std::panic::{AssertUnwindSafe, catch_unwind};

struct Users;
impl Resource for Users {
    type Id = u64;
    type Data = String;
    const METHOD: &'static str = "provision_user";
    const DOMAIN: &'static [u8] = b"toko.user.placement.v1";
    const LIMITS: ic_wire::Limits = crate::method::LIMITS;
}

///
/// Route
/// Retains the hub’s account reservation and assigned shard.
///

#[ic_db::entity(
    pk(field = "id"),
    index(fields = ["issuer", "subject"], unique),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "issuer", value(item(prim = "Principal"))),
        field(name = "subject", value(item(prim = "Principal"))),
        field(name = "shard", value(item(prim = "Principal"))),
        field(name = "name", value(item(prim = "Text", max_bytes = 2048))),
        field(name = "status", value(item(prim = "Bool"))),
    )
)]
struct Route {}

///
/// User
/// Stores the provisioned account and name on the shard.
///

#[ic_db::entity(
    pk(field = "id"),
    index(fields = ["issuer", "subject"], unique),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "issuer", value(item(prim = "Principal"))),
        field(name = "subject", value(item(prim = "Principal"))),
        field(name = "name", value(item(prim = "Text", max_bytes = 128))),
    )
)]
struct User {}

struct Hub;
struct Shard;

fn schema() -> Schema {
    Schema::new(&[Route::SCHEMA, User::SCHEMA]).unwrap()
}

crate::declare_db_store!(mod store, key = "test.accounts.v1", schema = super::schema);

#[test]
fn declared_rows_share_the_attached_store() {
    let host =
        crate::storage::bootstrap(Default::default(), true, "test", &["test.accounts.v1"], 128);
    store::attach(&host, Some([7; 16]));
    assert_eq!(Hub::with_store(|db| db.incarnation()), [7; 16]);
    assert_eq!(Shard::with_store(|db| db.incarnation()), [7; 16]);
}
impl Record for Hub {
    type Row = Route;
    type Resource = Users;

    fn with_store<T>(f: impl FnOnce(&mut Database) -> T) -> T {
        store::with(f)
    }

    fn identity(row: &Route) -> (Account, u64) {
        (
            Account {
                issuer: row.issuer,
                subject: row.subject,
            },
            row.id,
        )
    }
}

impl Record for Shard {
    type Row = User;
    type Resource = Users;

    fn with_store<T>(f: impl FnOnce(&mut Database) -> T) -> T {
        store::with(f)
    }

    fn identity(row: &User) -> (Account, u64) {
        (
            Account {
                issuer: row.issuer,
                subject: row.subject,
            },
            row.id,
        )
    }
}

impl HubRoute for Hub {
    const SCOPE: &'static str = "write";
    type Initial = String;

    fn normalize(name: String) -> String {
        name
    }

    fn pending(binding: Binding<u64>, name: String) -> Route {
        Route {
            id: binding.id,
            issuer: binding.account.issuer,
            subject: binding.account.subject,
            shard: binding.shard,
            name,
            status: false,
        }
    }

    fn retained(row: &Route) -> (Principal, String) {
        (row.shard, row.name.clone())
    }

    fn ready(row: &Route) -> bool {
        row.status
    }

    fn mark_ready(row: &mut Route) {
        row.status = true;
    }
}

impl ShardRecord for Shard {
    fn create(request: Ticket<Self>) -> User {
        assert!(!request.data.is_empty());
        let binding = request.binding;
        User {
            id: binding.id,
            issuer: binding.account.issuer,
            subject: binding.account.subject,
            name: request.data,
        }
    }
}

#[test]
fn racing_id_and_lost_receipt_reopen_the_original_reservation() {
    let backing = VectorMemory::default();
    let open = |fresh: bool| {
        let host =
            crate::storage::bootstrap(backing.clone(), fresh, "test", &["test.accounts.v1"], 128);
        let memory = host
            .open_memory(&StableKey::parse("test.accounts.v1").unwrap())
            .unwrap();
        let schema = Schema::new(&[Route::SCHEMA, User::SCHEMA]).unwrap();
        let db = if fresh {
            Database::create(memory, schema, [7; 16])
        } else {
            Database::open(memory, schema)
        };
        (host, db.unwrap())
    };
    let account = Account {
        issuer: Principal::from_slice(&[3, 1]),
        subject: Principal::from_slice(&[4, 2]),
    };
    let binding = Binding {
        id: 19,
        account,
        shard: Principal::from_slice(&[8, 1]),
    };
    let competing = Binding { id: 20, ..binding };
    {
        let (_host, mut database) = open(true);
        let db = &mut database;
        let winner = reserve::<Hub>(db, &binding, "Ada".into());
        let raced = reserve::<Hub>(db, &competing, "replace me".into());
        assert_eq!(ticket::<Hub>(&raced).binding, binding);
        assert_eq!(raced.name, winner.name);
        assert!(accept::<Shard>(db, ticket::<Hub>(&winner)));
        assert!(!Hub::ready(&winner));
    }
    {
        let (_host, mut database) = open(false);
        let db = &mut database;
        let winner = reserve::<Hub>(db, &competing, "late".into());
        assert_eq!(ticket::<Hub>(&winner).binding, binding);
        assert!(!Hub::ready(&winner));
        assert!(!accept::<Shard>(
            db,
            Request {
                binding: competing.clone(),
                data: "late".into()
            }
        ));
        assert!(catch_unwind(AssertUnwindSafe(|| confirm::<Hub>(db, &competing))).is_err());
        assert!(accept::<Shard>(db, ticket::<Hub>(&winner)));
        confirm::<Hub>(db, &binding);
        confirm::<Hub>(db, &binding);
        let mut invalid_replay = ticket::<Hub>(&winner);
        invalid_replay.data.clear();
        assert!(catch_unwind(AssertUnwindSafe(|| accept::<Shard>(db, invalid_replay))).is_err());
        assert!(db.get::<Route>(19).unwrap().unwrap().status);
        assert_eq!(db.get::<User>(19).unwrap().unwrap().name, "Ada");
        assert!(db.get::<User>(20).unwrap().is_none());
    }
}

#[test]
fn confirmation_checks_encoded_size_before_updating_a_retained_route() {
    let host = crate::storage::bootstrap(
        VectorMemory::default(),
        true,
        "test",
        &["test.rows.v1"],
        128,
    );
    let memory = host
        .open_memory(&StableKey::parse("test.rows.v1").unwrap())
        .unwrap();
    let mut db = Database::create(memory, Schema::new(&[Route::SCHEMA]).unwrap(), [7; 16]).unwrap();
    for (id, size) in [(1, 1024), (2, 1025)] {
        let binding = Binding {
            id,
            account: Account {
                issuer: Principal::from_slice(&[1]),
                subject: Principal::from_slice(&[u8::try_from(id).unwrap()]),
            },
            shard: Principal::from_slice(&[3]),
        };
        let mut row = Hub::pending(binding.clone(), "x".repeat(1024));
        let overhead = row.encode().unwrap().len() - row.name.len();
        row.name.truncate(size - overhead);
        assert_eq!(row.encode().unwrap().len(), size);
        let mut batch = ic_db::Batch::new();
        batch.insert(&row).unwrap();
        db.commit(batch).unwrap();

        let result = catch_unwind(AssertUnwindSafe(|| confirm::<Hub>(&mut db, &binding)));
        assert_eq!(result.is_ok(), size == 1024);
        assert_eq!(db.get::<Route>(id).unwrap().unwrap().status, size == 1024);
    }
}

#[test]
fn a_nonunique_account_index_cannot_authorize_provisioning() {
    ///
    /// Nonunique
    /// Exercises refusal of an account lookup backed by a nonunique index.
    ///

    #[ic_db::entity(
        pk(field = "id"),
        index(fields = ["issuer", "subject"]),
        fields(
            field(name = "id", value(item(prim = "Nat64"))),
            field(name = "issuer", value(item(prim = "Principal"))),
            field(name = "subject", value(item(prim = "Principal"))),
        )
    )]
    struct Nonunique {}

    let host = crate::storage::bootstrap(
        VectorMemory::default(),
        true,
        "test",
        &["test.accounts.v1"],
        128,
    );
    let memory = host
        .open_memory(&StableKey::parse("test.accounts.v1").unwrap())
        .unwrap();
    let db = Database::create(memory, Schema::new(&[Nonunique::SCHEMA]).unwrap(), [7; 16]).unwrap();
    let account = Account {
        issuer: Principal::from_slice(&[1]),
        subject: Principal::from_slice(&[2]),
    };
    assert!(
        catch_unwind(AssertUnwindSafe(|| super::rows::find::<Nonunique>(
            &db, account
        )))
        .is_err()
    );
}

#[test]
fn provisioning_requires_both_an_identified_issuer_and_subject() {
    let account = Account {
        issuer: Principal::from_slice(&[1]),
        subject: Principal::from_slice(&[2]),
    };
    assert!(account.valid());
    assert!(
        !Account {
            issuer: Principal::anonymous(),
            ..account
        }
        .valid()
    );
    assert!(
        !Account {
            subject: Principal::anonymous(),
            ..account
        }
        .valid()
    );
}

#[test]
fn an_identical_principal_cannot_confirm_a_replacement_generation() {
    use crate::sharding::{Allocation, Placement};
    let shard = Principal::from_slice(&[42, 1]);
    let allocation = Allocation {
        slot: 0,
        generation: [8; 32],
    };
    let next = Allocation {
        generation: [9; 32],
        ..allocation
    };
    assert!(matches_target(
        &Placement::Assigned(allocation.clone(), shard.to_text()),
        &allocation,
        shard
    ));
    for stale in [
        Placement::Assigned(next, shard.to_text()),
        Placement::Unavailable(allocation.clone(), Some(shard.to_text())),
        Placement::Unknown,
    ] {
        assert!(!matches_target(&stale, &allocation, shard));
    }
}

#[test]
fn published_account_key_and_flat_provisioning_frame_remain_compatible() {
    #[derive(serde::Serialize)]
    struct OldBinding {
        id: u64,
        issuer: Principal,
        subject: Principal,
        shard: Principal,
    }
    #[derive(serde::Serialize)]
    struct OldRequest {
        binding: OldBinding,
        display_name: String,
    }
    let issuer = Principal::from_slice(&[3, 1]);
    let subject = Principal::from_slice(&[4, 2]);
    let shard = Principal::from_slice(&[7, 1]);
    let account = Account { issuer, subject };
    assert_eq!(
        placement_key::<Users>(account),
        [
            175, 196, 237, 214, 246, 17, 157, 120, 131, 131, 19, 88, 217, 49, 210, 138, 61, 54, 38,
            156, 70, 154, 103, 109, 198, 130, 202, 238, 101, 79, 128, 48
        ]
    );
    let old = OldRequest {
        binding: OldBinding {
            id: 9,
            issuer,
            subject,
            shard,
        },
        display_name: "Ada".into(),
    };
    let frame = ic_wire::Encoding::Frame {
        method: ic_wire::method_id("provision_user"),
        direction: ic_wire::Direction::Request,
    };
    let bytes = ic_wire::encode(&old, frame, crate::method::LIMITS).unwrap();
    let decoded = Provision::<Users>::decode_request(&bytes).unwrap();
    assert_eq!(
        decoded.binding,
        Binding {
            id: 9,
            account,
            shard
        }
    );
    assert_eq!(decoded.data, "Ada");
    assert_eq!(Provision::<Users>::encode_request(&decoded).unwrap(), bytes);
}

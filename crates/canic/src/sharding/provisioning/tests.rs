use super::*;
use ic_db::{Database, DatabaseSession, Entity, Schema};
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

// These are application operations on real IC DB rows, including a mutable name.
fn reserve(db: &DatabaseSession, binding: &Binding<u64>, name: String) -> Route {
    account_row::<Route>(db, binding.account).unwrap_or_else(|| {
        let row = pending(binding, name);
        db.insert(&row).unwrap();

        row
    })
}

fn pending(binding: &Binding<u64>, name: String) -> Route {
    Route {
        id: binding.id,
        issuer: binding.account.issuer,
        subject: binding.account.subject,
        shard: binding.shard,
        name,
        status: false,
    }
}

fn ticket(row: &Route) -> Request<u64, String> {
    Request {
        binding: Binding {
            id: row.id,
            account: Account {
                issuer: row.issuer,
                subject: row.subject,
            },
            shard: row.shard,
        },
        data: row.name.clone(),
    }
}

fn confirm(db: &DatabaseSession, receipt: &Binding<u64>) {
    let mut row = account_row::<Route>(db, receipt.account).unwrap();
    receipt.require_same(&ticket(&row).binding);
    if !row.status {
        row.status = true;
        db.replace(&row).unwrap();
    }
}

fn accept(db: &DatabaseSession, request: Request<u64, String>) -> bool {
    assert!(!request.data.is_empty());
    let binding = request.binding;
    if let Some(user) = account_row::<User>(db, binding.account) {
        return user.id == binding.id;
    }
    db.insert(&User {
        id: binding.id,
        issuer: binding.account.issuer,
        subject: binding.account.subject,
        name: request.data,
    })
    .unwrap();

    true
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
        (host, DatabaseSession::new(db.unwrap()))
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
        let (_host, database) = open(true);
        let db = &database;
        let winner = reserve(db, &binding, "Ada".into());
        let raced = reserve(db, &competing, "replace me".into());
        assert_eq!(ticket(&raced).binding, binding);
        assert_eq!(raced.name, winner.name);
        assert!(accept(db, ticket(&winner)));
        assert!(!winner.status);
        let mut edited = db.get::<User>(19).unwrap().unwrap();
        edited.name = "Ada edited".into();
        db.replace(&edited).unwrap();
    }
    {
        let (_host, database) = open(false);
        let db = &database;
        let winner = reserve(db, &competing, "late".into());
        assert_eq!(ticket(&winner).binding, binding);
        assert!(!winner.status);
        assert!(!accept(
            db,
            Request {
                binding: competing.clone(),
                data: "late".into()
            }
        ));
        assert!(catch_unwind(AssertUnwindSafe(|| confirm(db, &competing))).is_err());
        assert!(accept(db, ticket(&winner)));
        confirm(db, &binding);
        confirm(db, &binding);
        let mut invalid_replay = ticket(&winner);
        invalid_replay.data.clear();
        assert!(catch_unwind(AssertUnwindSafe(|| accept(db, invalid_replay))).is_err());
        assert!(db.get::<Route>(19).unwrap().unwrap().status);
        assert_eq!(db.get::<User>(19).unwrap().unwrap().name, "Ada edited");
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
    let db = DatabaseSession::new(
        Database::create(memory, Schema::new(&[Route::SCHEMA]).unwrap(), [7; 16]).unwrap(),
    );
    for (id, size) in [(1, 1024), (2, 1025)] {
        let binding = Binding {
            id,
            account: Account {
                issuer: Principal::from_slice(&[1]),
                subject: Principal::from_slice(&[u8::try_from(id).unwrap()]),
            },
            shard: Principal::from_slice(&[3]),
        };
        let mut row = pending(&binding, "x".repeat(1024));
        let overhead = row.encode().unwrap().len() - row.name.len();
        row.name.truncate(size - overhead);
        assert_eq!(row.encode().unwrap().len(), size);
        let mut batch = ic_db::Batch::new();
        batch.insert(&row).unwrap();
        db.commit(batch).unwrap();

        let result = catch_unwind(AssertUnwindSafe(|| confirm(&db, &binding)));
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
        catch_unwind(AssertUnwindSafe(|| account_row::<Nonunique>(
            &DatabaseSession::new(db),
            account
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

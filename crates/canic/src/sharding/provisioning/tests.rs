use super::{
    rows::{accept, confirm, reserve, ticket},
    *,
};
use ic_db::{Database, Entity, Schema};
use ic_storage::{StableKey, VectorMemory};

struct Users;
impl Resource for Users {
    type Id = u64;
    type Data = String;
    const METHOD: &'static str = "provision_user";
    const DOMAIN: &'static [u8] = b"toko.user.placement.v1";
    const LIMITS: ic_wire::Limits = crate::method::LIMITS;
}

#[ic_db::entity(
    pk(field = "id"),
    index(fields = ["issuer", "subject"], unique),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "issuer", value(item(prim = "Principal"))),
        field(name = "subject", value(item(prim = "Principal"))),
        field(name = "shard", value(item(prim = "Principal"))),
        field(name = "name", value(item(prim = "Text", max_bytes = 128))),
        field(name = "status", value(item(prim = "Bool"))),
    )
)]
struct Route {}

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

use std::{
    convert::identity,
    panic::{AssertUnwindSafe, catch_unwind},
};

const PENDING: bool = false;
const READY: bool = true;

// The actor paths deliberately have no native implementation.
crate::declare_provisioning_row! {
    hub self::Route, resource = Users, store = crate::database::store::with,
    id = (identity, identity, crate::registration::new_id), scope = "write",
    payload = name: String, validate = identity, export = String::clone, status = (PENDING, READY)
}

fn valid_name(name: String) -> String {
    assert!(!name.is_empty());
    name
}

crate::declare_provisioning_row! {
    shard self::User, resource = Users, store = crate::database::store::with,
    id = (identity, identity), payload = name, validate = valid_name
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
        let winner = reserve::<Route>(db, &binding, "Ada".into());
        let raced = reserve::<Route>(db, &competing, "replace me".into());
        assert_eq!(ticket(&raced).binding, binding);
        assert_eq!(raced.name, winner.name);
        assert!(accept::<User>(db, ticket(&winner)));
        assert!(!winner.ready());
    }
    {
        let (_host, mut database) = open(false);
        let db = &mut database;
        let winner = reserve::<Route>(db, &competing, "late".into());
        assert_eq!(ticket(&winner).binding, binding);
        assert!(!winner.ready());
        assert!(!accept::<User>(
            db,
            Request {
                binding: competing.clone(),
                data: "late".into()
            }
        ));
        assert!(catch_unwind(AssertUnwindSafe(|| confirm::<Route>(db, &competing))).is_err());
        assert!(accept::<User>(db, ticket(&winner)));
        confirm::<Route>(db, &binding);
        confirm::<Route>(db, &binding);
        let mut invalid_replay = ticket(&winner);
        invalid_replay.data.clear();
        assert!(catch_unwind(AssertUnwindSafe(|| accept::<User>(db, invalid_replay))).is_err());
        assert!(db.get::<Route>(19).unwrap().unwrap().ready());
        assert_eq!(db.get::<User>(19).unwrap().unwrap().name, "Ada");
        assert!(db.get::<User>(20).unwrap().is_none());
    }
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

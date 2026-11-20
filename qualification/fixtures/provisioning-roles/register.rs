//! Registration items for Toko's hub outbound endpoint module.
use crate::{Status, database};
use canic::{
    db,
    sharding::provisioning::{self, Account, Binding, Progress, Request},
};
use ic_db::Ulid;
use toko::user::{initial_display_name, provisioning::Users};
use toko_schema::entity::user_hub::UserRoute;

// register
// Reserve the signed-in account and confirm insertion on its assigned shard.
// scope  : write
// limits : default
#[canic::update(scope = WRITE)]
async fn register(name: String) -> Progress {
    let name = initial_display_name(name);
    let account = Account::from(&auth);
    let lookup = || provisioning::account_row::<UserRoute>(&db!(), account);
    let existing = lookup();
    if existing
        .as_ref()
        .is_some_and(|row| row.status == Status::Ready)
    {
        return Progress::Ready;
    }
    let assignment =
        match provisioning::place::<Users>(&auth, existing.as_ref().map(|row| row.shard))
            .await
        {
            Ok(assignment) => assignment,
            Err(progress) => return progress,
        };
    // Another registration may have won during placement. Only the winner allocates an ID.
    let row = existing.or_else(lookup).unwrap_or_else(|| {
        let row = UserRoute {
            id: Ulid::generate_prand(),
            issuer: account.issuer,
            subject: account.subject,
            shard: assignment.shard(),
            status: Status::Pending,
            display_name: name,
        };
        db!().insert(&row).expect("reserve account");

        row
    });
    let request = Request {
        binding: binding(&row),
        data: row.display_name.0,
    };
    let receipt = match assignment.send(&request).await {
        Ok(receipt) => receipt,
        Err(progress) => return progress,
    };
    let mut retained = lookup().expect("missing reservation");
    receipt.require_same(&binding(&retained));
    if retained.status != Status::Ready {
        retained.status = Status::Ready;
        db!().replace(&retained).expect("confirm reservation");
    }

    Progress::Ready
}

const WRITE: &str = "write";

fn binding(row: &UserRoute) -> Binding<[u8; 16]> {
    Binding {
        id: row.id.to_bytes(),
        account: Account {
            issuer: row.issuer,
            subject: row.subject,
        },
        shard: row.shard,
    }
}

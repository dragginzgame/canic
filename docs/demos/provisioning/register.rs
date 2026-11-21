//! Toko hub migration for typed provisioning refusals.

use crate::{Status, database, ops};
use canic::{
    db,
    sharding::provisioning::{self, AuthSubject, Progress, Request, SendError},
};
use ic_db::Ulid;
use toko::user::{
    account::AccountError,
    display_name,
    provisioning::{ProvisionRefusal, Users},
};
use toko_schema::canister::user_hub::UserRoute;

// register
// Reserve the signed-in account and confirm insertion on its assigned shard.
// scope  : write
// limits : default
#[canic::update(scope = WRITE)]
async fn register(name: String) -> Result<Progress, AccountError> {
    let name = display_name(&name)?;
    let account = AuthSubject::from(&auth);
    let existing = ops::reservation(account)?;
    if existing
        .as_ref()
        .is_some_and(|row| row.status == Status::Ready)
    {
        return Ok(Progress::Ready);
    }
    let assignment = match provisioning::place::<Users>(
        &auth,
        canic::shard_pool!("user_shard"),
        existing.as_ref().map(|row| row.shard),
    )
    .await
    {
        Ok(assignment) => assignment,
        Err(progress) => return ops::registration_progress(account, progress),
    };
    // Another registration may have won during placement. Only the winner allocates an ID.
    let row = if let Some(row) = ops::reservation(account)? {
        if row.status == Status::Ready {
            return Ok(Progress::Ready);
        }

        row
    } else {
        let row = UserRoute {
            id: Ulid::generate_prand(),
            issuer: account.issuer,
            subject: account.subject,
            shard: assignment.shard(),
            status: Status::Pending,
            display_name: name,
        };
        db!().insert(&row)?;

        row
    };
    let request = Request {
        binding: ops::binding(&row),
        data: row.display_name.0,
    };
    let receipt = match assignment.send(&request).await {
        Ok(receipt) => receipt,
        Err(SendError::Progress(progress)) => return ops::registration_progress(account, progress),
        Err(SendError::Refused(reason)) => {
            return Err(match reason {
                ProvisionRefusal::InvalidDisplayName => AccountError::InvalidInput,
                ProvisionRefusal::BindingConflict => AccountError::Rejected,
                ProvisionRefusal::Limit => AccountError::Limit,
                ProvisionRefusal::Database => AccountError::Database,
            });
        }
    };
    let mut retained = ops::reservation(account)?.ok_or(AccountError::NotFound)?;
    if receipt != ops::binding(&retained) {
        return Err(AccountError::OutcomeUnknown);
    }
    if retained.status != Status::Ready {
        retained.status = Status::Ready;
        db!().replace(&retained)?;
    }

    Ok(Progress::Ready)
}

const WRITE: &str = "write";

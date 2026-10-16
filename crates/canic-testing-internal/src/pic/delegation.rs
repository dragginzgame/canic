use candid::Principal;

use canic::Error;
use canic::dto::auth::DelegatedRoleGrant;
use canic::ids::CanisterRole;

use ic_testkit::pic::{CandidCallExt, PocketIc};

/// Create one user shard through the reference `user_hub` path.
///
/// # Panics
///
/// Panics if the `create_account` transport or application call fails.
#[must_use]
pub fn create_user_shard(
    pic: &PocketIc,
    user_hub_pid: Principal,
    user_pid: Principal,
) -> Principal {
    let created: Result<Principal, Error> = pic
        .update_candid(
            user_hub_pid,
            "create_account",
            (user_pid,),
            canic::testing::CANDID_REPLY_LIMITS,
        )
        .expect("update Candid harness call");
    created.expect("create_account application failed")
}

#[must_use]
pub const fn role_grant(target: CanisterRole, scopes: Vec<String>) -> DelegatedRoleGrant {
    DelegatedRoleGrant { target, scopes }
}

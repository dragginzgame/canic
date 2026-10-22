use ic_platform::principal::ExternalPrincipal as Principal;

use canic::{Error, dto::auth::DelegatedRoleGrant, ids::CanisterRole};

use ic_platform::pocket_ic::PocketIc;

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
    let created: Result<Principal, Error> =
        (|| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
            let request_bytes = ic_platform::candid::contract::encode_args((user_pid,))?;
            let reply = ic_testkit::pic::try_pocket_ic_operation(|| {
                (pic).update_call(
                    user_hub_pid,
                    ic_platform::principal::ExternalPrincipal::anonymous(),
                    "create_account",
                    request_bytes,
                )
            })
            .map_err(ic_testkit::pic::PocketIcCallFailure::Operation)?
            .map_err(ic_testkit::pic::PocketIcCallFailure::Rejected)?;
            ic_platform::candid::decode_args(&reply, canic::testing::CANDID_REPLY_LIMITS)
                .map_err(Into::into)
        })()
        .map(|(reply,)| reply)
        .expect("update Candid harness call");
    created.expect("create_account application failed")
}

#[must_use]
pub const fn role_grant(target: CanisterRole, scopes: Vec<String>) -> DelegatedRoleGrant {
    DelegatedRoleGrant { target, scopes }
}

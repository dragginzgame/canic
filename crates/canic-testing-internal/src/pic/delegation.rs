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
        ic_testkit::pic::update_wire::<_, Result<ic_platform::Principal, Error>>(
            pic,
            user_hub_pid,
            ic_platform::principal::ExternalPrincipal::anonymous(),
            crate::pic::application_probe::CREATE_ACCOUNT,
            &ic_platform::Principal::from(user_pid),
        )
        .map(|reply| reply.map(Into::into))
        .expect("update Candid harness call");
    created.expect("create_account application failed")
}

#[must_use]
pub const fn role_grant(target: CanisterRole, scopes: Vec<String>) -> DelegatedRoleGrant {
    DelegatedRoleGrant { target, scopes }
}

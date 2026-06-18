#![expect(clippy::unused_async)]

#[cfg(canic_test_delegation_material)]
use canic::{
    Error,
    api::auth::AuthApi,
    cdk::types::Principal,
    dto::auth::{DelegatedRoleGrant, DelegationAudience},
    ids::{CanisterRole, cap},
    prelude::*,
};

canic::start!();

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

#[cfg(canic_test_delegation_material)]
#[canic_update(requires(caller::is_controller()))]
async fn root_test_upsert_delegation_issuer(issuer_pid: Principal) -> Result<(), Error> {
    AuthApi::test_upsert_root_issuer_policy(
        issuer_pid,
        vec![DelegationAudience::Project("test".to_string())],
        vec![DelegatedRoleGrant {
            target: CanisterRole::new("test"),
            scopes: vec![cap::VERIFY.to_string()],
        }],
        600_000_000_000,
        8_000,
    )
}

canic::finish!();

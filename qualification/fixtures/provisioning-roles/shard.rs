//! Private Toko accounts, distinct from Auth credentials and transport callers.
#[cfg(target_arch = "wasm32")]
mod endpoints;
mod ops;
mod schema;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

pub use ops::user::{Error, Identity, MAX_DISPLAY_NAME_BYTES, Profile, get_me, set_display_name};

const ARCHIVE: &str = "account_archive";

canic::mount_provisioning! {
    canic::sharding::provisioning::ProvisioningRole::Shard, schema::User => toko::user::provisioning::Users,
    role_name = ARCHIVE, authority = "user_shard", payload = display_name, prefix = "toko.user.shard.",
    database = "toko.user.shard.user.v1",
}

/// Report fixture metadata alongside the original retained database allocation.
#[cfg(target_arch = "wasm32")]
#[canic::query]
fn role_metadata() -> (String, String) {
    (
        APPLICATION_ROLE.unwrap_or_default().to_owned(),
        database::DATABASE_KEY.to_owned(),
    )
}

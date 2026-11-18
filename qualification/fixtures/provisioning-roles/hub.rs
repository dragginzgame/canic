//! Application-owned identity reservations and private account routing.
#[cfg(target_arch = "wasm32")]
mod endpoints;
mod ops;
mod schema;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;

pub use ops::get_my_route;
pub use toko::user::{Error, Identity, Route, Status};

use canic::sharding::provisioning::ProvisioningRole::Hub as Directory;

canic::mount_provisioning! {
    Directory, schema::UserRoute => toko::user::provisioning::Users,
    database = "toko.user.hub.principal.v1", config = "toko.user.hub.config.v1",
    authority = "toko", pages = 65536, payload = display_name, status = Status,
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

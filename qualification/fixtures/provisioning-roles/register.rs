//! Hub registration under the same scope checked after every suspension.
use crate::Hub;
use canic::{
    auth::protocol::SessionView,
    sharding::provisioning::{HubRoute, Progress, new_ulid, provision},
};
use toko::user::signup::LIMITS;

/// Reserve the authenticated account and confirm its original shard.
#[canic::update(
    scope = Hub::SCOPE,
    limits = LIMITS,
)]
async fn register(session: SessionView, name: String) -> Progress {
    provision::<Hub>(&session, name, new_ulid).await
}

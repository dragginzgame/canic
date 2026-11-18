//! Shard insertion under the authenticated parent.
use crate::Shard;
use canic::sharding::provisioning::{Binding, Provision, Ticket, receive};
use toko::user::provisioning::Users;

/// Insert the private row or acknowledge the original insertion.
#[canic::update(
    contract = Provision<Users>,
    guard = canic::topology::require_parent,
)]
fn provision_user(request: Ticket<Shard>) -> Binding<[u8; 16]> {
    receive::<Shard>(request)
}

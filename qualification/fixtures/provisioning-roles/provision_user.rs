//! Toko shard migration for typed provisioning refusals.

use crate::ops::user;
use canic::sharding::provisioning::{Binding, Provision, Request};
use ic_db::Ulid;
use toko::user::provisioning::{ProvisionRefusal, Users};

// provision_user
// Insert the private account row or acknowledge its existing binding without changing the name.
// guard  : canic::topology::require_parent
// limits : Provision<Users>
#[canic::update(
    contract = Provision<Users>,
    guard = canic::topology::require_parent,
)]
fn provision_user(request: Request<Ulid, String>) -> Result<Binding<Ulid>, ProvisionRefusal> {
    let binding = request.binding;
    binding
        .check_target()
        .map_err(|_| ProvisionRefusal::BindingConflict)?;
    user::provision(&binding, &request.data)?;

    Ok(binding)
}

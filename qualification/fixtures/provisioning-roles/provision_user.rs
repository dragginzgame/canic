//! Provisioning items for Toko's shard endpoint module.
use crate::{User, database};
use canic::{
    db,
    sharding::provisioning::{self, Binding, Provision, Request},
};
use ic_db::Ulid;
use toko::user::{initial_display_name, provisioning::Users};

// provision_user
// Insert the private account row or confirm an exact replay.
// guard  : canic::topology::require_parent
// limits : Provision<Users>
#[canic::update(
    contract = Provision<Users>,
    guard = canic::topology::require_parent,
)]
fn provision_user(request: Request<[u8; 16], String>) -> Binding<[u8; 16]> {
    let binding = request.binding;
    binding.require_target();
    let display_name = initial_display_name(request.data);
    let account = binding.account;
    if let Some(user) = provisioning::account_row::<User>(&db!(), account) {
        if user.id.to_bytes() != binding.id {
            canic::method::fail("BindingConflict");
        }
    } else {
        db!()
            .insert(&User {
                id: Ulid::from_bytes(binding.id),
                issuer: account.issuer,
                subject: account.subject,
                display_name,
            })
            .expect("insert user");
    }

    binding
}

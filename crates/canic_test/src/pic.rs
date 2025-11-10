use canic::{
    Error, memory::topology::SubnetIdentity, ops::CanisterInitPayload, types::CanisterType,
};
use derive_more::{Deref, DerefMut};
use pocket_ic::PocketIc;

///
/// Pic
///

#[derive(Deref, DerefMut)]
pub struct Pic<'a>(&'a PocketIc);

// install_args
// these vary depending on the canister type
pub fn install_args(ty: CanisterType) -> Result<Vec<u8>, Error> {
    let args = if ty.is_root() {
        candid::encode_one(SubnetIdentity::Test)
    } else {
        let payload = CanisterInitPayload::empty();
        candid::encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None))
    }?;

    Ok(args)
}

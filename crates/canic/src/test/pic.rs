use crate::{
    Error, memory::topology::SubnetIdentity, ops::CanisterInitPayload, types::CanisterType,
};

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

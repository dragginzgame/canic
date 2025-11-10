use crate::{
    Error,
    memory::topology::SubnetIdentity,
    ops::CanisterInitPayload,
    types::{CanisterType, Principal, TC},
};
use pocket_ic::PocketIc;

// create_canister
pub fn create_canister(pic: &PocketIc, ty: CanisterType, wasm: &[u8]) -> Result<Principal, Error> {
    // create canister and add cycles
    let canister_id = pic.create_canister();
    pic.add_cycles(canister_id, 5 * TC);

    // encode the init args for install
    let init = if ty.is_root() {
        candid::encode_one(SubnetIdentity::Test).unwrap()
    } else {
        let payload = CanisterInitPayload::empty();
        candid::encode_args::<(CanisterInitPayload, Option<Vec<u8>>)>((payload, None)).unwrap()
    };

    // install
    pic.install_canister(canister_id, wasm.to_vec(), init, None);

    Ok(canister_id)
}
